//! The model as deterministic, reviewable JSON (`ferrule-gen model` and the snapshots).
//!
//! Two-space indent, members in declaration order, and a value written on one line when it fits
//! in [`WIDTH`] columns, so a name or a small object reads as one line in a diff.

use serde_json::Value;

use crate::model::Client;

/// The column a one-line value may reach, as the repository's other formatters use.
pub const WIDTH: usize = 100;

/// The model as JSON with a trailing newline; `null` for a document without operations.
///
/// # Panics
///
/// Never: every value in the model is representable as JSON.
#[must_use]
pub fn to_json(client: Option<&Client>) -> String {
    let value = serde_json::to_value(client).expect("the model serializes");
    let mut out = String::new();
    write(&value, 0, 0, &mut out);
    out.push('\n');
    out
}

fn scalar(v: &Value) -> String {
    serde_json::to_string(v).expect("a JSON scalar serializes")
}

/// `v` on one line: `{"a": 1, "b": [true]}`.
fn flat(v: &Value) -> String {
    match v {
        Value::Array(items) => format!("[{}]", items.iter().map(flat).collect::<Vec<_>>().join(", ")),
        Value::Object(members) => format!(
            "{{{}}}",
            members
                .iter()
                .map(|(k, v)| format!("{}: {}", scalar(&Value::String(k.clone())), flat(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => scalar(v),
    }
}

/// Writes `v` at `indent` levels, its first line starting at column `column`.
fn write(v: &Value, indent: usize, column: usize, out: &mut String) {
    let one_line = flat(v);
    let fits = column + one_line.chars().count() < WIDTH;
    let pad = |level: usize| "  ".repeat(level);
    match v {
        Value::Array(items) if !fits && !items.is_empty() => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                let inner = pad(indent + 1);
                out.push_str(&inner);
                write(item, indent + 1, inner.len(), out);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(indent));
            out.push(']');
        }
        Value::Object(members) if !fits && !members.is_empty() => {
            out.push_str("{\n");
            for (i, (k, item)) in members.iter().enumerate() {
                let key = format!("{}{}: ", pad(indent + 1), scalar(&Value::String(k.clone())));
                out.push_str(&key);
                write(item, indent + 1, key.chars().count(), out);
                out.push_str(if i + 1 < members.len() { ",\n" } else { "\n" });
            }
            out.push_str(&pad(indent));
            out.push('}');
        }
        _ => out.push_str(&one_line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn print(v: &Value) -> String {
        let mut out = String::new();
        write(v, 0, 0, &mut out);
        out
    }

    #[test]
    fn writes_what_fits_on_one_line() {
        assert_eq!(
            print(&json!({ "a": { "written": "api_key", "words": ["api", "key"] }, "b": [] })),
            r#"{"a": {"written": "api_key", "words": ["api", "key"]}, "b": []}"#
        );
    }

    #[test]
    fn breaks_what_does_not_fit() {
        let long = "x".repeat(90);
        let printed = print(&json!({ "a": [long, 1], "b": { "c": null } }));
        assert_eq!(
            printed,
            format!("{{\n  \"a\": [\n    \"{long}\",\n    1\n  ],\n  \"b\": {{\"c\": null}}\n}}")
        );
        assert!(
            printed
                .lines()
                .all(|l| l.chars().count() <= WIDTH || l.trim_start().starts_with('"'))
        );
    }

    #[test]
    fn prints_null_for_no_model() {
        assert_eq!(to_json(None), "null\n");
    }
}
