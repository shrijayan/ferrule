//! `x-ferrule-streaming` (ADR 0003): a method's `streaming:` block in stainless.yml, with the
//! top-level `streaming.on_event` handlers. Port of `converter/src/extensions/streaming.ts`.

use super::json::{arr, bool_, deref, obj, obj_of, property_names, str_, success_response};
use crate::Diagnostic;
use crate::value::{Map, Value};

const JSONL_TYPES: [&str; 4] = [
    "application/x-ndjson",
    "application/jsonl",
    "application/jsonlines",
    "application/x-jsonlines",
];
const HANDLES: [&str; 5] = ["break", "continue", "done", "yield", "error"];

/// `type`, else inferred from the 2xx content types, else `sse` (public schema default).
fn format(spec: &Value, op: &Map, written: Option<&Value>) -> &'static str {
    match written.and_then(Value::as_str) {
        Some("sse") => return "sse",
        Some("jsonl") => return "jsonl",
        _ => {}
    }
    let resp = success_response(spec, op);
    let content = resp.map(|r| obj(r, "content"));
    let types: Vec<&String> = content.map(|c| c.keys().collect()).unwrap_or_default();
    if types.iter().any(|t| *t == "text/event-stream") {
        return "sse";
    }
    if types.iter().any(|t| JSONL_TYPES.contains(&t.as_str())) {
        "jsonl"
    } else {
        "sse"
    }
}

/// Read top-level `streaming.on_event`; entries with an unknown handle are dropped.
#[must_use]
pub fn read_on_event(config: &Value) -> Vec<Value> {
    let config = obj_of(Some(config));
    arr(obj(config, "streaming"), "on_event")
        .iter()
        .filter_map(|raw| {
            let e = obj_of(Some(raw));
            let handle = e
                .get("handle")
                .and_then(Value::as_str)
                .filter(|h| HANDLES.contains(h))?;
            let data_starts_with = str_(e, "data_starts_with");
            let error_property = str_(e, "error_property");
            let kind = match str_(e, "kind") {
                Some(k @ ("fallthrough" | "data" | "event")) => k,
                _ => {
                    if data_starts_with.is_none() {
                        "event"
                    } else {
                        "data"
                    }
                }
            };
            let mut out = Map::new();
            out.insert("kind", Value::from(kind));
            out.insert("handle", Value::from(handle));
            if kind == "event" {
                let event_type = match e.get("event_type") {
                    Some(v @ (Value::String(_) | Value::Null)) => v.clone(),
                    _ => Value::Array(
                        arr(e, "event_type")
                            .iter()
                            .filter(|t| t.as_str().is_some())
                            .cloned()
                            .collect(),
                    ),
                };
                out.insert("eventType", event_type);
            }
            if let Some(p) = error_property {
                out.insert("errorProperty", Value::from(p));
            }
            if let Some(d) = data_starts_with {
                out.insert("dataStartsWith", Value::from(d));
            }
            Some(Value::Object(out))
        })
        .collect()
}

fn strip_schema_prefix(s: &str) -> String {
    s.strip_prefix("#/components/schemas/").unwrap_or(s).to_owned()
}

/// Schema name of `$resource.$model` (sub-resources joined by dots), or `None`.
#[must_use]
pub fn resolve_event_model(config: &Value, written: &str) -> Option<String> {
    let config = obj_of(Some(config));
    let mut segs: Vec<&str> = written.split('.').collect();
    let key = segs.pop()?;
    let mut resources = obj(config, "resources");
    let mut resource: Option<&Map> = None;
    for (i, seg) in segs.iter().enumerate() {
        resource = resources.get(seg).and_then(Value::as_object);
        let r = resource?;
        if i < segs.len() - 1 {
            resources = obj(r, "subresources");
        }
    }
    let model = obj(resource?, "models").get(key)?;
    model.as_str().map(strip_schema_prefix)
}

/// What [`build_streaming`] reads.
pub struct StreamingInput<'a> {
    pub spec: &'a Value,
    pub config: &'a Value,
    pub op: &'a Map,
    /// The method's `streaming` value.
    pub streaming: Option<&'a Value>,
    /// JSON pointer of the method in the config.
    pub pointer: &'a str,
    pub on_event: &'a [Value],
}

/// Build `x-ferrule-streaming`, or `None` when not configured or skipped for Python.
#[must_use]
pub fn build_streaming(input: &StreamingInput<'_>) -> (Option<Value>, Vec<Diagnostic>) {
    let StreamingInput {
        spec,
        config,
        op,
        pointer,
        on_event,
        ..
    } = *input;
    let mut diagnostics = Vec::new();
    let Some(Value::Object(s)) = input.streaming else {
        return (None, diagnostics);
    };
    if arr(s, "skip").iter().any(|v| v.as_str() == Some("python")) {
        return (None, diagnostics);
    }
    let param_discriminator = match s.get("param_discriminator") {
        Some(Value::String(p)) => Value::from(p.as_str()),
        _ => Value::Null,
    };
    if !s.contains_key("param_discriminator") {
        diagnostics.push(Diagnostic::warning(
            "FER1401",
            format!("{pointer}/streaming"),
            "streaming has no param_discriminator; the method is treated as always streaming.",
            "Set param_discriminator to the request param that enables streaming, or null.",
        ));
    } else if let Value::String(p) = &param_discriminator {
        let body = deref(spec, op.get("requestBody"));
        let body = obj_of(body);
        let schema = obj(obj(body, "content"), "application/json").get("schema");
        if !property_names(spec, schema, 0).iter().any(|n| n == p) {
            diagnostics.push(Diagnostic::warning(
                "FER1402",
                format!("{pointer}/streaming/param_discriminator"),
                format!("param_discriminator {p} is not a JSON request body property."),
                "Name a boolean body property such as `stream`, or use null.",
            ));
        }
    }

    let event_model = str_(s, "stream_event_model");
    let event_schema = event_model.and_then(|m| resolve_event_model(config, m));
    if let (Some(m), None) = (event_model, &event_schema) {
        diagnostics.push(Diagnostic::warning(
            "FER1403",
            format!("{pointer}/streaming/stream_event_model"),
            format!("stream_event_model {m} is not a model of a configured resource."),
            "Write it as `$resource.$model`, naming a key under that resource's models.",
        ));
    }
    let mut out = Map::new();
    out.insert("format", Value::from(format(spec, op, s.get("type"))));
    out.insert("paramDiscriminator", param_discriminator);
    if let Some(m) = event_model {
        out.insert("eventModel", Value::from(m));
    }
    if let Some(e) = event_schema {
        out.insert("eventSchema", Value::from(e));
    }
    if let Some(p) = str_(s, "params_type_name") {
        out.insert("paramsTypeName", Value::from(p));
    }
    if let Some(m) = str_(s, "method_name") {
        out.insert("methodName", Value::from(m));
    }
    out.insert(
        "synthesizeEventAndData",
        Value::Bool(bool_(s, "synthesize_event_and_data_properties").unwrap_or(false)),
    );
    out.insert("onEvent", Value::Array(on_event.to_vec()));
    (Some(Value::Object(out)), diagnostics)
}
