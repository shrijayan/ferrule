//! The fake server's reply bodies: the smallest JSON value of a type that its Rust type reads.
//!
//! Required fields only, so that pages stop after one page (their cursors are left out) and
//! nothing optional needs a value; the first enum member and union variant; empty arrays and
//! maps. Deterministic, like the rest of the output.

use ferrule_sdk_model::model::{Client, Literal, Scalar, Type};
use serde_json::{Map, Value};

use crate::types::fake_string;

/// The deepest a value nests before a required recursive reference is cut off.
const MAX_DEPTH: usize = 12;

/// The fake value of `ty`.
#[must_use]
pub fn value(client: &Client, ty: &Type) -> Value {
    at(client, ty, 0)
}

fn at(client: &Client, ty: &Type, depth: usize) -> Value {
    if depth > MAX_DEPTH {
        return Value::Null;
    }
    match ty {
        Type::Scalar { scalar } => scalar_value(*scalar),
        Type::Enum { members } => members.first().map_or(Value::Null, |m| match &m.value {
            Literal::String(s) => Value::String(s.clone()),
            Literal::Number(n) if n.fract() == 0.0 => {
                #[expect(clippy::cast_possible_truncation, reason = "an integral enum value")]
                let int = *n as i64;
                Value::from(int)
            }
            Literal::Number(n) => Value::from(*n),
            Literal::Boolean(b) => Value::Bool(*b),
        }),
        Type::Array { .. } => Value::Array(Vec::new()),
        Type::Map { .. } => Value::Object(Map::new()),
        Type::Nullable { .. } => Value::Null,
        Type::Object(o) => {
            let mut out = Map::new();
            for base in &o.bases {
                if let Some(Value::Object(fields)) = client.model(base).map(|d| at(client, &d.ty, depth + 1)) {
                    out.extend(fields);
                }
            }
            for f in o.fields.iter().filter(|f| f.required) {
                out.insert(f.wire_name.clone(), at(client, &f.ty, depth + 1));
            }
            Value::Object(out)
        }
        Type::Union(u) => u
            .variants
            .iter()
            .find(|v| !matches!(v.ty, Type::Scalar { scalar: Scalar::Null }))
            .map_or(Value::Null, |v| at(client, &v.ty, depth + 1)),
        Type::Ref(r) => client.model(r).map_or(Value::Null, |d| at(client, &d.ty, depth + 1)),
    }
}

fn scalar_value(scalar: Scalar) -> Value {
    match scalar {
        Scalar::Unknown | Scalar::Null => Value::Null,
        Scalar::Boolean => Value::Bool(true),
        Scalar::Float | Scalar::Double => Value::from(0.0),
        s if s.is_integer() => Value::from(0),
        s => Value::String(fake_string(s).to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrule_sdk_model::model::{Field, Name, Object};

    fn field(wire: &str, required: bool, ty: Type) -> Field {
        Field {
            name: Name {
                written: wire.to_owned(),
                words: vec![wire.to_owned()],
            },
            wire_name: wire.to_owned(),
            required,
            deprecated: false,
            read_only: false,
            description: String::new(),
            default: None,
            ty,
        }
    }

    #[test]
    fn keeps_required_fields_only() {
        let client: Client = serde_json::from_value(serde_json::json!({
            "package": {"name": "x", "repo": ""}, "version": "", "description": "", "base_url": "",
            "settings": null, "casings": [], "resources": [], "client_methods": [], "models": [], "style": []
        }))
        .expect("a client");
        let ty = Type::Object(Object {
            fields: vec![
                field("id", true, Type::scalar(Scalar::Int)),
                field("when", true, Type::scalar(Scalar::DateTime)),
                field("next", false, Type::scalar(Scalar::String)),
                field(
                    "tags",
                    true,
                    Type::Array {
                        items: Box::new(Type::scalar(Scalar::String)),
                    },
                ),
            ],
            ..Object::default()
        });
        assert_eq!(
            value(&client, &ty),
            serde_json::json!({"id": 0, "when": "2019-12-27T18:11:19.117Z", "tags": []})
        );
    }
}
