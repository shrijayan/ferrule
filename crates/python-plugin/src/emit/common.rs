//! Helpers shared by the emitters. Port of `python-plugin/src/emit/common.ts`.

/// First line of every generated file (no vendor branding, per the plan).
pub const HEADER: &str = "# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.";

/// A Python string literal for any text. JSON's escaping (quotes, backslashes, control characters
/// as `\uXXXX`) is valid in a Python `"..."` literal, so no text can end it. Same bytes as
/// JavaScript's `JSON.stringify(text)`.
#[must_use]
pub fn py_string(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::py_string;

    #[test]
    fn escapes_like_json_stringify() {
        assert_eq!(py_string("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
    }
}
