//! Untyped JSON readers shared by the extension builders.

use crate::value::{Map, Value};
use std::collections::HashSet;
use std::sync::LazyLock;

static EMPTY: LazyLock<Map> = LazyLock::new(Map::new);

/// An empty object.
#[must_use]
pub fn empty() -> &'static Map {
    &EMPTY
}

/// Member `k` when it is an object, else an empty object.
#[must_use]
pub fn obj<'a>(o: &'a Map, k: &str) -> &'a Map {
    match o.get(k) {
        Some(Value::Object(m)) => m,
        _ => &EMPTY,
    }
}

/// Member `k` when it is an object, of any value.
#[must_use]
pub fn obj_of(v: Option<&Value>) -> &Map {
    match v {
        Some(Value::Object(m)) => m,
        _ => &EMPTY,
    }
}

/// Member `k` when it is an array, else empty.
#[must_use]
pub fn arr<'a>(o: &'a Map, k: &str) -> &'a [Value] {
    match o.get(k) {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

/// Member `k` when it is a string.
#[must_use]
pub fn str_<'a>(o: &'a Map, k: &str) -> Option<&'a str> {
    o.get(k).and_then(Value::as_str)
}

/// Member `k` when it is a finite number.
#[must_use]
pub fn num(o: &Map, k: &str) -> Option<f64> {
    o.get(k).and_then(Value::as_f64).filter(|n| n.is_finite())
}

/// Member `k` when it is a boolean.
#[must_use]
pub fn bool_(o: &Map, k: &str) -> Option<bool> {
    o.get(k).and_then(Value::as_bool)
}

/// Resolve a local `#/...` $ref chain (cycle-safe); other nodes are returned as is. `None` is
/// `undefined`.
#[must_use]
pub fn deref<'a>(spec: &'a Value, node: Option<&'a Value>) -> Option<&'a Value> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut node = node;
    loop {
        let Some(Value::Object(m)) = node else { return node };
        let Some(r) = str_(m, "$ref") else { return node };
        if !r.starts_with("#/") || !seen.insert(r.to_owned()) {
            return None;
        }
        let mut cur: Option<&Value> = Some(spec);
        for raw in r[2..].split('/') {
            cur = match cur {
                Some(Value::Object(o)) => o.get(&raw.replace("~1", "/").replace("~0", "~")),
                _ => None,
            };
        }
        node = cur;
    }
}

/// The schema of `name` among a schema's properties, following $ref and allOf.
#[must_use]
pub fn property_schema<'a>(spec: &'a Value, schema: Option<&'a Value>, name: &str, depth: usize) -> Option<&'a Value> {
    let Some(Value::Object(s)) = deref(spec, schema) else {
        return None;
    };
    if depth > 32 {
        return None;
    }
    if let Some(p) = obj(s, "properties").get(name) {
        return Some(p);
    }
    arr(s, "allOf")
        .iter()
        .find_map(|part| property_schema(spec, Some(part), name, depth + 1))
}

/// Property names of a schema, following $ref and allOf, in first-seen order.
#[must_use]
pub fn property_names(spec: &Value, schema: Option<&Value>, depth: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(Value::Object(s)) = deref(spec, schema) else {
        return out;
    };
    if depth > 32 {
        return out;
    }
    for k in obj(s, "properties").keys() {
        if !out.contains(k) {
            out.push(k.clone());
        }
    }
    for part in arr(s, "allOf") {
        for k in property_names(spec, Some(part), depth + 1) {
            if !out.contains(&k) {
                out.push(k);
            }
        }
    }
    out
}

/// Whether a schema is an array, following $ref and allOf.
#[must_use]
pub fn is_array_schema(spec: &Value, schema: Option<&Value>, depth: usize) -> bool {
    let Some(Value::Object(s)) = deref(spec, schema) else {
        return false;
    };
    if depth > 32 {
        return false;
    }
    match s.get("type") {
        Some(Value::String(t)) if t == "array" => return true,
        Some(Value::Array(a)) if a.iter().any(|t| t.as_str() == Some("array")) => return true,
        _ => {}
    }
    if s.contains_key("items") {
        return true;
    }
    arr(s, "allOf")
        .iter()
        .any(|part| is_array_schema(spec, Some(part), depth + 1))
}

/// The element schema of an array schema, following $ref and allOf.
#[must_use]
pub fn items_schema<'a>(spec: &'a Value, schema: Option<&'a Value>, depth: usize) -> Option<&'a Value> {
    let Some(Value::Object(s)) = deref(spec, schema) else {
        return None;
    };
    if depth > 32 {
        return None;
    }
    if let Some(items) = s.get("items") {
        return Some(items);
    }
    arr(s, "allOf")
        .iter()
        .find_map(|part| items_schema(spec, Some(part), depth + 1))
}

/// The lowest 2xx response of an operation, dereferenced.
#[must_use]
pub fn success_response<'a>(spec: &'a Value, op: &'a Map) -> Option<&'a Map> {
    let responses = obj(op, "responses");
    let mut codes: Vec<&String> = responses.keys().filter(|c| c.starts_with('2')).collect();
    codes.sort_by(|a, b| crate::value::cmp_utf16(a, b));
    let code = codes.first()?;
    deref(spec, responses.get(code.as_str())).and_then(Value::as_object)
}

/// The `application/json` schema of a request body's or response's `content`.
#[must_use]
pub fn json_schema_of(content: &Map) -> Option<&Value> {
    obj(content, "application/json").get("schema")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec() -> Value {
        Value::from(json!({
            "components": {
                "schemas": {
                    "Base": {"properties": {"id": {"type": "string"}}},
                    "Page": {"allOf": [{"$ref": "#/components/schemas/Base"}, {"properties": {"data": {"type": "array"}}}]},
                    "List": {"allOf": [{"items": {"type": "string"}}]},
                    "Self": {"$ref": "#/components/schemas/Self"},
                    "a/b": {"type": "string"},
                    "c~d": {"type": "integer"}
                }
            }
        }))
    }

    fn r(name: &str) -> Value {
        Value::from(json!({"$ref": format!("#/components/schemas/{name}")}))
    }

    #[test]
    fn deref_follows_escaped_and_cyclic_refs() {
        let s = spec();
        let ab = r("a~1b");
        assert_eq!(deref(&s, Some(&ab)), Some(&Value::from(json!({"type": "string"}))));
        let cd = r("c~0d");
        assert_eq!(deref(&s, Some(&cd)), Some(&Value::from(json!({"type": "integer"}))));
        let me = r("Self");
        assert_eq!(deref(&s, Some(&me)), None);
        let ext = Value::from(json!({"$ref": "other.yaml#/x"}));
        assert_eq!(deref(&s, Some(&ext)), None);
        let plain = Value::from(json!({"type": "x"}));
        assert_eq!(deref(&s, Some(&plain)), Some(&plain));
        assert_eq!(deref(&s, None), None);
    }

    #[test]
    fn property_helpers_follow_refs_and_all_of() {
        let s = spec();
        let page = r("Page");
        assert_eq!(property_names(&s, Some(&page), 0), ["id", "data"]);
        assert_eq!(
            property_schema(&s, Some(&page), "id", 0),
            Some(&Value::from(json!({"type": "string"})))
        );
        assert_eq!(property_schema(&s, Some(&page), "missing", 0), None);
        let data = property_schema(&s, Some(&page), "data", 0);
        assert!(is_array_schema(&s, data, 0));
        let list = r("List");
        assert!(is_array_schema(&s, Some(&list), 0));
        assert_eq!(
            items_schema(&s, Some(&list), 0),
            Some(&Value::from(json!({"type": "string"})))
        );
        let me = r("Self");
        assert_eq!(property_names(&s, Some(&me), 0), Vec::<String>::new());
        assert!(!is_array_schema(&s, Some(&me), 0));
        assert_eq!(items_schema(&s, Some(&me), 0), None);
        let typed = Value::from(json!({"type": ["array", "null"]}));
        assert!(is_array_schema(&s, Some(&typed), 0));
    }

    #[test]
    fn success_response_picks_the_lowest_2xx() {
        let s = Value::from(json!({"components": {"responses": {"R": {"description": "r"}}}}));
        let op = Value::from(
            json!({"responses": {"404": {}, "201": {"$ref": "#/components/responses/R"}, "200": {"description": "ok"}}}),
        );
        let got = success_response(&s, op.as_object().unwrap());
        assert_eq!(got.and_then(|m| m.get("description")), Some(&Value::from("ok")));
        let none = Value::from(json!({"responses": {"404": {}}}));
        assert!(success_response(&s, none.as_object().unwrap()).is_none());
    }
}
