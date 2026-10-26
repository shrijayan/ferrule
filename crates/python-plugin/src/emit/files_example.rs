//! The request README.md shows and `tests/test_client.py` retries: `readme.example_requests
//! .default` from the config when [`super::files_input::ProjectMeta`] carries it, else the first
//! method that needs no arguments, else the first method with its required arguments filled from
//! the spec's examples.

use serde_json::Value;

use super::common::{method_args, py_string, return_model, stainless_method_order};
use super::files::api_tests::{EXAMPLE_DATE, EXAMPLE_DATETIME, Uses, example_value};
use super::files::input::{ExampleArg, ExampleRequest};
use super::files_input::ReadmeRequest;
use crate::js;
use crate::model::{SdkClient, SdkMethod, SdkParam, SdkResource, SdkType};
use crate::naming::{singular, snake_case, split_words};

/// A JSON value as a one-line Python literal; `trailing` adds the trailing comma Stainless writes
/// in a README dict (lists have none).
fn py_literal(value: &Value, trailing: bool) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => py_string(s),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(|v| py_literal(v, trailing)).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(fields) => {
            if fields.is_empty() {
                return "{}".to_owned();
            }
            let fields: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", py_string(k), py_literal(v, trailing)))
                .collect();
            format!("{{{}{}}}", fields.join(", "), if trailing { "," } else { "" })
        }
    }
}

/// [`example_value`]'s layout collapsed onto one line: `{\n    "a": 1,\n}` -> `{"a": 1}`.
fn one_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines().map(str::trim) {
        if out.ends_with(',') {
            if line.starts_with(['}', ']', ')']) {
                out.pop();
            } else {
                out.push(' ');
            }
        }
        out.push_str(line);
    }
    out
}

/// `literal` as Python's `str()` of it: a string literal [`py_string`] wrote unescaped,
/// `True`/`False`/`None` as `path_template` writes them, anything else as written.
fn str_of(literal: &str) -> String {
    match literal {
        "True" => return "true".to_owned(),
        "False" => return "false".to_owned(),
        "None" => return "null".to_owned(),
        _ => {}
    }
    let Some(inner) = literal.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return literal.to_owned();
    };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// A JSON value as Python's `str()` of it, as `path_template` formats a path argument.
fn str_of_json(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => str_of(&py_literal(other, false)),
    }
}

/// `text` percent-encoded as `path_template` encodes a path segment: RFC 3986 `pchar` is safe.
fn quote_segment(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// `m`'s path with each path parameter `value` gives filled in, as the client sends it and respx
/// matches it (knock's probe mocks `/v1/users/user_id`); the others stay placeholders.
fn resolved_path(m: &SdkMethod, value: impl Fn(&SdkParam) -> Option<String>) -> String {
    let mut path = m.path.clone();
    for p in &m.path_params {
        if let Some(v) = value(p) {
            path = path.replacen(&format!("{{{}}}", p.wire_name), &quote_segment(&v), 1);
        }
    }
    path
}

/// A path argument: positional when it is the method's only path parameter (knock's
/// `get("user_id")`), else by keyword, since the call order (last path parameter first) is not
/// the signature's.
fn path_arg(m: &SdkMethod, p: &SdkParam, value: String, readme_value: String) -> ExampleArg {
    let named = arg(p, value, readme_value);
    if m.path_params.len() == 1 {
        ExampleArg {
            name: String::new(),
            ..named
        }
    } else {
        named
    }
}

fn arg(p: &SdkParam, value: String, readme_value: String) -> ExampleArg {
    ExampleArg {
        name: p.py_name.clone(),
        value,
        readme_value,
        required: p.required,
    }
}

/// The README's names for `method`'s result.
struct ResultNames {
    result: String,
    parsed: String,
    print_property: Option<String>,
    parsed_property: Option<String>,
}

/// `id` if the returned model has one, else its first property.
fn shown_property(client: &SdkClient, r: &SdkResource, m: &SdkMethod) -> Option<String> {
    let ret = return_model(Some(r), m)?;
    let fields = match ret.inline {
        Some(SdkType::Object(o)) => o.fields.clone(),
        Some(_) => return None,
        None => client.models.iter().find(|d| d.r#ref == ret.r#ref)?.ty.fields.clone(),
    };
    fields
        .iter()
        .find(|f| f.wire_name == "id")
        .or(fields.first())
        .map(|f| f.py_name.clone())
}

/// The variables the README assigns `method`'s result and a parsed raw response to, and the
/// properties it prints of them (none for a request without arguments, as OneBusAway's).
fn result_names(client: &SdkClient, r: &SdkResource, m: &SdkMethod, has_args: bool) -> ResultNames {
    let leaf = r.path.last().map_or("", String::as_str);
    let parsed = snake_case(&singular(&split_words(leaf)));
    let result = if m.pagination.is_some() {
        "page".to_owned()
    } else {
        match return_model(Some(r), m) {
            Some(ret) if ret.inline.is_none() => snake_case(&split_words(&ret.r#ref.name)),
            _ => parsed.clone(),
        }
    };
    let parsed_property = if has_args { shown_property(client, r, m) } else { None };
    let print_property = match &m.pagination {
        Some(p) if has_args => p
            .response
            .iter()
            .find(|(_, purpose)| purpose == "items")
            .map(|(f, _)| f.clone()),
        _ => parsed_property.clone(),
    };
    ResultNames {
        result,
        parsed,
        print_property,
        parsed_property,
    }
}

fn request(client: &SdkClient, r: &SdkResource, m: &SdkMethod, args: Vec<ExampleArg>, path: String) -> ExampleRequest {
    let names = result_names(client, r, m, !args.is_empty());
    ExampleRequest {
        accessor: r.path.join("."),
        method: m.name.clone(),
        args,
        http_method: m.http_method,
        path,
        result: names.result,
        parsed: names.parsed,
        print_property: names.print_property,
        parsed_property: names.parsed_property,
        paginated: m.pagination.is_some(),
    }
}

fn walk<'a>(resources: &'a [SdkResource], out: &mut Vec<(&'a SdkResource, &'a SdkMethod)>) {
    for r in resources {
        for m in stainless_method_order(&r.methods) {
            out.push((r, m));
        }
        walk(&r.subresources, out);
    }
}

/// `p`'s value in the generated tests: the spec's example, else a placeholder by type; query
/// parameters ignore the example, as in the oracles. A date or datetime goes as its ISO string,
/// which the `Union[str, date]` parameter accepts: `tests/test_client.py` and README.md do not
/// import `parse_date` or `parse_datetime`.
fn test_value(p: &SdkParam, query: bool) -> String {
    let mut uses = Uses::new();
    let example = if query { None } else { p.example.as_ref() };
    let text = one_line(&example_value(
        &p.ty,
        &mut uses,
        "",
        example,
        &p.wire_name,
        p.minimum.or(p.maximum),
    ));
    if uses.is_empty() {
        return text;
    }
    let (date, datetime) = (py_string(EXAMPLE_DATE), py_string(EXAMPLE_DATETIME));
    text.replace(&format!("parse_datetime({datetime})"), &datetime)
        .replace(&format!("parse_date({date})"), &date)
}

/// The retry probe's path: `m`'s with the values [`required_args`] passes.
fn probe_path(m: &SdkMethod) -> String {
    resolved_path(m, |p| Some(str_of(&test_value(p, false))))
}

/// The required arguments of `m` with the values its generated tests use (the spec's example,
/// else a placeholder by type), path parameters first ([`path_arg`]): the retry probe's, as in
/// the oracles (imagekit's probe sends `file_name="fileName"`, not its README's value).
fn required_args(m: &SdkMethod) -> Vec<ExampleArg> {
    let mut args: Vec<ExampleArg> = Vec::new();
    // The last path parameter first, as the tests call it.
    let mut path: Vec<&SdkParam> = m.path_params.iter().collect();
    if let Some(last) = path.pop() {
        path.insert(0, last);
    }
    for p in path {
        let v = test_value(p, false);
        args.push(path_arg(m, p, v.clone(), v));
    }
    let mut kwargs: Vec<&SdkParam> = method_args(m).into_iter().filter(|p| p.required).collect();
    kwargs.sort_by(|a, b| js::cmp(&a.py_name, &b.py_name));
    for p in kwargs {
        let query = m.query_params.iter().any(|q| std::ptr::eq(q, p));
        let v = test_value(p, query);
        args.push(arg(p, v.clone(), v));
    }
    args
}

/// `readme.example_requests.default`: its method and arguments.
fn from_config<'a>(
    client: &'a SdkClient,
    req: &ReadmeRequest,
) -> Option<(&'a SdkResource, &'a SdkMethod, Vec<ExampleArg>, String)> {
    let (verb, path) = req.endpoint.split_once(' ')?;
    let mut all = Vec::new();
    walk(&client.resources, &mut all);
    let (r, m) = all
        .into_iter()
        .find(|(_, m)| m.http_method.as_str() == verb && m.path == path)?;
    let mut args: Vec<ExampleArg> = Vec::new();
    for p in &m.path_params {
        if let Some(v) = req.params.get(&p.wire_name) {
            args.push(path_arg(m, p, py_literal(v, false), py_literal(v, true)));
        }
    }
    for p in method_args(m) {
        if let Some(v) = req.params.get(&p.wire_name).or_else(|| req.params.get(&p.py_name)) {
            args.push(arg(p, py_literal(v, false), py_literal(v, true)));
        }
    }
    let path = resolved_path(m, |p| req.params.get(&p.wire_name).map(str_of_json));
    Some((r, m, args, path))
}

/// The first method with no required arguments; else the first one without path parameters,
/// with its required arguments.
fn fallback(client: &SdkClient) -> Option<(&SdkResource, &SdkMethod, Vec<ExampleArg>, String)> {
    let mut all = Vec::new();
    walk(&client.resources, &mut all);
    if let Some(&(r, m)) = all
        .iter()
        .find(|(_, m)| m.path_params.is_empty() && method_args(m).iter().all(|p| !p.required))
    {
        return Some((r, m, Vec::new(), m.path.clone()));
    }
    let (r, m) = all.into_iter().find(|(_, m)| m.path_params.is_empty())?;
    Some((r, m, required_args(m), m.path.clone()))
}

/// The README example (`prebuilt`, else `config`, else a fallback) and the retry probe: the same
/// method with its required arguments.
#[must_use]
pub fn examples(
    client: &SdkClient,
    prebuilt: Option<&ExampleRequest>,
    config: Option<&ReadmeRequest>,
) -> Option<(ExampleRequest, ExampleRequest)> {
    if let Some(example) = prebuilt {
        let mut probe = example.clone();
        probe.args.retain(|a| a.required);
        return Some((example.clone(), probe));
    }
    let (r, m, args, path) = config
        .and_then(|c| from_config(client, c))
        .or_else(|| fallback(client))?;
    Some((
        request(client, r, m, args, path),
        request(client, r, m, required_args(m), probe_path(m)),
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{one_line, py_literal, quote_segment, str_of};

    #[test]
    fn writes_json_as_python() {
        let v = json!({"a": [1, 0.1, "x"], "b": null, "c": false});
        assert_eq!(
            py_literal(&v, false),
            "{\"a\": [1, 0.1, \"x\"], \"b\": None, \"c\": False}"
        );
        assert_eq!(
            py_literal(&v, true),
            "{\"a\": [1, 0.1, \"x\"], \"b\": None, \"c\": False,}"
        );
        assert_eq!(py_literal(&json!({}), true), "{}");
    }

    #[test]
    fn reads_a_literal_as_python_str_and_quotes_it_as_a_segment() {
        assert_eq!(str_of("\"a \\\"b\\\" \\u0001\""), "a \"b\" \u{1}");
        assert_eq!(str_of("True"), "true");
        assert_eq!(str_of("0"), "0");
        assert_eq!(quote_segment("a b/c@d~\u{e9}"), "a%20b%2Fc@d~%C3%A9");
    }

    #[test]
    fn collapses_a_laid_out_example() {
        assert_eq!(
            one_line("{\n    \"a\": 1,\n    \"b\": [\n        2\n    ],\n}"),
            "{\"a\": 1, \"b\": [2]}"
        );
    }
}
