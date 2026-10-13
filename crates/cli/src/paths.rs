// SPDX-License-Identifier: Apache-2.0
//! Path security: canonical roots, containment checks, registry key validation.
//! Symlinks/junctions resolve via `canonicalize`, so an escape through a link
//! never compares equal to (or under) the registered root.

use std::io;
use std::path::{Path, PathBuf};

/// Config dir for the registry. `RILTRA_HOME` (used by tests) points at the
/// directory itself; otherwise the OS per-user config location applies.
pub fn config_dir() -> PathBuf {
    if let Ok(h) = std::env::var("RILTRA_HOME") {
        if !h.is_empty() {
            return PathBuf::from(h);
        }
    }
    #[cfg(windows)]
    if let Ok(appdata) = std::env::var("APPDATA") {
        let mut p = PathBuf::from(appdata);
        p.push("riltra");
        return p;
    }
    #[cfg(not(windows))]
    if let Ok(home) = std::env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p.push("riltra");
        return p;
    }
    let mut p = std::env::temp_dir();
    p.push("riltra");
    p
}

/// Pure override form of [`config_dir`] (testable without touching env).
// shortcut: test seam today; production caller when config grows (post-M1).
#[allow(dead_code)]
pub fn config_dir_from(home_override: Option<&str>) -> PathBuf {
    match home_override {
        Some(h) => {
            let mut p = PathBuf::from(h);
            p.push("riltra");
            p
        }
        None => config_dir(),
    }
}

pub fn registry_path() -> PathBuf {
    let mut p = config_dir();
    p.push("registry.json");
    p
}

/// Resolve `p` to its canonical directory form. Fails for missing paths and
/// non-directories so only real roots get registered.
pub fn canonical_root(p: &Path) -> io::Result<PathBuf> {
    let c = p.canonicalize()?;
    if c.is_dir() {
        Ok(c)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "not a directory",
        ))
    }
}

/// True only when `candidate` resolves to the root itself or below it.
/// Missing paths and symlink escapes resolve outside (or fail) → false.
// shortcut: unit-tested trust boundary for M2 file-scoped queries; M1 enforces
// names + canonical records, so no production caller yet.
#[allow(dead_code)]
pub fn contains(canonical_root: &Path, candidate: &Path) -> bool {
    match candidate.canonicalize() {
        Ok(c) => c == *canonical_root || c.starts_with(canonical_root),
        Err(_) => false,
    }
}

/// Registry keys are plain names: no separators, dots, controls, or empties.
pub fn validate_name(name: &str) -> bool {
    !(name.is_empty()
        || name == "."
        || name == ".."
        || name.len() > 128
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.chars().any(|c| c.is_control()))
}

#[cfg(test)]
mod tests {
    use super::{canonical_root, config_dir_from, contains, validate_name};
    use crate::testutil::tmpdir;

    #[test]
    fn contains_accepts_root_itself_and_children() {
        let dir = tmpdir("paths-in");
        let root = canonical_root(&dir).unwrap();
        let child = dir.join("sub").join("f.txt");
        std::fs::create_dir_all(child.parent().unwrap()).unwrap();
        std::fs::write(&child, "x").unwrap();
        assert!(contains(&root, &root));
        assert!(contains(&root, &child));
    }

    #[test]
    fn contains_rejects_parent_and_sibling_escape() {
        let dir = tmpdir("paths-out");
        let root = canonical_root(&dir).unwrap();
        assert!(!contains(&root, &dir.join("..").join("evil")));
        assert!(!contains(&root, &dir.join("..")));
    }

    #[test]
    fn contains_rejects_missing_and_file_root() {
        let dir = tmpdir("paths-missing");
        let root = canonical_root(&dir).unwrap();
        assert!(!contains(&root, &dir.join("nope.txt")));
        let file = dir.join("f.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(canonical_root(&file).is_err());
    }

    #[test]
    fn contains_rejects_symlink_escape() {
        let dir = tmpdir("paths-link");
        let outside = tmpdir("paths-link-out");
        let root = canonical_root(&dir).unwrap();
        let link = dir.join("escape");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&outside, &link).is_ok();
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_dir(&outside, &link).is_ok();
        if !made {
            return; // OS refused symlinks here; nothing to prove.
        }
        assert!(!contains(&root, &link));
        assert!(!contains(&root, &link.join("x")));
    }

    #[test]
    fn config_dir_prefers_override_then_os_default() {
        let p = config_dir_from(Some("/tmp/riltra-test-home"));
        assert!(p.ends_with("riltra"));
        let d = config_dir_from(None);
        assert!(d.ends_with("riltra"));
    }

    #[test]
    fn validate_name_rejects_separators_and_dots() {
        assert!(validate_name("my-repo_1"));
        assert!(!validate_name(""));
        assert!(!validate_name("."));
        assert!(!validate_name(".."));
        assert!(!validate_name("../evil"));
        assert!(!validate_name("a/b"));
        assert!(!validate_name("a\\b"));
    }
}
