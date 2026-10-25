//! `x-ferrule-client` (ADR 0003): client class settings from stainless.yml `organization`,
//! `client_settings`, `environments`, `security`, `security_schemes` and `query_settings`, plus
//! the security schemes and servers of the spec. Port of `converter/src/extensions/client.ts`.

use super::json::{arr, bool_, empty, num, obj, obj_of, str_};
use crate::Diagnostic;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::value::{Map, Value, to_json};

/// Stainless defaults (public schema): 60 s timeout, 2 retries, 0.5 s to 8 s backoff.
pub const DEFAULT_TIMEOUT_SECONDS: f64 = 60.0;
/// Environment name used when the config has no `environments` and the spec has a server.
pub const DEFAULT_ENVIRONMENT_NAME: &str = "production";

const OPT_TYPES: [&str; 4] = ["string", "number", "integer", "boolean"];
const SENDS: [(&str, &str); 4] = [
    ("send_in_header", "header"),
    ("send_as_query_param", "query"),
    ("send_as_body_param", "body"),
    ("send_as_path_param", "path"),
];

/// `onebusaway-sdk` -> `ONEBUSAWAY_SDK`; `AcmeCorp` -> `ACME_CORP`.
#[must_use]
pub fn upper_snake(name: &str) -> String {
    // .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    let chars: Vec<char> = name.chars().collect();
    let mut spaced = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c.is_ascii_lowercase() || c.is_ascii_digit()) && chars.get(i + 1).is_some_and(char::is_ascii_uppercase) {
            spaced.push(c);
            spaced.push('_');
            spaced.push(chars[i + 1]);
            i += 2;
        } else {
            spaced.push(c);
            i += 1;
        }
    }
    spaced
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .to_ascii_uppercase()
}

/// `^P(?:(\d+(?:\.\d+)?)D)?(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$`
fn iso_duration(s: &str) -> Option<[f64; 4]> {
    let b = s.as_bytes();
    let mut i = 0;
    if b.first() != Some(&b'P') {
        return None;
    }
    i += 1;
    let number = |i: &mut usize| -> Option<f64> {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        if *i == start {
            return None;
        }
        if *i < b.len() && b[*i] == b'.' && b.get(*i + 1).is_some_and(u8::is_ascii_digit) {
            *i += 1;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
        }
        s[start..*i].parse().ok()
    };
    let mut out = [0.0; 4];
    // Each unit: try a number followed by the designator; backtrack when it does not follow.
    let mut unit = |i: &mut usize, designator: u8, slot: usize| {
        let save = *i;
        if let Some(n) = number(i)
            && *i < b.len()
            && b[*i] == designator
        {
            *i += 1;
            out[slot] = n;
            return;
        }
        *i = save;
    };
    unit(&mut i, b'D', 0);
    if i < b.len() && b[i] == b'T' {
        i += 1;
        unit(&mut i, b'H', 1);
        unit(&mut i, b'M', 2);
        unit(&mut i, b'S', 3);
    }
    (i == b.len()).then_some(out)
}

/// `default_timeout`: milliseconds, or an ISO 8601 duration such as `PT60S`. Seconds out.
#[must_use]
pub fn timeout_seconds(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => (*n > 0.0 && n.is_finite()).then(|| n / 1000.0),
        Value::String(s) => {
            let [d, h, m, sec] = iso_duration(s)?;
            if s == "P" || s.ends_with('T') {
                return None;
            }
            let total = d * 86400.0 + h * 3600.0 + m * 60.0 + sec;
            (total > 0.0).then_some(total)
        }
        _ => None,
    }
}

fn read_opts(settings: &Map, diagnostics: &mut Vec<Diagnostic>) -> Vec<Map> {
    let mut out = Vec::new();
    for (name, raw) in obj(settings, "opts") {
        let o = obj_of(Some(raw));
        let pointer = to_json_pointer(&[Seg::from("client_settings"), Seg::from("opts"), Seg::from(name)]);
        let ty = o.get("type");
        let Some(ty) = ty.and_then(Value::as_str).filter(|t| OPT_TYPES.contains(t)) else {
            let shown = ty.map_or_else(|| "undefined".to_owned(), to_json);
            diagnostics.push(Diagnostic::error(
                "FER1301",
                format!("{pointer}/type"),
                format!("Client option {name} has type {shown}."),
                format!("Set type to one of: {}.", OPT_TYPES.join(", ")),
            ));
            continue;
        };
        let sends: Vec<(&str, &str, &str)> = SENDS
            .iter()
            .filter_map(|(key, loc)| str_(o, key).map(|v| (*loc, v, *key)))
            .collect();
        if sends.len() > 1 {
            diagnostics.push(Diagnostic::warning(
                "FER1302",
                &pointer,
                format!(
                    "Client option {name} is sent in {} places; only {} is used.",
                    sends.len(),
                    sends[0].2
                ),
                "Keep one of send_in_header, send_as_query_param, send_as_body_param, send_as_path_param.",
            ));
        }
        let auth = obj(o, "auth");
        let mut opt = Map::new();
        opt.insert("name", Value::from(name.as_str()));
        opt.insert("type", Value::from(ty));
        opt.insert("nullable", Value::Bool(bool_(o, "nullable").unwrap_or(false)));
        if let Some(d) = o.get("default") {
            opt.insert("default", d.clone());
        }
        if let Some(v) = str_(o, "read_env") {
            opt.insert("readEnv", Value::from(v));
        }
        if let Some(v) = str_(o, "description") {
            opt.insert("description", Value::from(v));
        }
        if let Some(scheme) = str_(auth, "security_scheme") {
            let mut a = Map::new();
            a.insert("securityScheme", Value::from(scheme));
            a.insert("role", Value::from(str_(auth, "role").unwrap_or("value")));
            opt.insert("auth", Value::Object(a));
        }
        if let Some((loc, v, _)) = sends.first() {
            let mut s = Map::new();
            s.insert("in", Value::from(*loc));
            s.insert("name", Value::from(*v));
            opt.insert("send", Value::Object(s));
        }
        if let Some(v) = str_(o, "server_variable") {
            opt.insert("serverVariable", Value::from(v));
        }
        out.push(opt);
    }
    out
}

fn read_scheme(raw: &Value) -> Option<Map> {
    let s = obj_of(Some(raw));
    let ty = str_(s, "type");
    let scheme = str_(s, "scheme").map(str::to_lowercase);
    let auth = obj(s, "x-stainless-auth");
    let keyword = str_(auth, "scheme_keyword");
    let mut out = Map::new();
    match (ty, scheme.as_deref()) {
        (Some("http"), Some(sc @ ("bearer" | "digest"))) => {
            out.insert("type", Value::from("http"));
            out.insert("scheme", Value::from(sc));
            if let Some(k) = keyword {
                out.insert("keyword", Value::from(k));
            }
            return Some(out);
        }
        (Some("http"), Some("basic")) => {
            out.insert("type", Value::from("http"));
            out.insert("scheme", Value::from("basic"));
            if let Some(k) = keyword {
                out.insert("keyword", Value::from(k));
            }
            out.insert(
                "disableBase64",
                Value::Bool(bool_(auth, "disable_base64").unwrap_or(false)),
            );
            return Some(out);
        }
        _ => {}
    }
    let name = str_(s, "name");
    let location = str_(s, "in").unwrap_or("header");
    if let (Some("apiKey"), Some(name)) = (ty, name)
        && ["header", "query", "cookie"].contains(&location)
    {
        out.insert("type", Value::from("apiKey"));
        out.insert("in", Value::from(location));
        out.insert("name", Value::from(name));
        return Some(out);
    }
    if ty == Some("oauth2") {
        out.insert("type", Value::from("oauth2"));
        out.insert("flows", Value::Object(obj(s, "flows").clone()));
        return Some(out);
    }
    if let (Some("openIdConnect"), Some(url)) = (ty, str_(s, "openIdConnectUrl")) {
        out.insert("type", Value::from("openIdConnect"));
        out.insert("url", Value::from(url));
        return Some(out);
    }
    None
}

fn read_schemes(spec: &Map, config: &Map, diagnostics: &mut Vec<Diagnostic>) -> Map {
    let mut out = Map::new();
    let sources: [(Vec<Seg>, &Map); 2] = [
        (
            vec![Seg::from("components"), Seg::from("securitySchemes")],
            obj(obj(spec, "components"), "securitySchemes"),
        ),
        (vec![Seg::from("security_schemes")], obj(config, "security_schemes")),
    ];
    for (at, schemes) in sources {
        for (name, raw) in schemes {
            let Some(scheme) = read_scheme(raw) else {
                let mut p = at.clone();
                p.push(Seg::from(name));
                diagnostics.push(Diagnostic::warning(
                    "FER1303",
                    to_json_pointer(&p),
                    format!("Security scheme {name} is not an http bearer/basic/digest, apiKey, oauth2 or openIdConnect scheme."),
                    "Fix the scheme, or override it under security_schemes in stainless.yml.",
                ));
                continue;
            };
            out.insert(name.clone(), Value::Object(scheme));
        }
    }
    out
}

fn read_security(list: &[Value]) -> Vec<Value> {
    list.iter()
        .filter_map(Value::as_object)
        .map(|req| {
            let mut out = Map::new();
            for (name, scopes) in req {
                let scopes = match scopes {
                    Value::Array(a) => a.iter().filter(|s| s.as_str().is_some()).cloned().collect(),
                    _ => Vec::new(),
                };
                out.insert(name.clone(), Value::Array(scopes));
            }
            Value::Object(out)
        })
        .collect()
}

fn read_environments(spec: &Map, config: &Map) -> Vec<(String, String)> {
    let configured: Vec<(String, String)> = obj(config, "environments")
        .iter()
        .filter_map(|(name, url)| url.as_str().map(|u| (name.clone(), u.to_owned())))
        .collect();
    if !configured.is_empty() {
        return configured;
    }
    let servers = arr(spec, "servers");
    let url = servers.first().and_then(Value::as_object).and_then(|s| str_(s, "url"));
    url.map_or_else(Vec::new, |u| vec![(DEFAULT_ENVIRONMENT_NAME.to_owned(), u.to_owned())])
}

fn read_http_limits(settings: &Map) -> Option<Map> {
    let python = obj(settings, "python");
    let Some(Value::Object(limits)) = python.get("default_http_limits") else {
        return None;
    };
    let mut out = Map::new();
    for (from, to) in [
        ("max_connections", "maxConnections"),
        ("max_keepalive_connections", "maxKeepaliveConnections"),
        ("keepalive_expiry", "keepaliveExpiry"),
    ] {
        if let Some(n) = num(limits, from) {
            out.insert(to, Value::Number(n));
        }
    }
    Some(out)
}

fn pick(value: Option<&Value>, allowed: &[&str], fallback: &str) -> Value {
    Value::from(
        value
            .and_then(Value::as_str)
            .filter(|v| allowed.contains(v))
            .unwrap_or(fallback),
    )
}

/// Build `x-ferrule-client` from the (untyped) config and the spec. Pure.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "mirrors the TypeScript object literal field by field, in output order"
)]
pub fn build_client(spec: &Value, config: &Value) -> (Value, Vec<Diagnostic>) {
    let spec = spec.as_object().unwrap_or(empty());
    let config = config.as_object().unwrap_or(empty());
    let mut diagnostics = Vec::new();
    let settings = obj(config, "client_settings");
    let org = str_(obj(config, "organization"), "name");
    let python = obj(obj(config, "targets"), "python");
    let fallback = org.or_else(|| str_(python, "package_name")).unwrap_or("");
    let name = str_(settings, "default_client_name").unwrap_or(fallback);
    let env_prefix = str_(settings, "default_env_prefix").map_or_else(|| upper_snake(fallback), str::to_owned);

    let opts = read_opts(settings, &mut diagnostics);
    let security_schemes = read_schemes(spec, config, &mut diagnostics);
    for opt in &opts {
        let Some(scheme) = opt
            .get("auth")
            .and_then(|a| a.get("securityScheme"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !security_schemes.contains_key(scheme) {
            let opt_name = opt.get("name").and_then(Value::as_str).unwrap_or("");
            let defined: Vec<&str> = security_schemes.keys().map(String::as_str).collect();
            let list = if defined.is_empty() {
                "(none defined)".to_owned()
            } else {
                defined.join(", ")
            };
            diagnostics.push(Diagnostic::error(
                "FER1304",
                to_json_pointer(&[
                    Seg::from("client_settings"),
                    Seg::from("opts"),
                    Seg::from(opt_name),
                    Seg::from("auth"),
                    Seg::from("security_scheme"),
                ]),
                format!("Client option {opt_name} authenticates with security scheme {scheme}, which is not defined."),
                format!("Use one of: {list}, or add it under security_schemes."),
            ));
        }
    }

    let mut timeout = DEFAULT_TIMEOUT_SECONDS;
    if let Some(raw) = settings.get("default_timeout") {
        match timeout_seconds(raw) {
            Some(t) => timeout = t,
            None => diagnostics.push(Diagnostic::error(
                "FER1305",
                "/client_settings/default_timeout",
                format!(
                    "default_timeout {} is not a positive number of milliseconds or an ISO 8601 duration.",
                    to_json(raw)
                ),
                "Write milliseconds (60000) or an ISO 8601 duration (PT60S).",
            )),
        }
    }

    let retries = obj(settings, "default_retries");
    let environments = read_environments(spec, config);
    let idempotency_header = str_(obj(settings, "idempotency"), "header");
    let mut default_headers = Map::new();
    for (k, v) in obj(settings, "default_headers") {
        if let Value::String(s) = v {
            default_headers.insert(k.clone(), Value::from(s.as_str()));
        }
    }
    let query = obj(config, "query_settings");
    let http_limits = read_http_limits(settings);

    let mut client = Map::new();
    client.insert("name", Value::from(name));
    client.insert("envPrefix", Value::from(env_prefix));
    client.insert("opts", Value::Array(opts.into_iter().map(Value::Object).collect()));
    client.insert(
        "environments",
        Value::Array(
            environments
                .iter()
                .map(|(n, u)| {
                    let mut e = Map::new();
                    e.insert("name", Value::from(n.as_str()));
                    e.insert("url", Value::from(u.as_str()));
                    Value::Object(e)
                })
                .collect(),
        ),
    );
    if let Some((first, _)) = environments.first() {
        client.insert("defaultEnvironment", Value::from(first.as_str()));
    }
    client.insert("securitySchemes", Value::Object(security_schemes));
    let security = if config.contains_key("security") {
        arr(config, "security")
    } else {
        arr(spec, "security")
    };
    client.insert("security", Value::Array(read_security(security)));
    client.insert("timeoutSeconds", Value::Number(timeout));
    let mut r = Map::new();
    r.insert("maxRetries", Value::Number(num(retries, "max_retries").unwrap_or(2.0)));
    r.insert(
        "initialDelaySeconds",
        Value::Number(num(retries, "initial_delay_seconds").unwrap_or(0.5)),
    );
    r.insert(
        "maxDelaySeconds",
        Value::Number(num(retries, "max_delay_seconds").unwrap_or(8.0)),
    );
    client.insert("retries", Value::Object(r));
    if let Some(h) = idempotency_header {
        client.insert("idempotencyHeader", Value::from(h));
    }
    client.insert("defaultHeaders", Value::Object(default_headers));
    client.insert(
        "omitPlatformHeaders",
        Value::Bool(bool_(settings, "omit_stainless_headers").unwrap_or(false)),
    );
    let mut q = Map::new();
    q.insert(
        "nestedFormat",
        pick(query.get("nested_format"), &["brackets", "dots"], "brackets"),
    );
    q.insert(
        "arrayFormat",
        pick(
            query.get("array_format"),
            &["comma", "repeat", "indices", "brackets"],
            "comma",
        ),
    );
    client.insert("query", Value::Object(q));
    if let Some(limits) = http_limits {
        client.insert("httpLimits", Value::Object(limits));
    }
    (Value::Object(client), diagnostics)
}
