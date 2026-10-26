//! `x-ferrule-unwrap`: the response envelope field an operation returns instead of the whole body
//! (Stainless's `unwrap_response`). A method names it, or opts out with `false`; otherwise
//! `settings.unwrap_response_fields` names fields to unwrap from a one-field envelope, or from
//! any envelope with `settings.unwrap_multiproperty_response_fields` (the opencode SDK's
//! `{location, data}`).

use super::json::{bool_, json_schema_of, obj, property_names, success_response};
use crate::Diagnostic;
use crate::value::{Map, Value};

/// The field `method` unwraps from the 2xx JSON response of `op`, if any; a named field the
/// response lacks is FER1007 (a warning) and nothing is unwrapped.
#[must_use]
pub fn resolve_unwrap(
    spec: &Value,
    config: &Value,
    method: &Value,
    op: &Map,
    pointer: &str,
) -> (Option<String>, Option<Diagnostic>) {
    let fields = success_response(spec, op)
        .map_or_else(Vec::new, |r| property_names(spec, json_schema_of(obj(r, "content")), 0));
    match method.get("unwrap_response") {
        Some(Value::Bool(false)) => return (None, None),
        Some(Value::String(field)) => {
            if fields.iter().any(|f| f == field) {
                return (Some(field.clone()), None);
            }
            let warning = Diagnostic::warning(
                "FER1007",
                format!("{pointer}/unwrap_response"),
                format!("unwrap_response names {field}, which the response does not have."),
                "Name a top-level property of the 2xx JSON response, or remove unwrap_response.",
            );
            return (None, Some(warning));
        }
        _ => {}
    }
    let settings = obj(config.as_object().unwrap_or(super::json::empty()), "settings");
    let multi = bool_(settings, "unwrap_multiproperty_response_fields").unwrap_or(false);
    if fields.is_empty() || (fields.len() > 1 && !multi) {
        return (None, None);
    }
    let wanted = settings.get("unwrap_response_fields").and_then(Value::as_array);
    let field = wanted
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|w| fields.iter().any(|f| f == w));
    (field.map(str::to_owned), None)
}
