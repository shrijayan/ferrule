//! `x-ferrule-pagination` (ADR 0003): normalizes the `pagination` schemes of stainless.yml and
//! matches them against operations.

use super::json::{
    arr, bool_, deref, is_array_schema, items_schema, obj, obj_of, property_names, property_schema, str_,
    success_response,
};
use crate::Diagnostic;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::value::{Map, Value, to_json};

/// Every pagination `type` the public stainless.yml schema allows.
pub const PAGINATION_SCHEMES: [&str; 6] = [
    "cursor",
    "cursor_id",
    "cursor_url",
    "fake_page",
    "offset",
    "page_number",
];

/// Name given to the single-object `pagination:` form, which has no `name`.
pub const UNNAMED_PAGE: &str = "page";

/// The `x-ferrule-pagination` field maps written in config order instead of sorted: Stainless
/// declares a page's fields in the order stainless.yml lists them (knock-go `MsTeamsPagination`
/// has `SkipToken`, then `MsTeamsTeams`).
pub const CONFIG_ORDER_MAPS: [(&str, &str); 3] = [
    ("x-ferrule-pagination", "request"),
    ("x-ferrule-pagination", "response"),
    ("x-ferrule-pagination", "headers"),
];

#[derive(Default)]
struct Flat {
    body: Map,
    headers: Map,
}

fn merge(into: &mut Flat, from: Flat) {
    for (k, v) in from.body {
        into.body.insert(k, v);
    }
    for (k, v) in from.headers {
        into.headers.insert(k, v);
    }
}

/// Flatten configured fields. A field is listed when it has a purpose or has no annotated
/// descendants; the fields under an annotated object or array item are listed by path.
/// Returns whether any field carries `x-stainless-pagination-property`.
fn flatten(fields: &Map, prefix: &str, out: &mut Flat, depth: usize) -> bool {
    let mut annotated = false;
    for (name, raw) in fields {
        let def = obj_of(Some(raw));
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        let prop = obj(def, "x-stainless-pagination-property");
        let purpose = str_(prop, "purpose");
        let from_header = str_(prop, "from_header");
        let mut children = Flat::default();
        let child_annotated = depth < 16 && {
            let a = flatten(obj(def, "properties"), &path, &mut children, depth + 1);
            let b = flatten(
                obj(obj(def, "items"), "properties"),
                &format!("{path}.[]"),
                &mut children,
                depth + 1,
            );
            a || b
        };
        if child_annotated {
            merge(out, children);
        }
        if let Some(h) = from_header {
            out.headers.insert(h, Value::from(purpose.unwrap_or(name)));
        } else if purpose.is_some() || !child_annotated {
            out.body.insert(path, Value::from(purpose.unwrap_or(name)));
        }
        if purpose.is_some() || from_header.is_some() || child_annotated {
            annotated = true;
        }
    }
    annotated
}

/// A configured scheme, ready to match.
#[derive(Debug, Clone)]
pub struct Scheme {
    /// The `x-ferrule-pagination` value.
    pub extension: Map,
    /// Top-level request field names.
    pub request_fields: Vec<String>,
}

impl Scheme {
    /// `extension.page`.
    #[must_use]
    pub fn page(&self) -> &str {
        str_(&self.extension, "page").unwrap_or("")
    }

    fn response(&self) -> &Map {
        obj(&self.extension, "response")
    }

    fn specificity(&self) -> usize {
        self.request_fields.len() + self.response().len()
    }
}

/// Stainless's implicit items field: when no response field has purpose `items`, the one
/// top-level `type: array` field without a purpose holds the page items (knock-mgmt's
/// `entries`, ogx's `data`). With none or several such fields nothing is inferred.
fn implicit_items(fields: &Map, flat: &Map) -> Option<String> {
    if flat.values().any(|p| p.as_str() == Some("items")) {
        return None;
    }
    let mut arrays = fields.iter().filter(|(_, raw)| {
        let def = obj_of(Some(raw));
        let prop = obj(def, "x-stainless-pagination-property");
        str_(def, "type") == Some("array") && str_(prop, "purpose").is_none() && str_(prop, "from_header").is_none()
    });
    match (arrays.next(), arrays.next()) {
        (Some((name, _)), None) => Some(name.clone()),
        _ => None,
    }
}

fn to_scheme(raw: &Map, name: &str, ty: &str) -> Scheme {
    let mut request = Flat::default();
    let mut response = Flat::default();
    flatten(obj(raw, "request"), "", &mut request, 0);
    flatten(obj(raw, "response"), "", &mut response, 0);
    if let Some(items) = implicit_items(obj(raw, "response"), &response.body) {
        response.body.insert(items, Value::from("items"));
    }
    let mut headers = request.headers;
    for (k, v) in response.headers {
        headers.insert(k, v);
    }
    let mut ext = Map::new();
    ext.insert("scheme", Value::from(ty));
    ext.insert("page", Value::from(name));
    ext.insert(
        "paramLocation",
        Value::from(if str_(raw, "param_location") == Some("body") {
            "body"
        } else {
            "query"
        }),
    );
    ext.insert(
        "continueOnEmptyItems",
        Value::Bool(bool_(raw, "continue_on_empty_items").unwrap_or(false)),
    );
    ext.insert("request", Value::Object(request.body));
    ext.insert("response", Value::Object(response.body));
    if !headers.is_empty() {
        ext.insert("headers", Value::Object(headers));
    }
    Scheme {
        extension: ext,
        request_fields: obj(raw, "request").keys().cloned().collect(),
    }
}

/// One configured field as an ordered entry: `{name, type?, purpose?, fields?}`, its object
/// properties as `fields` in config order.
fn field_entries(fields: &Map) -> Vec<Value> {
    fields
        .iter()
        .map(|(name, raw)| {
            let def = obj_of(Some(raw));
            let mut entry = Map::new();
            entry.insert("name", Value::from(name.as_str()));
            if let Some(ty) = str_(def, "type") {
                entry.insert("type", Value::from(ty));
            }
            if let Some(purpose) = str_(obj(def, "x-stainless-pagination-property"), "purpose") {
                entry.insert("purpose", Value::from(purpose));
            }
            let props = obj(def, "properties");
            if !props.is_empty() {
                entry.insert("fields", Value::Array(field_entries(props)));
            }
            Value::Object(entry)
        })
        .collect()
}

/// `x-ferrule-page-schemes`: every configured scheme with its class name (spelled with the
/// custom casings) and its request and response fields as lists, so they keep config order in
/// the annotated spec. The implicit items field gets the purpose `items`.
#[must_use]
pub fn page_scheme_extension(config: &Value, class_name: impl Fn(&str) -> String) -> Option<Value> {
    let list: Vec<&Value> = match config.get("pagination") {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(v @ Value::Object(_)) => vec![v],
        _ => return None,
    };
    let schemes = list
        .into_iter()
        .map(|entry| {
            let raw = obj_of(Some(entry));
            let name = str_(raw, "name").unwrap_or(UNNAMED_PAGE);
            let response = obj(raw, "response");
            let mut flat = Flat::default();
            flatten(response, "", &mut flat, 0);
            let implicit = implicit_items(response, &flat.body);
            let mut fields = field_entries(response);
            for field in &mut fields {
                if let Value::Object(f) = field
                    && implicit.as_deref() == str_(f, "name")
                {
                    f.insert("purpose", Value::from("items"));
                }
            }
            let mut out = Map::new();
            out.insert("name", Value::from(name));
            out.insert("className", Value::from(class_name(name).as_str()));
            out.insert("kind", raw.get("type").cloned().unwrap_or(Value::Null));
            out.insert("request", Value::Array(field_entries(obj(raw, "request"))));
            out.insert("response", Value::Array(fields));
            Value::Object(out)
        })
        .collect();
    Some(Value::Array(schemes))
}

/// Normalize `pagination` (a list, or the single-object form) into schemes, in config order.
#[must_use]
pub fn read_schemes(config: &Value) -> (Vec<Scheme>, Vec<Diagnostic>) {
    let raw = config.get("pagination");
    let list: Vec<&Value> = match raw {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(v @ Value::Object(_)) => vec![v],
        _ => Vec::new(),
    };
    let single = !matches!(raw, Some(Value::Array(_)));
    let mut schemes = Vec::new();
    let mut diagnostics = Vec::new();
    for (i, entry) in list.into_iter().enumerate() {
        let p = obj_of(Some(entry));
        let pointer = to_json_pointer(&if single {
            vec![Seg::from("pagination")]
        } else {
            vec![Seg::from("pagination"), Seg::Index(i)]
        });
        let ty = p.get("type");
        let Some(ty) = ty.and_then(Value::as_str).filter(|t| PAGINATION_SCHEMES.contains(t)) else {
            let shown = ty.map_or_else(|| "undefined".to_owned(), to_json);
            diagnostics.push(Diagnostic::error(
                "FER1501",
                format!("{pointer}/type"),
                format!(
                    "Pagination type {shown} is not one of {}.",
                    PAGINATION_SCHEMES.join(", ")
                ),
                format!("Set type to one of: {}.", PAGINATION_SCHEMES.join(", ")),
            ));
            continue;
        };
        schemes.push(to_scheme(p, str_(p, "name").unwrap_or(UNNAMED_PAGE), ty));
    }
    (schemes, diagnostics)
}

/// Walk a dotted field path (`[]` = array items) through a schema; `None` when absent.
fn resolve_path<'a>(spec: &'a Value, schema: Option<&'a Value>, path: &str) -> Option<&'a Value> {
    let mut cur = schema;
    for seg in path.split('.') {
        cur = if seg == "[]" {
            items_schema(spec, cur, 0)
        } else {
            property_schema(spec, cur, seg, 0)
        };
        cur?;
    }
    cur
}

/// The request and response shape of an operation that pagination is matched against.
#[derive(Debug, Clone)]
pub struct OperationShape<'a> {
    pub query: Vec<String>,
    pub body: Vec<String>,
    pub response_schema: Option<&'a Value>,
}

fn json_schema(content: &Map) -> Option<&Value> {
    obj(content, "application/json").get("schema")
}

/// The query params, JSON body properties and 2xx JSON response schema of an operation.
#[must_use]
pub fn operation_shape<'a>(spec: &'a Value, path_item: &'a Map, op: &'a Map) -> OperationShape<'a> {
    let mut query: Vec<String> = Vec::new();
    for p in arr(path_item, "parameters").iter().chain(arr(op, "parameters")) {
        let param = deref(spec, Some(p));
        if let Some(Value::Object(param)) = param
            && str_(param, "in") == Some("query")
            && let Some(name) = str_(param, "name")
            && !query.iter().any(|q| q == name)
        {
            query.push(name.to_owned());
        }
    }
    let body = match deref(spec, op.get("requestBody")) {
        Some(Value::Object(rb)) => property_names(spec, json_schema(obj(rb, "content")), 0),
        _ => Vec::new(),
    };
    let response_schema = success_response(spec, op).and_then(|r| json_schema(obj(r, "content")));
    OperationShape {
        query,
        body,
        response_schema,
    }
}

/// Stainless's matching rule: every request field is a param in the scheme's location, and every
/// response field path exists in the 2xx JSON response, with the `items` field an array.
#[must_use]
pub fn matches(spec: &Value, scheme: &Scheme, shape: &OperationShape<'_>) -> bool {
    let params = if str_(&scheme.extension, "paramLocation") == Some("body") {
        &shape.body
    } else {
        &shape.query
    };
    if !scheme.request_fields.iter().all(|k| params.contains(k)) {
        return false;
    }
    scheme.response().iter().all(|(path, purpose)| {
        let field = resolve_path(spec, shape.response_schema, path);
        field.is_some() && (purpose.as_str() != Some("items") || is_array_schema(spec, field, 0))
    })
}

/// The most specific matching scheme (most fields), earliest in config order on ties.
#[must_use]
pub fn best_match<'s>(spec: &Value, schemes: &'s [Scheme], shape: &OperationShape<'_>) -> Option<&'s Scheme> {
    let mut best: Option<&Scheme> = None;
    for s in schemes {
        if matches(spec, s, shape) && best.is_none_or(|b| s.specificity() > b.specificity()) {
            best = Some(s);
        }
    }
    best
}
