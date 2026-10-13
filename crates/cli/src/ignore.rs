// SPDX-License-Identifier: Apache-2.0
//! `.riltraignore`: line-based exact/prefix matching. `#` comments and blanks
//! skipped; `!` negations unsupported by design (reported, never matched).

use std::path::Path;

use crate::paths::contains;

#[derive(Debug, Clone, PartialEq)]
pub struct IgnoreInfo {
    pub file_present: bool,
    pub rules: u32,
}

pub fn load_rules(root: &Path) -> (Vec<String>, IgnoreInfo) {
    let absent = || {
        (
            Vec::new(),
            IgnoreInfo {
                file_present: false,
                rules: 0,
            },
        )
    };
    // Never follow the ignore file outside the root it belongs to: resolve it
    // once and require containment (symlink/junction retarget). An escaped
    // ignore file reads as absent — conservative, no ignores applied.
    let canon_root = match root.canonicalize() {
        Ok(c) => c,
        Err(_) => return absent(),
    };
    let canon_file = match root.join(".riltraignore").canonicalize() {
        Ok(c) if contains(&canon_root, &c) => c,
        _ => return absent(),
    };
    let text = match std::fs::read_to_string(&canon_file) {
        Ok(t) => t,
        Err(_) => return absent(),
    };
    let mut rules = Vec::new();
    for line in text.lines() {
        let rule = line.trim();
        if rule.is_empty() || rule.starts_with('#') || rule.starts_with('!') {
            continue;
        }
        rules.push(rule.strip_prefix("./").unwrap_or(rule).to_string());
    }
    let count = rules.len() as u32;
    (
        rules,
        IgnoreInfo {
            file_present: true,
            rules: count,
        },
    )
}

/// Exact match, or dir-prefix match for rules ending in `/`.
/// Matching is path-component aware via the required `/` boundary.
// shortcut: unit-tested; production consumer is M2 file-scoped queries
// (M1 only reports rule counts).
#[allow(dead_code)]
pub fn is_ignored(rules: &[String], rel: &str) -> bool {
    let rel = rel.strip_prefix("./").unwrap_or(rel);
    rules.iter().any(|rule| {
        if let Some(dir) = rule.strip_suffix('/') {
            *rel == *dir || rel.starts_with(&format!("{dir}/"))
        } else {
            rel == *rule
        }
    })
}

pub fn ignore_json(info: &IgnoreInfo) -> String {
    format!(
        "{{\"file\":{},\"rules\":{}}}",
        info.file_present, info.rules
    )
}

#[cfg(test)]
mod tests {
    use super::{ignore_json, is_ignored, load_rules};
    use crate::testutil::tmpdir;

    #[test]
    fn matches_exact_and_dir_prefix_only() {
        let rules = vec!["target".to_string(), "build/".to_string()];
        assert!(is_ignored(&rules, "target"));
        assert!(is_ignored(&rules, "build/out.o"));
        assert!(!is_ignored(&rules, "targets"));
        assert!(!is_ignored(&rules, "src/build/x"));
        assert!(!is_ignored(&rules, "other"));
    }

    #[test]
    fn loads_file_skipping_comments_and_blanks() {
        let dir = tmpdir("ignore-load");
        std::fs::write(dir.join(".riltraignore"), "# c\n\ntarget\n./gen/\n").unwrap();
        let (rules, info) = load_rules(&dir);
        assert!(info.file_present);
        assert_eq!(info.rules, 2);
        assert!(is_ignored(&rules, "gen/a"));
    }

    #[test]
    fn missing_file_reports_absent_with_zero_rules() {
        let dir = tmpdir("ignore-missing");
        let (rules, info) = load_rules(&dir);
        assert!(!info.file_present);
        assert_eq!(info.rules, 0);
        assert!(rules.is_empty());
        assert_eq!(ignore_json(&info), "{\"file\":false,\"rules\":0}");
    }

    #[test]
    fn ignore_symlink_outside_root_is_not_followed() {
        let dir = tmpdir("ignore-outside");
        let outside = tmpdir("ignore-outside-target");
        std::fs::write(outside.join("rules.txt"), "secret-rule\n").unwrap();
        let link = dir.join(".riltraignore");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(outside.join("rules.txt"), &link).is_ok();
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(outside.join("rules.txt"), &link).is_ok();
        if !made {
            return; // Privilege skip on Windows; unix runs unprivileged (see CI).
        }
        let (rules, info) = load_rules(&dir);
        assert!(
            !info.file_present,
            "escaped ignore file must read as absent"
        );
        assert!(rules.is_empty());
    }

    #[test]
    fn ignore_symlink_retarget_outside_is_not_followed() {
        let dir = tmpdir("ignore-retarget");
        let outside = tmpdir("ignore-retarget-target");
        let inside = dir.join("real-rules");
        std::fs::write(&inside, "aaa\n").unwrap();
        std::fs::write(outside.join("evil-rules"), "e1\ne2\ne3\ne4\ne5\n").unwrap();
        let link = dir.join(".riltraignore");
        #[cfg(unix)]
        let mut made = std::os::unix::fs::symlink(&inside, &link).is_ok();
        #[cfg(windows)]
        let mut made = std::os::windows::fs::symlink_file(&inside, &link).is_ok();
        if !made {
            return; // Privilege skip on Windows; unix runs unprivileged (see CI).
        }
        let (rules, info) = load_rules(&dir);
        assert!(
            info.file_present && info.rules == 1,
            "inside target must be honored"
        );
        assert_eq!(rules, vec!["aaa".to_string()]);
        std::fs::remove_file(&link).unwrap();
        #[cfg(unix)]
        {
            made = std::os::unix::fs::symlink(outside.join("evil-rules"), &link).is_ok();
        }
        #[cfg(windows)]
        {
            made = std::os::windows::fs::symlink_file(outside.join("evil-rules"), &link).is_ok();
        }
        assert!(
            made,
            "retarget link creation must succeed where the first one did"
        );
        let (rules, info) = load_rules(&dir);
        assert!(!info.file_present, "retargeted escape must read as absent");
        assert!(rules.is_empty());
    }

    #[test]
    fn ignore_hardlink_inside_is_read_not_falsely_rejected() {
        // A hardlink is the same file under an inside name: contained, so its
        // rules apply. Needs no privilege on any OS (fails loudly otherwise).
        let dir = tmpdir("ignore-hardlink");
        let outside = tmpdir("ignore-hardlink-target");
        std::fs::write(outside.join("rules.txt"), "hh\n").unwrap();
        std::fs::hard_link(outside.join("rules.txt"), dir.join(".riltraignore"))
            .expect("hardlink creation");
        let (rules, info) = load_rules(&dir);
        assert!(info.file_present);
        assert!(is_ignored(&rules, "hh"));
    }
}
