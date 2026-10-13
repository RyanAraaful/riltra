// SPDX-License-Identifier: Apache-2.0
//! `.riltraignore`: line-based exact/prefix matching. `#` comments and blanks
//! skipped; `!` negations unsupported by design (reported, never matched).

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct IgnoreInfo {
    pub file_present: bool,
    pub rules: u32,
}

pub fn load_rules(root: &Path) -> (Vec<String>, IgnoreInfo) {
    let text = match std::fs::read_to_string(root.join(".riltraignore")) {
        Ok(t) => t,
        Err(_) => {
            return (
                Vec::new(),
                IgnoreInfo {
                    file_present: false,
                    rules: 0,
                },
            )
        }
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
}
