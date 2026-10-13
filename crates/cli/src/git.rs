// SPDX-License-Identifier: Apache-2.0
//! Git fingerprint via read-only plumbing (`rev-parse`, `status --porcelain`).
//! Never executes repository code; `GIT_TERMINAL_PROMPT=0` blocks auth prompts
//! and `--no-optional-locks` keeps status read-only.

use std::path::Path;

use crate::json::escape;

#[derive(Debug, Clone, PartialEq)]
pub struct GitInfo {
    pub present: bool,
    pub valid: bool,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub untracked: u32,
    pub toplevel: Option<String>,
    pub worktree: bool,
}

pub fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git_out(root: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn absent() -> GitInfo {
    GitInfo {
        present: false,
        valid: false,
        head: None,
        branch: None,
        dirty: false,
        untracked: 0,
        toplevel: None,
        worktree: false,
    }
}

/// How this root claims (or not) to be a repository.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DotGit {
    /// Real directory: this root IS the worktree top, nothing else qualifies.
    Dir,
    /// File gitlink (worktree/submodule) or symlink/junction: follow git.
    Link,
}

fn canonical_str(p: &Path) -> Option<String> {
    std::fs::canonicalize(p)
        .ok()
        .map(|c| c.to_string_lossy().to_string())
}

/// Fingerprint the repository claimed at `root`. Never borrows a parent
/// repository: without a `.git` entry the answer is absent even inside a
/// larger work tree, and a directory `.git` only validates when the discovered
/// toplevel is this root itself (corrupt metadata or parent bleed → invalid).
pub fn fingerprint(root: &Path) -> GitInfo {
    let claim = match std::fs::symlink_metadata(root.join(".git")) {
        Err(_) => return absent(),
        Ok(m) if m.file_type().is_dir() => DotGit::Dir,
        Ok(_) => DotGit::Link,
    };
    let canon_root = match canonical_str(root) {
        Some(c) => c,
        None => {
            return GitInfo {
                present: true,
                ..absent()
            }
        }
    };
    let discovered =
        git_out(root, &["rev-parse", "--show-toplevel"]).and_then(|s| canonical_str(Path::new(&s)));
    let toplevel = match (claim, discovered) {
        (DotGit::Dir, Some(t)) if t == canon_root => Some(t),
        (DotGit::Link, Some(t)) => Some(t),
        _ => None,
    };
    let Some(toplevel) = toplevel else {
        return GitInfo {
            present: true,
            ..absent()
        };
    };
    let head = git_out(root, &["rev-parse", "HEAD"])
        .filter(|s| s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()));
    let branch = git_out(root, &["branch", "--show-current"]).filter(|s| !s.is_empty());
    let status = git_out(
        root,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    );
    // No status output without a successful run: an unverifiable work tree is
    // never reported clean, and the fingerprint is not valid (a failed status
    // used to fall through as clean + valid).
    let (dirty, untracked, status_ok) = match status {
        None => (false, 0, false),
        Some(text) => {
            let mut dirty = false;
            let mut untracked = 0u32;
            for line in text.lines() {
                if line.starts_with("??") {
                    untracked += 1;
                } else if !line.is_empty() {
                    dirty = true;
                }
            }
            (dirty, untracked, true)
        }
    };
    let git_dir = git_out(root, &["rev-parse", "--absolute-git-dir"])
        .and_then(|s| canonical_str(Path::new(&s)));
    // `--git-common-dir` may print relative (main repo) or absolute
    // (linked worktree); resolve against the root either way.
    let common_dir = git_out(root, &["rev-parse", "--git-common-dir"])
        .and_then(|s| canonical_str(&root.join(&s)));
    let worktree = match (git_dir, common_dir) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    };
    GitInfo {
        present: true,
        valid: status_ok,
        head,
        branch,
        dirty,
        untracked,
        toplevel: Some(toplevel),
        worktree,
    }
}

fn opt_json(value: &Option<String>) -> String {
    match value {
        Some(s) => format!("\"{}\"", escape(s)),
        None => "null".to_string(),
    }
}

/// Fixed key order: present, valid, head, branch, dirty, untracked, toplevel, worktree.
pub fn git_json(g: &GitInfo) -> String {
    format!(
        "{{\"present\":{},\"valid\":{},\"head\":{},\"branch\":{},\"dirty\":{},\"untracked\":{},\"toplevel\":{},\"worktree\":{}}}",
        g.present,
        g.valid,
        opt_json(&g.head),
        opt_json(&g.branch),
        g.dirty,
        g.untracked,
        opt_json(&g.toplevel),
        g.worktree
    )
}

#[cfg(test)]
mod tests {
    use super::{fingerprint, git_available, git_json};
    use crate::testutil::{git_repo, tmpdir};

    #[test]
    fn fingerprints_clean_repo_with_branch() {
        let Some(dir) = git_repo("git-clean", "main") else {
            return;
        };
        let g = fingerprint(&dir);
        assert!(g.present && g.valid);
        assert_eq!(g.head.as_ref().map(String::len), Some(40));
        assert_eq!(g.branch.as_deref(), Some("main"));
        assert!(!g.dirty);
        assert_eq!(g.untracked, 0);
        assert!(!g.worktree);
        assert!(g.toplevel.is_some());
    }

    #[test]
    fn detects_dirty_and_untracked() {
        let Some(dir) = git_repo("git-dirty", "main") else {
            return;
        };
        std::fs::write(dir.join("a.txt"), "changed\n").unwrap();
        std::fs::write(dir.join("new.txt"), "new\n").unwrap();
        let g = fingerprint(&dir);
        assert!(g.valid && g.dirty);
        assert_eq!(g.untracked, 1);
    }

    #[test]
    fn detects_linked_worktree() {
        if !git_available() {
            return;
        }
        let Some(dir) = git_repo("git-wt", "main") else {
            return;
        };
        let wt = dir.join("wt");
        let st = std::process::Command::new("git")
            .args(["worktree", "add", "--detach"])
            .arg(&wt)
            .current_dir(&dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .expect("run git worktree");
        if !st.success() {
            return;
        }
        let g = fingerprint(&wt);
        assert!(g.present && g.valid && g.worktree);
        assert!(git_json(&g).contains("\"worktree\":true"));
    }

    #[test]
    fn non_git_dir_is_not_present_and_corrupt_is_invalid() {
        let dir = tmpdir("git-plain");
        let g = fingerprint(&dir);
        assert!(!g.present && !g.valid);
        assert!(g.head.is_none() && g.branch.is_none());

        let corrupt = tmpdir("git-corrupt");
        std::fs::create_dir_all(corrupt.join(".git")).unwrap();
        std::fs::write(corrupt.join(".git").join("HEAD"), "garbage-no-newline").unwrap();
        let c = fingerprint(&corrupt);
        assert!(c.present && !c.valid);

        // Broken worktree pointer: git refuses the gitfile and (unlike the
        // directory case) never walks up into a parent repository.
        let corrupt_file = tmpdir("git-corrupt-file");
        std::fs::write(corrupt_file.join(".git"), "garbage").unwrap();
        let f = fingerprint(&corrupt_file);
        assert!(f.present && !f.valid);
    }

    #[test]
    fn status_failure_is_never_clean_nor_valid() {
        let Some(dir) = git_repo("git-status-fail", "main") else {
            return;
        };
        // Real modification that status must report...
        std::fs::write(dir.join("a.txt"), "changed\n").unwrap();
        // ...then destroy the evidence channel: rev-parse still works,
        // but status cannot verify anything.
        let index = std::fs::File::options()
            .write(true)
            .open(dir.join(".git").join("index"))
            .unwrap();
        index.set_len(20).unwrap();
        let g = fingerprint(&dir);
        assert!(g.present);
        assert!(
            !g.valid,
            "unverifiable status must not be reported valid (and its clean/dirty flags are meaningless)"
        );
    }

    #[test]
    fn git_json_has_fixed_key_order_with_nulls() {
        let dir = tmpdir("git-json");
        let g = fingerprint(&dir);
        assert_eq!(
            git_json(&g),
            "{\"present\":false,\"valid\":false,\"head\":null,\"branch\":null,\"dirty\":false,\"untracked\":0,\"toplevel\":null,\"worktree\":false}"
        );
    }
}
