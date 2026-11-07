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

/// Recursively match serde_json's default BTreeMap key ordering, even when an
/// optional dependency enables the `preserve_order` feature for the binary.
#[cfg(feature = "semantic-knowledge")]
pub fn sort_value_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            let mut entries: Vec<_> = std::mem::take(object).into_iter().collect();
            for (_, value) in &mut entries {
                sort_value_keys(value);
            }
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            object.extend(entries);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sort_value_keys(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::escape;
    #[cfg(feature = "semantic-knowledge")]
    use super::sort_value_keys;

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

    #[cfg(feature = "semantic-knowledge")]
    #[test]
    fn sorts_nested_json_objects_without_reordering_arrays() {
        let mut value = serde_json::json!({
            "z": { "y": 1, "a": 2 },
            "items": [{ "d": 4, "b": 3 }],
            "a": 0,
        });
        sort_value_keys(&mut value);
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"{"a":0,"items":[{"b":3,"d":4}],"z":{"a":2,"y":1}}"#
        );
    }
}
