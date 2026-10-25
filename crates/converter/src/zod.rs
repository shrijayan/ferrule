//! The few Zod 4 schema forms the TypeScript converter uses, with Zod's issue paths and
//! messages, so diagnostics built from them read the same.

use crate::diagnostic::Seg;
use crate::value::Value;

/// A Zod schema.
#[derive(Debug, Clone)]
pub enum Z {
    String,
    Boolean,
    Unknown,
    /// `z.looseObject`: listed fields are checked in order, other keys pass through.
    Object(Vec<(&'static str, Z, bool)>),
    /// `z.record(z.string(), inner)`.
    Record(Box<Z>),
    Array(Box<Z>),
    Union(Vec<Z>),
    Nullable(Box<Z>),
}

/// One Zod issue: where, which code and the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub path: Vec<Seg>,
    /// `invalid_type` or `invalid_union`.
    pub code: &'static str,
    pub message: String,
}

/// Zod's name for the type of a received value.
fn received(value: Option<&Value>) -> &'static str {
    match value {
        None => "undefined",
        Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "boolean",
        Some(Value::Number(n)) if n.is_nan() => "NaN",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Array(_)) => "array",
        Some(Value::Object(_)) => "object",
    }
}

fn invalid_type(expected: &str, value: Option<&Value>, path: &[Seg], out: &mut Vec<Issue>) {
    out.push(Issue {
        path: path.to_vec(),
        code: "invalid_type",
        message: format!("Invalid input: expected {expected}, received {}", received(value)),
    });
}

impl Z {
    /// `schema.safeParse(value)`: the issues, in Zod's order; empty when it parses.
    #[must_use]
    pub fn issues(&self, value: &Value) -> Vec<Issue> {
        let mut out = Vec::new();
        self.check(Some(value), &[], &mut out);
        out
    }

    fn check(&self, value: Option<&Value>, path: &[Seg], out: &mut Vec<Issue>) {
        match self {
            Z::Unknown => {}
            Z::String => {
                if !matches!(value, Some(Value::String(_))) {
                    invalid_type("string", value, path, out);
                }
            }
            Z::Boolean => {
                if !matches!(value, Some(Value::Bool(_))) {
                    invalid_type("boolean", value, path, out);
                }
            }
            Z::Nullable(inner) => {
                if !matches!(value, Some(Value::Null)) {
                    inner.check(value, path, out);
                }
            }
            Z::Object(fields) => {
                let Some(Value::Object(map)) = value else {
                    invalid_type("object", value, path, out);
                    return;
                };
                for (key, schema, optional) in fields {
                    let child = map.get(key);
                    if child.is_none() && *optional {
                        continue;
                    }
                    let mut p = path.to_vec();
                    p.push(Seg::Key((*key).to_owned()));
                    schema.check(child, &p, out);
                }
            }
            Z::Record(inner) => {
                let Some(Value::Object(map)) = value else {
                    invalid_type("record", value, path, out);
                    return;
                };
                for (key, child) in map {
                    let mut p = path.to_vec();
                    p.push(Seg::Key(key.clone()));
                    inner.check(Some(child), &p, out);
                }
            }
            Z::Array(inner) => {
                let Some(Value::Array(items)) = value else {
                    invalid_type("array", value, path, out);
                    return;
                };
                for (i, child) in items.iter().enumerate() {
                    let mut p = path.to_vec();
                    p.push(Seg::Index(i));
                    inner.check(Some(child), &p, out);
                }
            }
            Z::Union(options) => {
                let ok = options.iter().any(|o| {
                    let mut issues = Vec::new();
                    o.check(value, path, &mut issues);
                    issues.is_empty()
                });
                if !ok {
                    out.push(Issue {
                        path: path.to_vec(),
                        code: "invalid_union",
                        message: "Invalid input".to_owned(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn issues(z: &Z, v: &serde_json::Value) -> Vec<(String, String)> {
        z.issues(&Value::from(v))
            .into_iter()
            .map(|i| (crate::diagnostic::to_json_pointer(&i.path), i.message))
            .collect()
    }

    #[test]
    fn reports_types_unions_and_missing_fields_like_zod() {
        let target = Z::Union(vec![Z::String, Z::Array(Box::new(Z::String))]);
        let args = Z::Object(vec![("target", target, false), ("template", Z::Boolean, true)]);
        assert_eq!(
            issues(&args, &json!({"target": 5, "template": "x"})),
            [
                ("/target".to_owned(), "Invalid input".to_owned()),
                (
                    "/template".to_owned(),
                    "Invalid input: expected boolean, received string".to_owned()
                )
            ]
        );
        let plain = Z::Object(vec![("target", Z::String, false)]);
        assert_eq!(
            issues(&plain, &json!({})),
            [(
                "/target".to_owned(),
                "Invalid input: expected string, received undefined".to_owned()
            )]
        );
        let keys = Z::Object(vec![("keys", Z::Array(Box::new(Z::String)), true)]);
        assert_eq!(
            issues(&keys, &json!({"keys": [1, "b", null]})),
            [
                (
                    "/keys/0".to_owned(),
                    "Invalid input: expected string, received number".to_owned()
                ),
                (
                    "/keys/2".to_owned(),
                    "Invalid input: expected string, received null".to_owned()
                )
            ]
        );
        let rec = Z::Object(vec![("value", Z::Record(Box::new(Z::Unknown)), false)]);
        assert_eq!(
            issues(&rec, &json!({"value": [1]})),
            [(
                "/value".to_owned(),
                "Invalid input: expected record, received array".to_owned()
            )]
        );
    }
}
