// SPDX-License-Identifier: Apache-2.0
//! Doctor: path-security and repository health checks with explicit pass/fail.
//! Exit code follows the checks (0 all-pass, 3 otherwise); JSON always on stdout.
//!
//! `pass` means "no problem found": absent git or absent ignore file is plain
//! informational (plain-directory mode is supported), while a missing root, a
//! moved root, or unreadable git metadata fails.

use std::path::Path;

use crate::git::GitInfo;
use crate::ignore::IgnoreInfo;
use crate::json::escape;

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

fn check(name: &str, pass: bool, detail: String) -> Check {
    Check {
        name: name.to_string(),
        pass,
        detail,
    }
}

/// `root` must already be canonicalized; `stored_root` is the registry record.
pub fn run_checks(
    name: &str,
    root: &Path,
    stored_root: &str,
    git_info: &GitInfo,
    ignore: &IgnoreInfo,
) -> Vec<Check> {
    let _ = name;
    let root_s = root.to_string_lossy().to_string();
    let git_valid_pass = !git_info.present || git_info.valid;
    vec![
        check(
            "root-exists-dir",
            root.is_dir(),
            if root.is_dir() {
                "registered root is a directory".to_string()
            } else {
                format!("root {root_s} is missing or not a directory")
            },
        ),
        check(
            "root-canonical-matches-record",
            root_s == stored_root,
            if root_s == stored_root {
                "canonical root matches registry record".to_string()
            } else {
                format!("canonical root {root_s} differs from record {stored_root}")
            },
        ),
        check(
            "git-present",
            true,
            if git_info.present {
                "git repository detected".to_string()
            } else {
                "plain directory (no git)".to_string()
            },
        ),
        check(
            "git-valid",
            git_valid_pass,
            if !git_info.present {
                "not applicable".to_string()
            } else if git_info.valid {
                "git metadata readable".to_string()
            } else {
                "git metadata unreadable (corrupt .git?)".to_string()
            },
        ),
        check(
            "ignore-file",
            true,
            if ignore.file_present {
                format!("{} rule(s) loaded from .riltraignore", ignore.rules)
            } else {
                "no .riltraignore".to_string()
            },
        ),
    ]
}

pub fn doctor_json(name: &str, root: &str, checks: &[Check]) -> String {
    let pass = checks.iter().all(|c| c.pass);
    let inner: Vec<String> = checks
        .iter()
        .map(|c| {
            format!(
                "{{\"name\":\"{}\",\"pass\":{},\"detail\":\"{}\"}}",
                escape(&c.name),
                c.pass,
                escape(&c.detail)
            )
        })
        .collect();
    format!(
        "{{\"name\":\"{}\",\"root\":\"{}\",\"pass\":{},\"checks\":[{}]}}",
        escape(name),
        escape(root),
        pass,
        inner.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::{doctor_json, run_checks, Check};
    use crate::git::GitInfo;
    use crate::ignore::IgnoreInfo;
    use crate::testutil::tmpdir;

    fn git_info(present: bool, valid: bool) -> GitInfo {
        GitInfo {
            present,
            valid,
            head: None,
            branch: None,
            dirty: false,
            untracked: 0,
            toplevel: None,
            worktree: false,
        }
    }

    fn ignore_info() -> IgnoreInfo {
        IgnoreInfo {
            file_present: false,
            rules: 0,
        }
    }

    fn check_named(checks: &[Check], name: &str) -> bool {
        checks
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.pass)
            .unwrap_or(false)
    }

    #[test]
    fn healthy_registered_dir_passes_all_checks() {
        let dir = tmpdir("doctor-ok");
        let root = dir.canonicalize().unwrap();
        let root_s = root.to_string_lossy().to_string();
        let checks = run_checks("r", &root, &root_s, &git_info(false, false), &ignore_info());
        assert!(checks.iter().all(|c| c.pass));
        let json = doctor_json("r", &root_s, &checks);
        assert!(json.contains("\"pass\":true"));
    }

    #[test]
    fn moved_root_fails_canonical_match() {
        let dir = tmpdir("doctor-moved");
        let root = dir.canonicalize().unwrap();
        let checks = run_checks(
            "r",
            &root,
            "C:\\stale\\root",
            &git_info(false, false),
            &ignore_info(),
        );
        assert!(!check_named(&checks, "root-canonical-matches-record"));
    }

    #[test]
    fn corrupt_git_fails_validity_check() {
        let dir = tmpdir("doctor-corrupt");
        let root = dir.canonicalize().unwrap();
        let root_s = root.to_string_lossy().to_string();
        let checks = run_checks("r", &root, &root_s, &git_info(true, false), &ignore_info());
        assert!(!check_named(&checks, "git-valid"));
        assert!(check_named(&checks, "git-present"));
    }

    #[test]
    fn doctor_json_reports_overall_and_each_check() {
        let checks = vec![Check {
            name: "a".to_string(),
            pass: true,
            detail: "ok".to_string(),
        }];
        assert_eq!(
            doctor_json("r", "R:\\r", &checks),
            "{\"name\":\"r\",\"root\":\"R:\\\\r\",\"pass\":true,\"checks\":[{\"name\":\"a\",\"pass\":true,\"detail\":\"ok\"}]}"
        );
    }
}
