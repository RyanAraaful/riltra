// SPDX-License-Identifier: Apache-2.0
//! Multi-repo registry: JSON file with versioned schema, sorted entries,
//! idempotent re-registration of the same root. Corrupt files are reported,
//! never silently dropped.

use std::path::Path;

use crate::json::escape;
use crate::paths::validate_name;

#[derive(Debug, Clone, PartialEq)]
pub struct RepoEntry {
    pub name: String,
    pub root: String,
    pub added_unix: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Registry {
    pub version: u32,
    pub repos: Vec<RepoEntry>,
}

/// Register `root` under `name`. `Ok(true)` = new entry, `Ok(false)` =
/// idempotent re-add of the same root. A name pointing at a different root,
/// or an invalid name, is an error.
pub fn add(reg: &mut Registry, name: &str, root: &str, now_unix: u64) -> Result<bool, String> {
    if !validate_name(name) {
        return Err(format!("invalid repository name: '{name}'"));
    }
    if let Some(existing) = reg.repos.iter().find(|e| e.name == name) {
        if existing.root == root {
            return Ok(false);
        }
        return Err(format!(
            "name '{name}' is already registered for another root"
        ));
    }
    reg.repos.push(RepoEntry {
        name: name.to_string(),
        root: root.to_string(),
        added_unix: now_unix,
    });
    reg.repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(true)
}

pub fn find<'a>(reg: &'a Registry, name: &str) -> Option<&'a RepoEntry> {
    reg.repos.iter().find(|e| e.name == name)
}

pub fn entry_json(e: &RepoEntry) -> String {
    format!(
        "{{\"name\":\"{}\",\"root\":\"{}\",\"added_unix\":{}}}",
        escape(&e.name),
        escape(&e.root),
        e.added_unix
    )
}

/// `{"repos":[...]}` sorted by name (deterministic regardless of insertion order).
pub fn list_json(reg: &Registry) -> String {
    let mut entries: Vec<&RepoEntry> = reg.repos.iter().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let inner: Vec<String> = entries.iter().map(|e| entry_json(e)).collect();
    format!("{{\"repos\":[{}]}}", inner.join(","))
}

pub fn save(path: &Path, reg: &Registry) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create registry dir: {e}"))?;
        }
    }
    let sorted = Registry {
        version: reg.version,
        repos: {
            let mut v = reg.repos.clone();
            v.sort_by(|a, b| a.name.cmp(&b.name));
            v
        },
    };
    let inner: Vec<String> = sorted.repos.iter().map(entry_json).collect();
    let text = format!(
        "{{\"version\":{},\"repos\":[{}]}}",
        sorted.version,
        inner.join(",")
    );
    std::fs::write(path, text).map_err(|e| format!("cannot write registry: {e}"))?;
    Ok(())
}

/// Missing file → empty registry. Unparseable file → error (user data at stake).
pub fn load(path: &Path) -> Result<Registry, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Registry {
                version: 1,
                repos: Vec::new(),
            });
        }
        Err(e) => return Err(format!("cannot read registry: {e}")),
    };
    parse_registry(&text)
}

#[derive(Debug, PartialEq)]
enum Val {
    Str(String),
    Num(u64),
    Bool(bool),
    Null,
    Arr(Vec<Val>),
    Obj(Vec<(String, Val)>),
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err("malformed registry JSON".to_string())
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.ws();
        if self.peek() != Some(b'"') {
            return Err("malformed registry JSON".to_string());
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            let c = *self.bytes.get(self.pos).ok_or("malformed registry JSON")?;
            self.pos += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = *self.bytes.get(self.pos).ok_or("malformed registry JSON")?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if self.pos + 4 > self.bytes.len() {
                                return Err("malformed registry JSON".to_string());
                            }
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                .map_err(|_| "malformed registry JSON".to_string())?;
                            let code = u32::from_str_radix(hex, 16)
                                .map_err(|_| "malformed registry JSON".to_string())?;
                            out.push(char::from_u32(code).ok_or("malformed registry JSON")?);
                            self.pos += 4;
                        }
                        _ => return Err("malformed registry JSON".to_string()),
                    }
                }
                0x00..=0x1F => return Err("malformed registry JSON".to_string()),
                _ => {
                    // Copy through the full UTF-8 sequence for this character.
                    let s = std::str::from_utf8(&self.bytes[self.pos - 1..])
                        .map_err(|_| "malformed registry JSON".to_string())?;
                    let ch = s.chars().next().ok_or("malformed registry JSON")?;
                    out.push(ch);
                }
            }
        }
    }

    fn number(&mut self) -> Result<u64, String> {
        self.ws();
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if start == self.pos {
            return Err("malformed registry JSON".to_string());
        }
        std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "malformed registry JSON".to_string())?
            .parse::<u64>()
            .map_err(|_| "malformed registry JSON".to_string())
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        self.ws();
        if self.bytes.get(self.pos..self.pos + word.len()) == Some(word.as_bytes()) {
            self.pos += word.len();
            Ok(())
        } else {
            Err("malformed registry JSON".to_string())
        }
    }

    fn value(&mut self) -> Result<Val, String> {
        self.ws();
        match self.peek() {
            Some(b'"') => Ok(Val::Str(self.string()?)),
            Some(b'{') => {
                self.pos += 1;
                let mut obj = Vec::new();
                self.ws();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    return Ok(Val::Obj(obj));
                }
                loop {
                    let key = self.string()?;
                    self.expect(b':')?;
                    let val = self.value()?;
                    obj.push((key, val));
                    self.ws();
                    match self.peek() {
                        Some(b',') => {
                            self.pos += 1;
                        }
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(Val::Obj(obj));
                        }
                        _ => return Err("malformed registry JSON".to_string()),
                    }
                }
            }
            Some(b'[') => {
                self.pos += 1;
                let mut arr = Vec::new();
                self.ws();
                if self.peek() == Some(b']') {
                    self.pos += 1;
                    return Ok(Val::Arr(arr));
                }
                loop {
                    arr.push(self.value()?);
                    self.ws();
                    match self.peek() {
                        Some(b',') => {
                            self.pos += 1;
                        }
                        Some(b']') => {
                            self.pos += 1;
                            return Ok(Val::Arr(arr));
                        }
                        _ => return Err("malformed registry JSON".to_string()),
                    }
                }
            }
            Some(b't') => self.literal("true").map(|_| Val::Bool(true)),
            Some(b'f') => self.literal("false").map(|_| Val::Bool(false)),
            Some(b'n') => self.literal("null").map(|_| Val::Null),
            Some(b'0'..=b'9') => Ok(Val::Num(self.number()?)),
            _ => Err("malformed registry JSON".to_string()),
        }
    }
}

fn field<'a>(obj: &'a [(String, Val)], key: &str) -> Option<&'a Val> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn parse_registry(text: &str) -> Result<Registry, String> {
    let mut p = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let root = p.value()?;
    p.ws();
    if p.pos != p.bytes.len() {
        return Err("malformed registry JSON".to_string());
    }
    let Val::Obj(top) = root else {
        return Err("malformed registry JSON".to_string());
    };
    let version = match field(&top, "version") {
        Some(Val::Num(n)) => *n as u32,
        _ => return Err("malformed registry JSON".to_string()),
    };
    let repos = match field(&top, "repos") {
        Some(Val::Arr(items)) => items
            .iter()
            .map(|item| {
                let Val::Obj(entry) = item else {
                    return Err("malformed registry JSON".to_string());
                };
                let (Some(Val::Str(name)), Some(Val::Str(root)), Some(Val::Num(added))) = (
                    field(entry, "name"),
                    field(entry, "root"),
                    field(entry, "added_unix"),
                ) else {
                    return Err("malformed registry JSON".to_string());
                };
                Ok(RepoEntry {
                    name: name.clone(),
                    root: root.clone(),
                    added_unix: *added,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("malformed registry JSON".to_string()),
    };
    Ok(Registry { version, repos })
}

#[cfg(test)]
mod tests {
    use super::{add, entry_json, find, list_json, load, save, Registry};
    use crate::testutil::tmpdir;

    fn reg_path() -> std::path::PathBuf {
        tmpdir("registry").join("registry.json")
    }

    #[test]
    fn round_trips_through_file_sorted_by_name() {
        let path = reg_path();
        let mut reg = Registry {
            version: 1,
            repos: Vec::new(),
        };
        assert!(add(&mut reg, "zeta", "C:\\z", 10).unwrap());
        assert!(add(&mut reg, "alpha", "C:\\a", 20).unwrap());
        save(&path, &reg).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.version, 1);
        assert_eq!(back.repos.len(), 2);
        assert_eq!(back.repos[0].name, "alpha");
        assert_eq!(find(&back, "zeta").unwrap().root, "C:\\z");
        assert!(find(&back, "missing").is_none());
    }

    #[test]
    fn re_adding_same_root_is_idempotent() {
        let mut reg = Registry {
            version: 1,
            repos: Vec::new(),
        };
        assert!(add(&mut reg, "a", "C:\\a", 1).unwrap());
        assert!(!add(&mut reg, "a", "C:\\a", 2).unwrap());
        assert_eq!(reg.repos.len(), 1);
        assert_eq!(reg.repos[0].added_unix, 1);
    }

    #[test]
    fn rejects_name_clash_and_bad_names() {
        let mut reg = Registry {
            version: 1,
            repos: Vec::new(),
        };
        add(&mut reg, "a", "C:\\a", 1).unwrap();
        assert!(add(&mut reg, "a", "C:\\b", 1).is_err());
        assert!(add(&mut reg, "../evil", "C:\\b", 1).is_err());
        assert!(add(&mut reg, "", "C:\\b", 1).is_err());
    }

    #[test]
    fn missing_file_loads_empty_and_corrupt_file_errors() {
        let dir = tmpdir("registry-missing");
        assert_eq!(load(&dir.join("nope.json")).unwrap().repos.len(), 0);
        let bad = dir.join("bad.json");
        std::fs::write(&bad, "{not json").unwrap();
        assert!(load(&bad).is_err());
    }

    #[test]
    fn list_json_is_deterministic_and_sorted() {
        let mut reg = Registry {
            version: 1,
            repos: Vec::new(),
        };
        add(&mut reg, "b", "R:\\b", 2).unwrap();
        add(&mut reg, "a", "R:\\a", 1).unwrap();
        assert_eq!(
            list_json(&reg),
            "{\"repos\":[{\"name\":\"a\",\"root\":\"R:\\\\a\",\"added_unix\":1},{\"name\":\"b\",\"root\":\"R:\\\\b\",\"added_unix\":2}]}"
        );
        assert_eq!(
            entry_json(&reg.repos[0]),
            "{\"name\":\"a\",\"root\":\"R:\\\\a\",\"added_unix\":1}"
        );
    }
}
