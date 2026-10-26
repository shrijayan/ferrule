//! The template tokens that depend on how the client authenticates and on the example request:
//! the spellings the oracle SDKs' tests and README use for each auth scheme.

use super::input::{Auth, AuthOpt, AuthScheme, ExampleRequest};
use super::render::TemplateVars;
use crate::emit::common::py_string;

/// A flag token: non-empty when set (see [`super::render`]).
#[must_use]
pub fn flag(on: bool) -> String {
    if on { "1".to_owned() } else { String::new() }
}

/// Whether the tests check the header the credentials go in (`test_validate_headers`): every
/// scheme but none and a query parameter (OneBusAway has no such test).
#[must_use]
pub fn validates_headers(auth: &Auth) -> bool {
    !auth.opts.is_empty() && !matches!(auth.scheme, AuthScheme::None | AuthScheme::Query(_))
}

fn each(opts: &[AuthOpt], f: impl Fn(&AuthOpt) -> String) -> String {
    opts.iter().map(f).collect()
}

/// `client.copy(...)` checks of `test_copy`, one block per option, each after a blank line.
fn copy_checks(opts: &[AuthOpt], client: &str) -> String {
    each(opts, |o| {
        let another = py_string(&format!("another {}", o.value));
        format!(
            "\n        copied = {client}.copy({name}={another})\n        assert copied.{name} == {another}\n        assert {client}.{name} == {value}\n",
            name = o.name,
            value = py_string(&o.value),
        )
    })
}

/// The `update_env(...)` arguments that unset every option's environment variable (one per
/// line, with a magic trailing comma, when there are several; an empty mapping for none).
fn unset_env(opts: &[AuthOpt]) -> String {
    let envs: Vec<String> = opts
        .iter()
        .filter_map(|o| o.env.as_deref())
        .map(|e| format!("{}: Omit()", py_string(e)))
        .collect();
    match envs.as_slice() {
        [] => "**{}".to_owned(),
        [one] => format!("**{{{one}}}"),
        many => format!("**{{{},}}", many.join(", ")),
    }
}

/// `test_validate_headers`' check of the header the client sends.
fn header_check(auth: &Auth) -> String {
    let first = auth.opts.first().map_or("", |o| o.name.as_str());
    match &auth.scheme {
        AuthScheme::Bearer => {
            format!("assert request.headers.get(\"Authorization\") == f\"Bearer {{{first}}}\"")
        }
        AuthScheme::Basic => "assert \"Basic\" in request.headers.get(\"Authorization\")".to_owned(),
        AuthScheme::Header(name) => format!("assert request.headers.get({}) == {first}", py_string(name)),
        AuthScheme::None | AuthScheme::Query(_) => String::new(),
    }
}

/// `a an` before an option name in README.md: the oracles write "an `api_key`" but
/// "a `auth_token`", so it is not the English rule.
fn article(name: &str) -> &'static str {
    if name == "api_key" { "an" } else { "a" }
}

/// The README client's credential argument: read from its environment variable, else (an option
/// without `read_env`) a placeholder the reader replaces, since nothing would fill it.
fn readme_auth_arg(o: &AuthOpt) -> String {
    match &o.env {
        Some(env) => format!(
            "{}=os.environ.get({}),  # This is the default and can be omitted",
            o.name,
            py_string(env)
        ),
        None => format!("{}={},", o.name, py_string(&o.value)),
    }
}

/// The auth tokens.
pub fn auth_vars(auth: &Auth, vars: &mut TemplateVars) {
    let opts = &auth.opts;
    let query = match &auth.scheme {
        AuthScheme::Query(param) => param.clone(),
        _ => String::new(),
    };
    let first = opts.first();
    let first_name = first.map_or("", |o| o.name.as_str());
    let validate = validates_headers(auth);
    // README.md shows the options without a default (imagekit's `password` has one).
    let readme = opts.iter().find(|o| !o.has_default).or(first);
    // The query parameter in a request's expected parameters: as the variable, as its value, and
    // URL-encoded.
    let (item, value_item, raw) = match first {
        Some(o) if !query.is_empty() => (
            format!("{}: {first_name}", py_string(&query)),
            format!("{}: {}", py_string(&query), py_string(&o.value)),
            format!("&{query}={}", o.value.replace(' ', "+")),
        ),
        _ => (String::new(), String::new(), String::new()),
    };
    let more = |s: &str| if s.is_empty() { String::new() } else { format!(", {s}") };
    vars.extend([
        ("AUTH_QUERY", query.clone()),
        ("AUTH_FIRST", first_name.to_owned()),
        ("AUTH_QUERY_MORE", more(&item)),
        ("AUTH_QUERY_VALUE_MORE", more(&value_item)),
        ("AUTH_QUERY_ITEM", item),
        ("AUTH_QUERY_RAW", raw),
        ("AUTH_VALIDATE", flag(validate)),
        ("AUTH_ARGS", each(opts, |o| format!("{name}={name}, ", name = o.name))),
        ("AUTH_NONE_ARGS", each(opts, |o| format!("{}=None, ", o.name))),
        ("AUTH", flag(!opts.is_empty())),
        (
            "AUTH_DECLS",
            each(opts, |o| format!("{} = {}\n", o.name, py_string(&o.value))),
        ),
        (
            "AUTH_DECL_LINES",
            opts.iter()
                .map(|o| format!("{} = {}", o.name, py_string(&o.value)))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        ("AUTH_COPY", copy_checks(opts, "client")),
        ("AUTH_COPY_ASYNC", copy_checks(opts, "async_client")),
        ("AUTH_HEADER_CHECK", header_check(auth)),
        ("AUTH_UNSET_ENV", unset_env(opts)),
        ("README_AUTH", flag(readme.is_some())),
        ("README_AUTH_OPT", readme.map_or_else(String::new, |o| o.name.clone())),
        ("README_AUTH_ARG", readme.map_or_else(String::new, readme_auth_arg)),
        (
            "README_AUTH_ENV",
            readme.and_then(|o| o.env.clone()).unwrap_or_default(),
        ),
        (
            "README_AUTH_VALUE",
            readme.map_or_else(String::new, |o| o.value.clone()),
        ),
        (
            "README_AUTH_LABEL",
            readme.map_or_else(String::new, |o| o.label.clone()),
        ),
        (
            "README_AUTH_ARTICLE",
            readme.map_or("a", |o| article(&o.name)).to_owned(),
        ),
    ]);
}

fn args(e: &ExampleRequest, readme: bool) -> String {
    e.args
        .iter()
        .map(|a| {
            let value = if readme { &a.readme_value } else { &a.value };
            // A path parameter is positional.
            if a.name.is_empty() {
                value.clone()
            } else {
                format!("{}={value}", a.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The example-request tokens: `EX_*` for README.md, `PROBE_*` for the retry tests.
pub fn example_vars(example: &ExampleRequest, probe: &ExampleRequest, vars: &mut TemplateVars) {
    let plain = args(example, false);
    let readme = args(example, true);
    let probe_args = args(probe, false);
    vars.extend([
        ("EX_ACCESSOR", example.accessor.clone()),
        ("EX_METHOD", example.method.clone()),
        // Stainless writes README calls with a trailing comma, so ruff splits them.
        (
            "EX_ARGS",
            if readme.is_empty() {
                readme
            } else {
                format!("{readme},")
            },
        ),
        ("EX_ARGS_PLAIN", plain),
        ("EX_RESULT", example.result.clone()),
        ("EX_PARSED", example.parsed.clone()),
        ("EX_PRINT", example.print_property.clone().unwrap_or_default()),
        (
            "EX_PARSED_PRINT",
            example
                .parsed_property
                .as_ref()
                .map_or_else(String::new, |p| format!(".{p}")),
        ),
        ("PROBE_VERB", probe.http_method.as_str().to_owned()),
        ("PROBE_PATH", probe.path.clone()),
        ("PROBE_ACCESSOR", probe.accessor.clone()),
        ("PROBE_METHOD", probe.method.clone()),
        (
            "PROBE_ARGS_COMMA",
            if probe_args.is_empty() {
                String::new()
            } else {
                format!("{probe_args}, ")
            },
        ),
        ("PROBE_ARGS", probe_args),
        ("LEAK_TESTS", flag(!probe.paginated)),
    ]);
}
