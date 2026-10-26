//! What the TypeScript output needs from the OpenAPI document that the neutral model does not
//! carry yet, read from the document itself (plain JSON, never Forge types):
//!
//! - examples of parameters and body properties (`example`, `examples[0]`), which Stainless
//!   uses in `@example` blocks, tests and the README (Forge gap G18);
//! - the description of an operation's first tag, which documents its resource class;
//! - the order properties are written in, which the TypeScript SDK keeps (the annotated spec is
//!   written with sorted keys, and the model lists fields in that order).
//!
//! Every lookup is optional: without a document the emitters fall back to the model's order and
//! to examples derived from types. Pure: the caller reads the document.

use std::collections::HashMap;

use serde_json::Value;

/// Lookups into one OpenAPI document.
#[derive(Debug, Clone, Default)]
pub struct SpecExtras {
    doc: Value,
    /// Sorted property names -> the order the first schema with exactly those properties
    /// writes them in, and that schema's JSON pointer.
    orders: HashMap<Vec<String>, (Vec<String>, String)>,
}

/// `#/a/b~1c` -> the value at that JSON pointer.
fn pointer<'a>(doc: &'a Value, ptr: &str) -> Option<&'a Value> {
    let path = ptr.strip_prefix('#').unwrap_or(ptr);
    if path.is_empty() {
        return Some(doc);
    }
    doc.pointer(path)
}

impl SpecExtras {
    /// Indexes `doc`, an OpenAPI document as JSON with its keys in written order.
    #[must_use]
    pub fn new(doc: Value) -> Self {
        let mut extras = Self {
            doc,
            orders: HashMap::new(),
        };
        let mut found: Vec<(Vec<String>, String)> = Vec::new();
        collect_orders(&extras, &extras.doc, &mut String::new(), &mut found, 0);
        for (order, ptr) in found {
            let mut key = order.clone();
            key.sort();
            extras.orders.entry(key).or_insert((order, ptr));
        }
        extras
    }

    /// No document: every lookup misses.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Follows `$ref`s (at most 32) to the schema they name.
    #[must_use]
    pub fn resolve<'a>(&'a self, mut schema: &'a Value) -> &'a Value {
        for _ in 0..32 {
            match schema.get("$ref").and_then(Value::as_str) {
                Some(r) => match pointer(&self.doc, r) {
                    Some(target) => schema = target,
                    None => return schema,
                },
                None => return schema,
            }
        }
        schema
    }

    /// The operation at a JSON pointer such as `#/paths/~1embeddings/post`.
    #[must_use]
    pub fn operation(&self, op: &str) -> Option<&Value> {
        pointer(&self.doc, op)
    }

    /// The description of the operation's first tag.
    #[must_use]
    pub fn tag_description(&self, op: &str) -> Option<String> {
        let tag = self.operation(op)?.get("tags")?.get(0)?.as_str()?;
        let tags = self.doc.get("tags")?.as_array()?;
        tags.iter()
            .find(|t| t.get("name").and_then(Value::as_str) == Some(tag))?
            .get("description")?
            .as_str()
            .map(str::to_owned)
    }

    /// The JSON request body schema of an operation.
    #[must_use]
    pub fn body_schema(&self, op: &str) -> Option<&Value> {
        let body = self.resolve(self.operation(op)?.get("requestBody")?);
        let content = body.get("content")?.as_object()?;
        let media = content.get("application/json").or_else(|| content.values().next())?;
        media.get("schema").map(|s| self.resolve(s))
    }

    /// The JSON request body's example: the media type's `example` (or first `examples` value),
    /// else the schema's.
    #[must_use]
    pub fn body_example(&self, op: &str) -> Option<Value> {
        let body = self.resolve(self.operation(op)?.get("requestBody")?);
        let content = body.get("content")?.as_object()?;
        let media = content.get("application/json").or_else(|| content.values().next())?;
        media
            .get("example")
            .cloned()
            .or_else(|| {
                media
                    .get("examples")?
                    .as_object()?
                    .values()
                    .next()
                    .map(|e| self.resolve(e))?
                    .get("value")
                    .cloned()
            })
            .or_else(|| self.example(media.get("schema")?))
    }

    /// The schema of the first 2xx JSON response of an operation.
    #[must_use]
    pub fn response_schema(&self, op: &str) -> Option<&Value> {
        let responses = self.operation(op)?.get("responses")?.as_object()?;
        let (_, response) = responses.iter().find(|(status, _)| status.starts_with('2'))?;
        let response = self.resolve(response);
        let media = response.get("content")?.get("application/json")?;
        media.get("schema").map(|s| self.resolve(s))
    }

    /// A parameter of an operation (or of its path item), by `in` and name.
    #[must_use]
    pub fn parameter(&self, op: &str, location: &str, name: &str) -> Option<&Value> {
        let operation = self.operation(op)?;
        let item_ptr = op.rsplit_once('/').map_or(op, |(item, _)| item);
        let item = pointer(&self.doc, item_ptr);
        let lists = [operation.get("parameters"), item.and_then(|i| i.get("parameters"))];
        lists
            .into_iter()
            .flatten()
            .filter_map(Value::as_array)
            .flatten()
            .map(|p| self.resolve(p))
            .find(|p| {
                p.get("in").and_then(Value::as_str) == Some(location)
                    && p.get("name").and_then(Value::as_str) == Some(name)
            })
    }

    /// A parameter's example: its own, else its schema's.
    #[must_use]
    pub fn parameter_example(&self, param: &Value) -> Option<Value> {
        example(param).or_else(|| param.get("schema").and_then(|s| self.example(s)))
    }

    /// A schema's own example: `example`, else `examples[0]`.
    #[must_use]
    pub fn example(&self, schema: &Value) -> Option<Value> {
        example(schema).or_else(|| example(self.resolve(schema)))
    }

    /// The schemas a value of `schema` may match: itself, its `allOf` members, and its `oneOf`
    /// and `anyOf` variants, refs resolved, depth first.
    fn alternatives<'a>(&'a self, schema: &'a Value, out: &mut Vec<&'a Value>, depth: usize) {
        if depth > 16 {
            return;
        }
        let schema = self.resolve(schema);
        out.push(schema);
        for key in ["allOf", "oneOf", "anyOf"] {
            if let Some(items) = schema.get(key).and_then(Value::as_array) {
                for item in items {
                    self.alternatives(item, out, depth + 1);
                }
            }
        }
    }

    /// The schema of property `name` of `schema`, looking through `allOf`, `oneOf` and `anyOf`.
    #[must_use]
    pub fn property<'a>(&'a self, schema: &'a Value, name: &str) -> Option<&'a Value> {
        let mut alts = Vec::new();
        self.alternatives(schema, &mut alts, 0);
        alts.into_iter().find_map(|s| s.get("properties")?.get(name))
    }

    /// The item schema of an array schema (or of its first array alternative).
    #[must_use]
    pub fn items<'a>(&'a self, schema: &'a Value) -> Option<&'a Value> {
        let mut alts = Vec::new();
        self.alternatives(schema, &mut alts, 0);
        alts.into_iter().find_map(|s| s.get("items"))
    }

    /// The written order of an object's properties, given their names in any order.
    #[must_use]
    pub fn property_order(&self, names: &[&str]) -> Option<&[String]> {
        let mut key: Vec<String> = names.iter().map(|&n| n.to_owned()).collect();
        key.sort();
        self.orders.get(&key).map(|(order, _)| order.as_slice())
    }

    /// The first schema whose properties are exactly `names`.
    #[must_use]
    pub fn object_schema(&self, names: &[&str]) -> Option<&Value> {
        let mut key: Vec<String> = names.iter().map(|&n| n.to_owned()).collect();
        key.sort();
        let (_, ptr) = self.orders.get(&key)?;
        self.doc.pointer(ptr)
    }

    /// `components.schemas[name]`.
    #[must_use]
    pub fn component(&self, name: &str) -> Option<&Value> {
        self.doc.get("components")?.get("schemas")?.get(name)
    }

    /// Whether `description` only repeats the schema's `title` (Forge's description falls back to
    /// the title; Stainless documents a type from its `description` alone).
    #[must_use]
    pub fn is_title_only(&self, schema: &Value, description: &str) -> bool {
        let schema = self.resolve(schema);
        schema.get("description").is_none() && schema.get("title").and_then(Value::as_str) == Some(description)
    }

    /// The description Stainless documents a property with, when it differs from the property's
    /// own: a `oneOf`/`anyOf` of one schema and `null` takes that schema's description.
    #[must_use]
    pub fn property_description(&self, property: &Value) -> Option<String> {
        let variants = property
            .get("oneOf")
            .or_else(|| property.get("anyOf"))
            .and_then(Value::as_array)?;
        let non_null: Vec<&Value> = variants
            .iter()
            .map(|v| self.resolve(v))
            .filter(|v| v.get("type").and_then(Value::as_str) != Some("null"))
            .collect();
        match non_null.as_slice() {
            [only] if variants.len() == 2 => only.get("description").and_then(Value::as_str).map(str::to_owned),
            _ => None,
        }
    }

    /// The properties of `schema` in written order, `allOf` members merged.
    fn merged_properties(&self, schema: &Value, out: &mut Vec<String>, depth: usize) {
        if depth > 16 {
            return;
        }
        let schema = self.resolve(schema);
        if let Some(members) = schema.get("allOf").and_then(Value::as_array) {
            for m in members {
                self.merged_properties(m, out, depth + 1);
            }
        }
        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for k in props.keys() {
                if !out.contains(k) {
                    out.push(k.clone());
                }
            }
        }
    }
}

fn example(v: &Value) -> Option<Value> {
    v.get("example")
        .cloned()
        .or_else(|| v.get("examples")?.as_array()?.first().cloned())
}

/// `~` and `/` escaped for a JSON pointer segment.
fn escape(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// Every schema with properties (or an `allOf` of them), anywhere in the document, with its
/// pointer.
fn collect_orders(
    extras: &SpecExtras,
    v: &Value,
    path: &mut String,
    out: &mut Vec<(Vec<String>, String)>,
    depth: usize,
) {
    if depth > 64 {
        return;
    }
    let len = path.len();
    match v {
        Value::Object(map) => {
            let has_props = map.get("properties").is_some_and(Value::is_object);
            let has_all_of = map.get("allOf").is_some_and(Value::is_array);
            if has_props || has_all_of {
                let mut props = Vec::new();
                extras.merged_properties(v, &mut props, 0);
                if !props.is_empty() {
                    out.push((props, path.clone()));
                }
            }
            for (k, child) in map {
                if k != "example" && k != "examples" {
                    path.push('/');
                    path.push_str(&escape(k));
                    collect_orders(extras, child, path, out, depth + 1);
                    path.truncate(len);
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                path.push('/');
                path.push_str(&i.to_string());
                collect_orders(extras, item, path, out, depth + 1);
                path.truncate(len);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc() -> Value {
        json!({
            "paths": {"/things": {"post": {
                "tags": ["Things"],
                "parameters": [{"in": "query", "name": "limit", "schema": {"type": "integer", "example": 5}}],
                "requestBody": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Req"}}}}
            }}},
            "tags": [{"name": "Things", "description": "Make things."}],
            "components": {"schemas": {
                "Req": {"properties": {"name": {"type": "string", "examples": ["n"]}, "opts": {"oneOf": [{"$ref": "#/components/schemas/Opts"}, {"type": "null"}]}}},
                "Opts": {"properties": {"z": {"type": "integer"}, "a": {"type": "integer", "example": 3}}}
            }}
        })
    }

    #[test]
    fn finds_examples_tags_and_order() {
        let x = SpecExtras::new(doc());
        let op = "#/paths/~1things/post";
        assert_eq!(x.tag_description(op).as_deref(), Some("Make things."));
        let body = x.body_schema(op).expect("body");
        assert_eq!(x.property(body, "name").and_then(|s| x.example(s)), Some(json!("n")));
        let opts = x.property(body, "opts").expect("opts");
        assert_eq!(x.property(opts, "a").and_then(|s| x.example(s)), Some(json!(3)));
        let limit = x.parameter(op, "query", "limit").expect("limit");
        assert_eq!(x.parameter_example(limit), Some(json!(5)));
        assert_eq!(
            x.property_order(&["a", "z"]),
            Some(&["z".to_owned(), "a".to_owned()][..])
        );
    }
}
