// SPDX-License-Identifier: Apache-2.0
//! Minimal JSON string escaping with deterministic key order by construction.
//! Only what the CLI emits: objects with fixed keys, string/bool/u64/null values.

/// Escape a string for embedding in double quotes. Non-ASCII printable
/// characters pass through as UTF-8; C0 controls and DEL become `\u00XX`.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_quote_backslash_and_newline() {
        assert_eq!(escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
    }

    #[test]
    fn escapes_tab_cr_and_control() {
        assert_eq!(escape("\t\r\x01"), "\\t\\r\\u0001");
    }

    #[test]
    fn escapes_del_and_passes_unicode_through() {
        assert_eq!(escape("\u{7f}héllo🦀"), "\\u007fhéllo🦀");
    }

    #[test]
    fn leaves_plain_ascii_untouched() {
        assert_eq!(
            escape("C:\\repos\\riltra-1_2.0"),
            "C:\\\\repos\\\\riltra-1_2.0"
        );
    }
}
