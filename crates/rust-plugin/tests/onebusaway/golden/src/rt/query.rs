//! Query strings and path segments, encoded as the API's `query_settings` say.

use serde_json::Value;

/// How an array query parameter is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayFormat {
    /// `a=1,2`
    Comma,
    /// `a=1&a=2`
    Repeat,
    /// `a[0]=1&a[1]=2`
    Indices,
    /// `a[]=1&a[]=2`
    Brackets,
}

/// How an object query parameter is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedFormat {
    /// `a[b]=1`
    Brackets,
    /// `a.b=1`
    Dots,
}

/// The query string (without `?`) of `params`, in order; `null` values are left out.
pub(crate) fn encode(
    params: &[(String, Value)],
    arrays: ArrayFormat,
    nested: NestedFormat,
) -> String {
    let mut pairs = Vec::new();
    for (key, value) in params {
        flatten(key, value, arrays, nested, &mut pairs);
    }
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", percent(k), percent(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn flatten(
    key: &str,
    value: &Value,
    arrays: ArrayFormat,
    nested: NestedFormat,
    out: &mut Vec<(String, String)>,
) {
    match value {
        Value::Null => {}
        Value::Array(items) => match arrays {
            ArrayFormat::Comma => {
                let joined: Vec<String> = items.iter().filter_map(scalar).collect();
                out.push((key.to_owned(), joined.join(",")));
            }
            ArrayFormat::Repeat => items
                .iter()
                .for_each(|v| flatten(key, v, arrays, nested, out)),
            ArrayFormat::Indices => {
                for (i, v) in items.iter().enumerate() {
                    flatten(&format!("{key}[{i}]"), v, arrays, nested, out);
                }
            }
            ArrayFormat::Brackets => items
                .iter()
                .for_each(|v| flatten(&format!("{key}[]"), v, arrays, nested, out)),
        },
        Value::Object(fields) => {
            for (k, v) in fields {
                let key = match nested {
                    NestedFormat::Brackets => format!("{key}[{k}]"),
                    NestedFormat::Dots => format!("{key}.{k}"),
                };
                flatten(&key, v, arrays, nested, out);
            }
        }
        _ => out.extend(scalar(value).map(|v| (key.to_owned(), v))),
    }
}

/// A scalar as text: strings unquoted, numbers and booleans as JSON writes them.
pub(crate) fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

/// Percent-encodes everything but RFC 3986's unreserved characters.
pub(crate) fn percent(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn query(value: &Value, arrays: ArrayFormat, nested: NestedFormat) -> String {
        encode(&[("a".to_owned(), value.clone())], arrays, nested)
    }

    #[test]
    fn encodes_arrays_in_each_format() {
        let v = json!([1, "x y"]);
        assert_eq!(
            query(&v, ArrayFormat::Comma, NestedFormat::Brackets),
            "a=1%2Cx%20y"
        );
        assert_eq!(
            query(&v, ArrayFormat::Repeat, NestedFormat::Brackets),
            "a=1&a=x%20y"
        );
        assert_eq!(
            query(&v, ArrayFormat::Indices, NestedFormat::Brackets),
            "a%5B0%5D=1&a%5B1%5D=x%20y"
        );
        assert_eq!(
            query(&v, ArrayFormat::Brackets, NestedFormat::Brackets),
            "a%5B%5D=1&a%5B%5D=x%20y"
        );
    }

    #[test]
    fn encodes_objects_and_skips_nulls() {
        let v = json!({"b": true, "c": null});
        assert_eq!(
            query(&v, ArrayFormat::Repeat, NestedFormat::Brackets),
            "a%5Bb%5D=true"
        );
        assert_eq!(
            query(&v, ArrayFormat::Repeat, NestedFormat::Dots),
            "a.b=true"
        );
        assert_eq!(
            query(&Value::Null, ArrayFormat::Repeat, NestedFormat::Dots),
            ""
        );
    }
}
