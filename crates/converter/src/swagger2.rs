//! Swagger 2.0 to OpenAPI 3.0 (`converter/src/swagger2/index.ts`): the pinned Forge reader knows
//! only OpenAPI 3. Runs before `openapi.transforms`. Pure. Body and form parameters become
//! request bodies noted with `x-ferrule-normalized`; the document gets one FER1610 info
//! diagnostic. See docs/forge-gaps.md (G24).

use crate::Diagnostic;
use crate::normalize::NORMALIZED_KEY;
use crate::value::{Map, Value, js_string};

pub const SWAGGER2_CODE: &str = "FER1610";

const REFS: [(&str, &str); 3] = [
    ("#/definitions/", "#/components/schemas/"),
    ("#/parameters/", "#/components/parameters/"),
    ("#/responses/", "#/components/responses/"),
];

/// Keywords of a non-body Swagger 2 parameter that move into its OpenAPI 3 `schema`.
const SCHEMA_KEYS: [&str; 16] = [
    "type",
    "format",
    "items",
    "default",
    "maximum",
    "exclusiveMaximum",
    "minimum",
    "exclusiveMinimum",
    "maxLength",
    "minLength",
    "pattern",
    "maxItems",
    "minItems",
    "uniqueItems",
    "enum",
    "multipleOf",
];

const METHODS: [&str; 7] = ["get", "put", "post", "delete", "options", "head", "patch"];

fn collection_format(v: &Value) -> Option<(&'static str, bool)> {
    match js_string(v).as_str() {
        "csv" => Some(("form", false)),
        "ssv" => Some(("spaceDelimited", false)),
        "pipes" => Some(("pipeDelimited", false)),
        "multi" => Some(("form", true)),
        _ => None,
    }
}

/// Rewrite refs, `x-nullable`, `type: file` and string discriminators throughout a schema.
fn schema(value: &Value) -> Value {
    match value {
        Value::Array(a) => Value::Array(a.iter().map(schema).collect()),
        Value::Object(m) => {
            let mut out = Map::new();
            for (k, v) in m {
                match (k.as_str(), v) {
                    ("$ref", Value::String(r)) => {
                        let rewritten = REFS
                            .iter()
                            .find(|(from, _)| r.starts_with(from))
                            .map_or_else(|| r.clone(), |(from, to)| format!("{to}{}", &r[from.len()..]));
                        out.insert(k.clone(), Value::String(rewritten));
                    }
                    ("x-nullable", _) => {
                        out.insert("nullable", v.clone());
                    }
                    ("discriminator", Value::String(d)) => {
                        let mut p = Map::new();
                        p.insert("propertyName", Value::from(d.as_str()));
                        out.insert(k.clone(), Value::Object(p));
                    }
                    ("properties" | "patternProperties", Value::Object(props)) => {
                        out.insert(k.clone(), Value::Object(named(props)));
                    }
                    _ if k == "example" || k == "enum" || k == "default" || k.starts_with("x-") => {
                        out.insert(k.clone(), v.clone());
                    }
                    _ => {
                        out.insert(k.clone(), schema(v));
                    }
                }
            }
            if out.get("type").and_then(Value::as_str) == Some("file") {
                out.insert("type", Value::from("string"));
                out.insert("format", Value::from("binary"));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// A map from names to schemas; a name such as `default` is not a keyword there.
fn named(map: &Map) -> Map {
    map.iter().map(|(n, v)| (n.clone(), schema(v))).collect()
}

/// A non-body parameter (or a header object): its type keywords move into `schema`.
fn simple_param(p: &Map) -> Map {
    let mut out = Map::new();
    let mut inner = Map::new();
    let in_query = p.get("in").and_then(Value::as_str) == Some("query");
    for (k, v) in p {
        if SCHEMA_KEYS.contains(&k.as_str()) {
            inner.insert(k.clone(), v.clone());
        } else if k == "collectionFormat" {
            if let Some((style, explode)) = collection_format(v) {
                out.insert("style", Value::from(style));
                out.insert("explode", Value::Bool(explode));
            }
        } else if k == "x-nullable" {
            inner.insert("nullable", v.clone());
        } else if k != "allowEmptyValue" || in_query {
            out.insert(k.clone(), v.clone());
        }
    }
    if !inner.is_empty() {
        out.insert("schema", schema(&Value::Object(inner)));
    }
    out
}

/// Form field schema of a `formData` parameter: its type keywords plus its description.
fn form_field(p: &Map) -> Value {
    let simple = simple_param(p);
    let mut field = match simple.get("schema") {
        Some(Value::Object(s)) => s.clone(),
        _ => Map::new(),
    };
    if let Some(Value::String(d)) = p.get("description") {
        field.insert("description", Value::from(d.as_str()));
    }
    Value::Object(field)
}

/// `{ [type]: { schema } }` for each media type; an undefined schema is left out, as
/// `JSON.stringify` drops it.
fn content(types: &[String], s: Option<&Value>) -> Value {
    let mut out = Map::new();
    for t in types {
        let mut media = Map::new();
        if let Some(s) = s {
            media.insert("schema", s.clone());
        }
        out.insert(t.clone(), Value::Object(media));
    }
    Value::Object(out)
}

fn strings(v: Option<&Value>) -> Option<Vec<String>> {
    match v {
        Some(Value::Array(a)) if !a.is_empty() => Some(a.iter().map(js_string).collect()),
        _ => None,
    }
}

fn is_body_or_form(p: &Value) -> bool {
    matches!(p.get("in").and_then(Value::as_str), Some("body" | "formData")) && p.is_object()
}

fn in_is(p: &Value, what: &str) -> bool {
    p.is_object() && p.get("in").and_then(Value::as_str) == Some(what)
}

/// `String(p.in) + ":" + String(p.name)`, with JavaScript's `undefined`.
fn param_key(p: &Map) -> String {
    let s = |v: Option<&Value>| v.map_or_else(|| "undefined".to_owned(), js_string);
    format!("{}:{}", s(p.get("in")), s(p.get("name")))
}

struct Converter<'a> {
    root_consumes: Vec<String>,
    root_produces: Vec<String>,
    global_params: &'a Map,
}

impl Converter<'_> {
    /// Body and formData parameters cannot be components in OpenAPI 3, so their refs are inlined.
    fn resolve(&self, p: &Value) -> Value {
        if let Some(Value::String(r)) = p.get("$ref")
            && let Some(name) = r.strip_prefix("#/parameters/")
            && let Some(target) = self.global_params.get(name)
            && is_body_or_form(target)
        {
            return target.clone();
        }
        p.clone()
    }

    fn response(r: &Value, produces: &[String]) -> Value {
        let Value::Object(m) = r else { return schema(r) };
        if matches!(m.get("$ref"), Some(Value::String(_))) {
            return schema(r);
        }
        let mut out = Map::new();
        for (k, v) in m {
            if k == "schema" {
                out.insert("content", content(produces, Some(&schema(v))));
            } else if let ("headers", Value::Object(h)) = (k.as_str(), v) {
                let headers: Map = h
                    .iter()
                    .map(|(n, hv)| {
                        let converted = match hv {
                            Value::Object(ho) => Value::Object(simple_param(ho)),
                            other => other.clone(),
                        };
                        (n.clone(), converted)
                    })
                    .collect();
                out.insert("headers", Value::Object(headers));
            } else if k != "examples" {
                out.insert(k.clone(), v.clone());
            }
        }
        Value::Object(out)
    }

    fn param(p: &Value) -> Value {
        match p {
            Value::Object(m) if !matches!(m.get("$ref"), Some(Value::String(_))) => Value::Object(simple_param(m)),
            other => schema(other),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "mirrors operation() of the TypeScript source step by step"
    )]
    fn operation(&self, op: &Map, path_params: &[Value]) -> Map {
        let consumes = strings(op.get("consumes")).unwrap_or_else(|| self.root_consumes.clone());
        let produces = strings(op.get("produces")).unwrap_or_else(|| self.root_produces.clone());
        let own: Vec<Value> = match op.get("parameters") {
            Some(Value::Array(a)) => a.iter().map(|p| self.resolve(p)).collect(),
            _ => Vec::new(),
        };
        let own_keys: Vec<String> = own.iter().filter_map(Value::as_object).map(param_key).collect();
        // Path-level body and form parameters belong to each operation's request body.
        let inherited: Vec<Value> = path_params
            .iter()
            .map(|p| self.resolve(p))
            .filter(is_body_or_form)
            .filter(|p| !own_keys.contains(&param_key(p.as_object().unwrap_or(crate::extensions::json::empty()))))
            .collect();
        let all: Vec<&Value> = inherited.iter().chain(own.iter()).collect();
        let body = all.iter().find(|p| in_is(p, "body")).and_then(|p| p.as_object());
        let form: Vec<&Map> = all
            .iter()
            .filter(|p| in_is(p, "formData"))
            .filter_map(|p| p.as_object())
            .collect();
        let rest: Vec<&Value> = own.iter().filter(|p| !is_body_or_form(p)).collect();

        let mut out = Map::new();
        for (k, v) in op {
            match k.as_str() {
                "consumes" | "produces" => {}
                "parameters" => {
                    if !rest.is_empty() {
                        out.insert(k.clone(), Value::Array(rest.iter().map(|p| Self::param(p)).collect()));
                    }
                }
                "responses" if v.is_object() => {
                    let responses: Map = v
                        .as_object()
                        .into_iter()
                        .flatten()
                        .map(|(c, r)| (c.clone(), Self::response(r, &produces)))
                        .collect();
                    out.insert(k.clone(), Value::Object(responses));
                }
                _ if k.starts_with("x-") => {
                    out.insert(k.clone(), v.clone());
                }
                _ => {
                    out.insert(k.clone(), schema(v));
                }
            }
        }
        if let Some(body) = body {
            let types: Vec<String> = consumes.iter().filter(|t| !t.contains("form")).cloned().collect();
            let types = if types.is_empty() {
                vec!["application/json".to_owned()]
            } else {
                types
            };
            let mut rb = Map::new();
            if let Some(Value::String(d)) = body.get("description") {
                rb.insert("description", Value::from(d.as_str()));
            }
            if body.get("required") == Some(&Value::Bool(true)) {
                rb.insert("required", Value::Bool(true));
            }
            rb.insert("content", content(&types, body.get("schema").map(schema).as_ref()));
            rb.insert(NORMALIZED_KEY, Value::Array(vec![Value::from("swagger2-body")]));
            out.insert("requestBody", Value::Object(rb));
        } else if !form.is_empty() {
            let multipart = form
                .iter()
                .any(|p| p.get("type").and_then(Value::as_str) == Some("file"))
                || !consumes.iter().any(|c| c == "application/x-www-form-urlencoded");
            let required: Vec<Value> = form
                .iter()
                .filter(|p| p.get("required") == Some(&Value::Bool(true)))
                .map(|p| Value::String(p.get("name").map_or_else(|| "undefined".to_owned(), js_string)))
                .collect();
            let mut rb = Map::new();
            if !required.is_empty() {
                rb.insert("required", Value::Bool(true));
            }
            let mut s = Map::new();
            s.insert("type", Value::from("object"));
            if !required.is_empty() {
                s.insert("required", Value::Array(required));
            }
            let properties: Map = form
                .iter()
                .map(|p| {
                    (
                        p.get("name").map_or_else(|| "undefined".to_owned(), js_string),
                        form_field(p),
                    )
                })
                .collect();
            s.insert("properties", Value::Object(properties));
            let media = if multipart {
                "multipart/form-data"
            } else {
                "application/x-www-form-urlencoded"
            };
            rb.insert("content", content(&[media.to_owned()], Some(&Value::Object(s))));
            rb.insert(NORMALIZED_KEY, Value::Array(vec![Value::from("swagger2-form-data")]));
            out.insert("requestBody", Value::Object(rb));
        }
        out
    }
}

/// `basic` becomes HTTP basic; `oauth2` flows move under `flows`; `apiKey` is unchanged.
fn security_schemes(defs: &Map) -> Map {
    let mut out = Map::new();
    for (name, d) in defs {
        let Value::Object(dm) = d else {
            out.insert(name.clone(), d.clone());
            continue;
        };
        match dm.get("type").and_then(Value::as_str) {
            Some("basic") => {
                let mut s: Map = dm
                    .iter()
                    .filter(|(k, _)| *k != "type")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                s.insert("type", Value::from("http"));
                s.insert("scheme", Value::from("basic"));
                out.insert(name.clone(), Value::Object(s));
            }
            Some("oauth2") => {
                let skip = ["flow", "authorizationUrl", "tokenUrl", "scopes"];
                let mut s: Map = dm
                    .iter()
                    .filter(|(k, _)| !skip.contains(&k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let flow = dm.get("flow").map_or_else(|| "undefined".to_owned(), js_string);
                let key = match flow.as_str() {
                    "implicit" => "implicit".to_owned(),
                    "password" => "password".to_owned(),
                    "application" => "clientCredentials".to_owned(),
                    "accessCode" => "authorizationCode".to_owned(),
                    other => other.to_owned(),
                };
                let mut f = Map::new();
                if let Some(u) = dm.get("authorizationUrl") {
                    f.insert("authorizationUrl", u.clone());
                }
                if let Some(u) = dm.get("tokenUrl") {
                    f.insert("tokenUrl", u.clone());
                }
                let scopes = match dm.get("scopes") {
                    None | Some(Value::Null) => Value::object(),
                    Some(v) => v.clone(),
                };
                f.insert("scopes", scopes);
                let mut flows = Map::new();
                flows.insert(key, Value::Object(f));
                s.insert("flows", Value::Object(flows));
                out.insert(name.clone(), Value::Object(s));
            }
            _ => {
                out.insert(name.clone(), d.clone());
            }
        }
    }
    out
}

/// Convert a Swagger 2.0 document; anything else is returned unchanged, without diagnostics.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the document in the order of the TypeScript source"
)]
pub fn swagger2_to_openapi3(spec: &Value) -> Option<(Value, Vec<Diagnostic>)> {
    let doc = spec.as_object()?;
    if doc.get("swagger").and_then(Value::as_str) != Some("2.0") {
        return None;
    }
    let empty = Map::new();
    let global_params = match doc.get("parameters") {
        Some(Value::Object(p)) => p,
        _ => &empty,
    };
    let c = Converter {
        root_consumes: strings(doc.get("consumes")).unwrap_or_else(|| vec!["application/json".to_owned()]),
        root_produces: strings(doc.get("produces")).unwrap_or_else(|| vec!["application/json".to_owned()]),
        global_params,
    };

    let mut paths = Map::new();
    if let Some(Value::Object(doc_paths)) = doc.get("paths") {
        for (path, item) in doc_paths {
            let Value::Object(item_map) = item else {
                paths.insert(path.clone(), item.clone());
                continue;
            };
            let path_params: &[Value] = match item_map.get("parameters") {
                Some(Value::Array(a)) => a,
                _ => &[],
            };
            let mut out = Map::new();
            for (k, v) in item_map {
                if let (true, Value::Object(op)) = (METHODS.contains(&k.as_str()), v) {
                    out.insert(k.clone(), Value::Object(c.operation(op, path_params)));
                } else if k == "parameters" {
                    let kept: Vec<Value> = path_params
                        .iter()
                        .filter(|p| !is_body_or_form(&c.resolve(p)))
                        .map(Converter::param)
                        .collect();
                    if !kept.is_empty() {
                        out.insert(k.clone(), Value::Array(kept));
                    }
                } else if k.starts_with("x-") {
                    out.insert(k.clone(), v.clone());
                } else {
                    out.insert(k.clone(), schema(v));
                }
            }
            paths.insert(path.clone(), Value::Object(out));
        }
    }

    let mut components = Map::new();
    if let Some(Value::Object(defs)) = doc.get("definitions") {
        components.insert("schemas", Value::Object(named(defs)));
    }
    let params: Map = global_params
        .iter()
        .filter(|(_, p)| !is_body_or_form(p))
        .map(|(n, p)| (n.clone(), Converter::param(p)))
        .collect();
    if !params.is_empty() {
        components.insert("parameters", Value::Object(params));
    }
    if let Some(Value::Object(responses)) = doc.get("responses") {
        let r: Map = responses
            .iter()
            .map(|(n, r)| (n.clone(), Converter::response(r, &c.root_produces)))
            .collect();
        components.insert("responses", Value::Object(r));
    }
    if let Some(Value::Object(defs)) = doc.get("securityDefinitions") {
        components.insert("securitySchemes", Value::Object(security_schemes(defs)));
    }

    let mut out = Map::new();
    out.insert("openapi", Value::from("3.0.3"));
    for (k, v) in doc {
        if [
            "swagger",
            "host",
            "basePath",
            "schemes",
            "consumes",
            "produces",
            "definitions",
            "parameters",
            "responses",
            "securityDefinitions",
        ]
        .contains(&k.as_str())
        {
            continue;
        }
        out.insert(
            k.clone(),
            if k == "paths" {
                Value::Object(paths.clone())
            } else {
                v.clone()
            },
        );
    }
    let host = doc.get("host").and_then(Value::as_str);
    let base_path = doc.get("basePath").and_then(Value::as_str);
    if host.is_some() || base_path.is_some() {
        let schemes = strings(doc.get("schemes")).unwrap_or_else(|| vec!["https".to_owned()]);
        let base = format!("{}{}", host.unwrap_or(""), base_path.unwrap_or(""));
        let servers = schemes
            .iter()
            .map(|s| {
                let mut server = Map::new();
                let url = if host.is_some() {
                    format!("{s}://{base}")
                } else {
                    base.clone()
                };
                server.insert("url", Value::String(url));
                Value::Object(server)
            })
            .collect();
        out.insert("servers", Value::Array(servers));
    }
    if !components.is_empty() {
        out.insert("components", Value::Object(components));
    }
    out.insert(NORMALIZED_KEY, Value::Array(vec![Value::from("swagger2")]));
    Some((
        Value::Object(out),
        vec![Diagnostic::info(
            SWAGGER2_CODE,
            "/swagger",
            "Swagger 2.0 document converted to OpenAPI 3.0.3 before transforms",
            "convert the spec to OpenAPI 3 upstream to drop this step",
        )],
    ))
}
