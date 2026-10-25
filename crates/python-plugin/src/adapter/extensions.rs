//! The only reader of `x-ferrule-*` extensions (ADR 0003). Validates their shapes and turns them
//! into typed values; anything malformed or unknown is a FER diagnostic.
//!
//! Untrusted input: extensions carry spec and config text into generated Python. Every value that
//! becomes a Python identifier (names, env vars, pagination fields) is checked here against an
//! identifier pattern (FER101); every string literal the emitters print goes through
//! `py_string`/`py_fstring` (`emit::common`), and every docstring through `escape_docstring`.
//!
//! The TypeScript plugin reads the values through `JSON.parse`, so object keys are visited in
//! JavaScript order ([`js::entries`]) and a JSON `null` counts as present but `??` replaces it with
//! the default, as there.

use std::collections::HashMap;

use forge::Json;

use crate::diagnostics::{FerCode, FerruleError};
use crate::js;
use crate::model::{
    ApiKeyLocation, ArrayFormat, AuthRole, HttpAuthScheme, NestedFormat, OptType, PaginationScheme, ParamLocation,
    SdkAuth, SdkClientOpt, SdkEnvironment, SdkPackage, SdkSecurityScheme, SdkSend, SdkStyle, SendLocation,
    StreamFormat,
};
use crate::naming::{Casing, snake_case, split_words};

/// An extension value of the wrong shape; the caller builds the FER101 diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelName {
    pub name: String,
    /// Dotted module under `types`; defaults to the `snake_case` of `name`.
    pub module: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPagination {
    pub scheme: PaginationScheme,
    pub page: String,
    pub param_location: ParamLocation,
    /// Spec field name -> purpose, in JavaScript key order.
    pub request: Vec<(String, String)>,
    pub response: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStreaming {
    pub format: StreamFormat,
    pub param_discriminator: Option<String>,
    /// The components.schemas name of the event model, when the config names one.
    pub event_schema: Option<String>,
}

/// `x-ferrule-client` as the plugin reads it (ADR 0003, Phase 2 amendment).
#[derive(Debug, Clone, PartialEq)]
pub struct RawClient {
    /// As written, such as `onebusaway-sdk`.
    pub name: String,
    pub env_prefix: String,
    pub opts: Vec<SdkClientOpt>,
    pub environments: Vec<SdkEnvironment>,
    pub security_schemes: Vec<(String, SdkSecurityScheme)>,
    pub security: Vec<Vec<String>>,
    pub timeout_seconds: f64,
    pub max_retries: f64,
    pub idempotency_header: Option<String>,
    pub default_headers: Vec<(String, String)>,
    pub array_format: ArrayFormat,
    pub nested_format: NestedFormat,
}

/// A resource path as written in stainless.yml and the order of each segment among its siblings.
pub type ResourcePosition = (Vec<String>, Vec<usize>);

/// The `x-ferrule-*` extensions an operation carries.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodExtensions {
    /// Resource path as written in stainless.yml; `["$client"]` for client methods.
    pub resource: Vec<String>,
    /// Index of each path segment among its siblings; empty when the converter wrote none.
    pub resource_order: Vec<usize>,
    pub method_name: String,
    /// Index in its resource's `methods`; 0 when the converter wrote none.
    pub method_order: usize,
    /// Mirrored from the root (Forge gap G2).
    pub package: SdkPackage,
    /// Mirrored from the root (Forge gap G2); absent in old fixtures.
    pub client: Option<RawClient>,
    /// Schema name -> Python model name (Forge gap G3).
    pub models: HashMap<String, ModelName>,
    pub pagination: Option<RawPagination>,
    pub streaming: Option<RawStreaming>,
    /// Mirrored from the root (Forge gap G2): `custom_casings` by lowercase word; empty without.
    pub casings: Vec<(String, Casing)>,
    /// Mirrored from the root (Forge gap G2): every `standalone_api` resource, including ones
    /// without methods; empty without.
    pub standalone: Vec<ResourcePosition>,
    /// Mirrored from the root (Forge gap G2); today's style without it.
    pub style: SdkStyle,
}

/// Operation-level extensions this plugin reads. Root and schema ones never reach a plugin.
const KNOWN: [&str; 11] = [
    "x-ferrule-resource",
    "x-ferrule-method-name",
    "x-ferrule-method-order",
    "x-ferrule-package",
    "x-ferrule-models",
    "x-ferrule-pagination",
    "x-ferrule-streaming",
    "x-ferrule-client",
    "x-ferrule-casings",
    "x-ferrule-standalone",
    "x-ferrule-style",
];

/// The member `key` of `v` if `v` is an object (`members(v).key`), else `None` (`undefined`).
fn member<'a>(v: Option<&'a Json>, key: &str) -> Option<&'a Json> {
    match v {
        Some(obj @ Json::Object(_)) => obj.get(key),
        _ => None,
    }
}

fn as_str(v: Option<&Json>) -> Option<&str> {
    v.and_then(Json::as_str)
}

/// `v ?? fallback`: `undefined` and `null` both take the fallback.
fn or_null(v: Option<&Json>) -> Option<&Json> {
    v.filter(|j| !j.is_null())
}

/// `v === undefined || typeof v === 'string'`, and the string.
fn opt_string(v: Option<&Json>) -> Result<Option<String>, Malformed> {
    match v {
        None => Ok(None),
        Some(Json::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(Malformed),
    }
}

/// An object whose values are all strings, in JavaScript key order.
fn string_record(v: Option<&Json>) -> Option<Vec<(String, String)>> {
    let Some(Json::Object(members)) = v else { return None };
    js::entries(members)
        .into_iter()
        .map(|(k, v)| v.as_str().map(|s| (k.to_owned(), s.to_owned())))
        .collect()
}

fn is_integer(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0
}

/// A non-negative integer as an index (one beyond `usize::MAX` saturates, as `as` does).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn index(v: &Json) -> Option<usize> {
    let n = v.as_f64().filter(|&n| is_integer(n) && n >= 0.0)?;
    Some(n as usize)
}

fn ident_with(s: &str, first: impl Fn(char) -> bool, rest: impl Fn(char) -> bool) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(first) && chars.all(rest)
}

/// `/^[A-Za-z_][A-Za-z0-9_]*$/`
fn py_ident(s: &str) -> bool {
    ident_with(
        s,
        |c| c.is_ascii_alphabetic() || c == '_',
        |c| c.is_ascii_alphanumeric() || c == '_',
    )
}

/// `/^[A-Z][A-Z0-9_]*$/`: `<PREFIX>_BASE_URL` and friends.
fn env_prefix(s: &str) -> bool {
    ident_with(
        s,
        |c| c.is_ascii_uppercase(),
        |c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_',
    )
}

/// `/^[a-z_][a-z0-9_.]*$/`
fn module_name(s: &str) -> bool {
    ident_with(
        s,
        |c| c.is_ascii_lowercase() || c == '_',
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.',
    )
}

/// A stainless.yml resource key, `snake_cased` by the adapter (`trip-details`); `$client` is reserved.
fn resource_name(s: &str) -> bool {
    s == "$client"
        || ident_with(
            s,
            |c| c.is_ascii_alphabetic() || c == '_',
            |c| c.is_ascii_alphanumeric() || c == '_' || c == '-',
        )
}

/// A GitHub `owner/name`, optionally with the `#branch` stainless.yml allows (imagekit
/// `imagekit-developer/imagekit-python#master`). Returns `owner/name`, which is what Stainless
/// links. It is interpolated into docstrings, so nothing else is allowed.
fn repo_slug(s: &str) -> Option<&str> {
    let part = |p: &str, slash: bool| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') || (slash && c == '/'))
    };
    let (slug, branch) = match s.split_once('#') {
        Some((slug, branch)) => (slug, Some(branch)),
        None => (s, None),
    };
    if branch.is_some_and(|b| !part(b, true)) {
        return None;
    }
    if s.is_empty() {
        return Some(s);
    }
    let (owner, name) = slug.split_once('/')?;
    (part(owner, false) && part(name, false)).then_some(slug)
}

/// A dotted module whose segments are stainless.yml keys as written (`assistantMessage`,
/// `accounts.UrlEndpointResponse`, `trip-details.stop`). A segment that is not already a module
/// name is `snake_cased` as resource keys are, giving Stainless's `assistant_message`.
fn module_path(s: &str) -> Option<String> {
    s.split('.')
        .map(|seg| {
            if module_name(seg) {
                Some(seg.to_owned())
            } else if resource_name(seg) && seg != "$client" {
                Some(snake_case(&split_words(seg))).filter(|m| module_name(m))
            } else {
                None
            }
        })
        .collect::<Option<Vec<_>>>()
        .map(|segs| segs.join("."))
}

/// An http(s) URL with nothing that could close or interpolate into a Python string.
fn http_url(s: &str) -> bool {
    let rest = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://"));
    rest.is_some_and(|r| {
        !r.is_empty()
            && !r
                .chars()
                .any(|c| js::is_space(c) || matches!(c, '"' | '\'' | '\\' | '{' | '}'))
    })
}

/// Pagination field paths: dotted identifiers, with `[]` stepping into array items.
fn field_paths(map: &[(String, String)]) -> bool {
    map.iter()
        .all(|(path, _)| path.split('.').all(|seg| seg == "[]" || py_ident(seg)))
}

/// Pagination request fields: [`field_paths`] whose segments may also start with `$`, since they
/// are param names sent as written (knock's `query_options.$skiptoken`).
fn param_names(map: &[(String, String)]) -> bool {
    map.iter().all(|(name, _)| {
        name.split('.')
            .all(|seg| seg == "[]" || py_ident(seg.strip_prefix('$').unwrap_or(seg)))
    })
}

/// Reads the `x-ferrule-*` extensions of one operation.
///
/// # Errors
///
/// FER102 for an unknown extension, FER103 for a missing one, FER101 for a malformed one.
pub fn read_method_extensions(extensions: &[(&str, &Json)], pointer: &str) -> Result<MethodExtensions, FerruleError> {
    let mut found: Vec<(&str, &Json)> = Vec::new();
    for &(key, value) in extensions {
        if !key.starts_with("x-ferrule-") {
            continue;
        }
        if !KNOWN.contains(&key) {
            return Err(FerruleError::new(
                FerCode::Fer102,
                format!("{pointer}/{key}"),
                format!("unknown extension {key}"),
                format!("remove it, or upgrade the plugin to one that knows {key}"),
            ));
        }
        match found.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => found.push((key, value)),
        }
    }
    let get = |key: &str| found.iter().find(|(k, _)| *k == key).map(|&(_, v)| v);
    let bad = |key: &str, shape: &str| {
        let has = get(key).is_some();
        FerruleError::new(
            if has { FerCode::Fer101 } else { FerCode::Fer103 },
            format!("{pointer}/{key}"),
            if has {
                format!("{key} is not {shape}")
            } else {
                format!("missing {key}")
            },
            format!("regenerate the annotated spec with ferrule-convert so {key} is {shape}"),
        )
    };

    let (path, resource_order) =
        read_resource(get("x-ferrule-resource")).map_err(|shape| bad("x-ferrule-resource", shape))?;

    let method_name = match get("x-ferrule-method-name") {
        Some(Json::String(s)) if py_ident(s) => s.clone(),
        _ => return Err(bad("x-ferrule-method-name", "a Python identifier")),
    };
    let method_order = match or_null(get("x-ferrule-method-order")) {
        None => 0,
        Some(v) => index(v).ok_or_else(|| bad("x-ferrule-method-order", "a non-negative integer"))?,
    };

    let raw_pkg = get("x-ferrule-package");
    let package = match (as_str(member(raw_pkg, "name")), as_str(member(raw_pkg, "repo"))) {
        (Some(name), Some(repo)) if py_ident(name) && repo_slug(repo).is_some() => SdkPackage {
            name: name.to_owned(),
            repo: repo_slug(repo).unwrap_or_default().to_owned(),
        },
        _ => {
            return Err(bad(
                "x-ferrule-package",
                "{ name: string, repo: \"owner/name\" or \"\" }",
            ));
        }
    };

    let models = read_models(or_null(get("x-ferrule-models")))
        .map_err(|Malformed| bad("x-ferrule-models", "Record<schema, { name, module? }>"))?;

    let pagination = get("x-ferrule-pagination")
        .map(|raw| read_pagination(raw).ok_or_else(|| bad("x-ferrule-pagination", PAGINATION_SHAPE)))
        .transpose()?;

    let streaming = get("x-ferrule-streaming")
        .map(|raw| read_streaming(raw).ok_or_else(|| bad("x-ferrule-streaming", STREAMING_SHAPE)))
        .transpose()?;

    let client = match get("x-ferrule-client") {
        None => None,
        Some(raw) => Some(read_client(raw).map_err(|Malformed| bad("x-ferrule-client", "the ADR 0003 client shape"))?),
    };

    let casings = read_casings(get("x-ferrule-casings")).map_err(|Malformed| {
        bad(
            "x-ferrule-casings",
            "Record<word, { initialism: boolean } | { pascal: identifier }>",
        )
    })?;
    let standalone = read_standalone(get("x-ferrule-standalone"))
        .map_err(|Malformed| bad("x-ferrule-standalone", "{ path: string[], order: number[] }[]"))?;
    let style = read_style(get("x-ferrule-style")).map_err(|Malformed| bad("x-ferrule-style", STYLE_SHAPE))?;

    Ok(MethodExtensions {
        resource: path,
        resource_order,
        method_name,
        method_order,
        package,
        client,
        models,
        pagination,
        streaming,
        casings,
        standalone,
        style,
    })
}

const PAGINATION_SHAPE: &str =
    "{ scheme, page, paramLocation?, request: Record<string,string>, response: Record<string,string>, headers? }";

const STREAMING_SHAPE: &str = "{ format: \"sse\" | \"jsonl\", paramDiscriminator, eventSchema? }";

const STYLE_SHAPE: &str = "{ sentinel?: \"omit\" | \"NOT_GIVEN\", resources?: \"lazy\" | \"eager\", paths?: \"path_template\" | \"fstring\", multipart?: \"deepcopy_with_paths\" | \"deepcopy_minimal\" }";

/// `x-ferrule-style`: every key optional, today's spelling when absent; anything else is FER101.
fn read_style(raw: Option<&Json>) -> Result<SdkStyle, Malformed> {
    let mut style = SdkStyle::default();
    let Some(raw) = raw else { return Ok(style) };
    let Json::Object(members) = raw else {
        return Err(Malformed);
    };
    for (key, value) in js::entries(members) {
        let (slot, today, older) = match key {
            "sentinel" => (&mut style.omit, "omit", "NOT_GIVEN"),
            "resources" => (&mut style.lazy_resources, "lazy", "eager"),
            "paths" => (&mut style.path_template, "path_template", "fstring"),
            "multipart" => (
                &mut style.deepcopy_with_paths,
                "deepcopy_with_paths",
                "deepcopy_minimal",
            ),
            _ => return Err(Malformed),
        };
        *slot = match value.as_str() {
            Some(v) if v == today => true,
            Some(v) if v == older => false,
            _ => return Err(Malformed),
        };
    }
    Ok(style)
}

/// `x-ferrule-standalone`: each entry as `x-ferrule-resource` reads.
fn read_standalone(raw: Option<&Json>) -> Result<Vec<ResourcePosition>, Malformed> {
    match raw {
        None => Ok(Vec::new()),
        Some(Json::Array(items)) => items
            .iter()
            .map(|item| read_resource(Some(item)).map_err(|_| Malformed))
            .collect(),
        Some(_) => Err(Malformed),
    }
}

/// `x-ferrule-casings`. A `pascal` spelling is spliced into class names, so it must be
/// identifier characters, led by a digit only when its word is (`3ds` -> `3DS`, never `qa` ->
/// `123`); FER101 otherwise. Words are lowercase alphanumeric.
fn read_casings(raw: Option<&Json>) -> Result<Vec<(String, Casing)>, Malformed> {
    let Some(raw) = raw else { return Ok(Vec::new()) };
    let Json::Object(members) = raw else {
        return Err(Malformed);
    };
    js::entries(members)
        .into_iter()
        .map(|(word, v)| {
            if word.is_empty() || !word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
                return Err(Malformed);
            }
            let casing = match (member(Some(v), "initialism"), member(Some(v), "pascal")) {
                (Some(Json::Bool(b)), None) => Casing::Initialism(*b),
                (None, Some(Json::String(p)))
                    if py_ident(p)
                        || (word.starts_with(|c: char| c.is_ascii_digit()) && py_ident(&format!("_{p}"))) =>
                {
                    Casing::Pascal(p.clone())
                }
                _ => return Err(Malformed),
            };
            Ok((word.to_owned(), casing))
        })
        .collect()
}

/// `x-ferrule-resource`: the path and the order of each segment; the shape it lacks otherwise.
fn read_resource(raw: Option<&Json>) -> Result<(Vec<String>, Vec<usize>), &'static str> {
    let path: Option<Vec<String>> = match member(raw, "path") {
        Some(Json::Array(items)) if !items.is_empty() => items
            .iter()
            .map(|p| p.as_str().filter(|s| resource_name(s)).map(str::to_owned))
            .collect(),
        _ => None,
    };
    let path = path.ok_or("{ path: string[] } of resource names")?;
    let order_shape = "{ path: string[], order?: number[] } of equal length";
    let order = match member(raw, "order") {
        None => Vec::new(),
        Some(Json::Array(items)) if items.len() == path.len() => {
            items.iter().map(index).collect::<Option<_>>().ok_or(order_shape)?
        }
        Some(_) => return Err(order_shape),
    };
    Ok((path, order))
}

/// `x-ferrule-models` (`undefined` or `null` for none).
fn read_models(raw: Option<&Json>) -> Result<HashMap<String, ModelName>, Malformed> {
    let mut models = HashMap::new();
    match raw {
        None => {}
        Some(Json::Object(members)) => {
            for (schema, value) in js::entries(members) {
                let name = as_str(member(Some(value), "name"))
                    .filter(|n| py_ident(n))
                    .ok_or(Malformed)?;
                let module = match member(Some(value), "module") {
                    None => None,
                    Some(Json::String(m)) => Some(module_path(m).ok_or(Malformed)?),
                    Some(_) => return Err(Malformed),
                };
                models.insert(
                    schema.to_owned(),
                    ModelName {
                        name: name.to_owned(),
                        module,
                    },
                );
            }
        }
        Some(_) => return Err(Malformed),
    }
    Ok(models)
}

fn read_pagination(raw: &Json) -> Option<RawPagination> {
    let v = Some(raw);
    let scheme = match as_str(member(v, "scheme"))? {
        "cursor" => PaginationScheme::Cursor,
        "cursor_id" => PaginationScheme::CursorId,
        "cursor_url" => PaginationScheme::CursorUrl,
        "offset" => PaginationScheme::Offset,
        "page_number" => PaginationScheme::PageNumber,
        "fake_page" => PaginationScheme::FakePage,
        _ => return None,
    };
    let page = as_str(member(v, "page")).filter(|p| py_ident(p))?.to_owned();
    let request = string_record(member(v, "request"))?;
    let response = string_record(member(v, "response"))?;
    if !param_names(&request) || !field_paths(&response) {
        return None;
    }
    let headers = match member(v, "headers") {
        None => Vec::new(),
        h => string_record(h)?,
    };
    let param_location = match member(v, "paramLocation") {
        None => ParamLocation::Query,
        Some(Json::String(s)) if s == "query" => ParamLocation::Query,
        Some(Json::String(s)) if s == "body" => ParamLocation::Body,
        Some(_) => return None,
    };
    Some(RawPagination {
        scheme,
        page,
        param_location,
        request,
        response,
        headers,
    })
}

fn read_streaming(raw: &Json) -> Option<RawStreaming> {
    let v = Some(raw);
    let format = match as_str(member(v, "format"))? {
        "sse" => StreamFormat::Sse,
        "jsonl" => StreamFormat::Jsonl,
        _ => return None,
    };
    let param_discriminator = match member(v, "paramDiscriminator") {
        None | Some(Json::Null) => None,
        Some(Json::String(s)) => Some(s.clone()),
        Some(_) => return None,
    };
    let event_schema = opt_string(member(v, "eventSchema")).ok()?;
    Some(RawStreaming {
        format,
        param_discriminator,
        event_schema,
    })
}

/// `n` as an `i64` when it is an integer JavaScript holds exactly (at most 2^53 in magnitude).
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn exact_integer(n: f64) -> Option<i64> {
    (is_integer(n) && n.abs() <= 9_007_199_254_740_992.0).then_some(n as i64)
}

/// A JSON value as `serde_json` holds it, with object keys in JavaScript order and integral
/// numbers as integers (JavaScript has one number type).
fn to_value(j: &Json) -> serde_json::Value {
    use serde_json::Value;
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => exact_integer(*n).map_or_else(
            || serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number),
            Value::from,
        ),
        Json::String(s) => Value::String(s.clone()),
        Json::Array(items) => Value::Array(items.iter().map(to_value).collect()),
        Json::Object(members) => Value::Object(
            js::entries(members)
                .into_iter()
                .map(|(k, v)| (k.to_owned(), to_value(v)))
                .collect(),
        ),
    }
}

fn read_opt(v: &Json) -> Result<SdkClientOpt, Malformed> {
    let m = Some(v);
    // The `snake_case` of the config key, as Stainless names it (stagehand `BROWSERBASE_API_KEY`
    // -> `browserbase_api_key`, papr `X-Session-Token` -> `x_session_token`).
    let name = as_str(member(m, "name"))
        .map(|n| snake_case(&split_words(n)))
        .filter(|n| py_ident(n))
        .ok_or(Malformed)?;
    let ty = match as_str(member(m, "type")) {
        Some("string") => OptType::String,
        Some("number") => OptType::Number,
        Some("integer") => OptType::Integer,
        Some("boolean") => OptType::Boolean,
        _ => return Err(Malformed),
    };
    let nullable = member(m, "nullable").and_then(Json::as_bool).ok_or(Malformed)?;
    let read_env = opt_string(member(m, "readEnv"))?;
    if read_env.as_deref().is_some_and(|e| !py_ident(e)) {
        return Err(Malformed);
    }
    let description = opt_string(member(m, "description"))?;
    let auth = match member(m, "auth") {
        None => None,
        a => {
            let scheme = as_str(member(a, "securityScheme")).ok_or(Malformed)?.to_owned();
            let role = match as_str(member(a, "role")) {
                Some("value") => AuthRole::Value,
                Some("username") => AuthRole::Username,
                Some("password") => AuthRole::Password,
                Some("client_id") => AuthRole::ClientId,
                Some("client_secret") => AuthRole::ClientSecret,
                _ => return Err(Malformed),
            };
            Some(SdkAuth {
                security_scheme: scheme,
                role,
            })
        }
    };
    let send = match member(m, "send") {
        None => None,
        s => {
            let location = match as_str(member(s, "in")) {
                Some("header") => SendLocation::Header,
                Some("query") => SendLocation::Query,
                Some("body") => SendLocation::Body,
                Some("path") => SendLocation::Path,
                _ => return Err(Malformed),
            };
            let name = as_str(member(s, "name")).ok_or(Malformed)?.to_owned();
            Some(SdkSend { location, name })
        }
    };
    Ok(SdkClientOpt {
        name,
        ty,
        nullable,
        default: member(m, "default").map(to_value),
        read_env,
        description,
        auth,
        send,
    })
}

fn read_scheme(v: &Json) -> Result<SdkSecurityScheme, Malformed> {
    let m = Some(v);
    match as_str(member(m, "type")) {
        Some("http") => {
            let scheme = match as_str(member(m, "scheme")) {
                Some("bearer") => HttpAuthScheme::Bearer,
                Some("digest") => HttpAuthScheme::Digest,
                Some("basic") => HttpAuthScheme::Basic,
                _ => return Err(Malformed),
            };
            Ok(SdkSecurityScheme::Http {
                scheme,
                keyword: opt_string(member(m, "keyword"))?,
            })
        }
        Some("apiKey") => {
            let location = match as_str(member(m, "in")) {
                Some("header") => ApiKeyLocation::Header,
                Some("query") => ApiKeyLocation::Query,
                Some("cookie") => ApiKeyLocation::Cookie,
                _ => return Err(Malformed),
            };
            let name = as_str(member(m, "name")).ok_or(Malformed)?.to_owned();
            Ok(SdkSecurityScheme::ApiKey { location, name })
        }
        Some("oauth2") => Ok(SdkSecurityScheme::OAuth2),
        Some("openIdConnect") => Ok(SdkSecurityScheme::OpenIdConnect),
        _ => Err(Malformed),
    }
}

/// Reads `x-ferrule-client`.
///
/// # Errors
///
/// `Err(Malformed)` for a malformed value; the caller builds the diagnostic.
pub fn read_client(v: &Json) -> Result<RawClient, Malformed> {
    let m = Some(v);
    let name = as_str(member(m, "name")).ok_or(Malformed)?.to_owned();
    let prefix = as_str(member(m, "envPrefix"))
        .filter(|p| env_prefix(p))
        .ok_or(Malformed)?
        .to_owned();
    let Some(Json::Array(opts)) = member(m, "opts") else {
        return Err(Malformed);
    };
    let Some(Json::Array(environments)) = member(m, "environments") else {
        return Err(Malformed);
    };
    let Some(Json::Object(schemes)) = member(m, "securitySchemes") else {
        return Err(Malformed);
    };
    let Some(Json::Array(security)) = member(m, "security") else {
        return Err(Malformed);
    };
    let timeout_seconds = member(m, "timeoutSeconds").and_then(Json::as_f64).ok_or(Malformed)?;
    let idempotency_header = opt_string(member(m, "idempotencyHeader"))?;
    let default_headers = match member(m, "defaultHeaders") {
        None => None,
        h => Some(string_record(h).ok_or(Malformed)?),
    };
    let environments = environments
        .iter()
        .map(
            |e| match (as_str(member(Some(e), "name")), as_str(member(Some(e), "url"))) {
                (Some(n), Some(url)) if http_url(url) => Ok(SdkEnvironment {
                    name: n.to_owned(),
                    url: url.to_owned(),
                }),
                _ => Err(Malformed),
            },
        )
        .collect::<Result<Vec<_>, Malformed>>()?;
    let security = security
        .iter()
        .map(|alt| match alt {
            Json::Object(members) => Ok(js::entries(members).into_iter().map(|(k, _)| k.to_owned()).collect()),
            _ => Err(Malformed),
        })
        .collect::<Result<Vec<Vec<String>>, Malformed>>()?;
    let retries = member(m, "retries");
    let query = member(m, "query");
    let max_retries = match member(retries, "maxRetries") {
        None => 2.0,
        Some(Json::Number(n)) => *n,
        Some(_) => return Err(Malformed),
    };
    let array_format = match member(query, "arrayFormat") {
        None => ArrayFormat::Repeat,
        Some(j) => match j.as_str() {
            Some("comma") => ArrayFormat::Comma,
            Some("repeat") => ArrayFormat::Repeat,
            Some("indices") => ArrayFormat::Indices,
            Some("brackets") => ArrayFormat::Brackets,
            _ => return Err(Malformed),
        },
    };
    let nested_format = match member(query, "nestedFormat") {
        None => NestedFormat::Brackets,
        Some(j) => match j.as_str() {
            Some("brackets") => NestedFormat::Brackets,
            Some("dots") => NestedFormat::Dots,
            _ => return Err(Malformed),
        },
    };
    let opts = opts.iter().map(read_opt).collect::<Result<Vec<_>, Malformed>>()?;
    let security_schemes = js::entries(schemes)
        .into_iter()
        .map(|(k, s)| read_scheme(s).map(|s| (k.to_owned(), s)))
        .collect::<Result<Vec<_>, Malformed>>()?;
    Ok(RawClient {
        name,
        env_prefix: prefix,
        opts,
        environments,
        security_schemes,
        security,
        timeout_seconds,
        max_retries,
        idempotency_header,
        default_headers: default_headers.unwrap_or_default(),
        array_format,
        nested_format,
    })
}

#[cfg(test)]
mod tests;
