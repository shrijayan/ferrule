//! A small JSON Schema (draft 2020-12) validator for exactly the keywords the public stainless.yml
//! schema uses.
//!
//! Anything outside [`ASSERTION_KEYWORDS`] and [`ANNOTATION_KEYWORDS`] is reported by
//! [`unsupported_keywords`], and a test runs it over the bundled schema, so a schema update that
//! needs a new keyword fails loudly instead of being silently ignored.

use crate::diagnostic::Seg;
use crate::value::{Map, Value, js_string, number_to_string, to_json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

/// Keywords this validator enforces.
pub const ASSERTION_KEYWORDS: [&str; 19] = [
    "$ref",
    "type",
    "const",
    "enum",
    "minLength",
    "maxLength",
    "pattern",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "required",
    "properties",
    "additionalProperties",
    "propertyNames",
    "items",
    "allOf",
    "anyOf",
    "oneOf",
];

/// Keywords that carry no assertion (2020-12 treats `format` as an annotation by default).
pub const ANNOTATION_KEYWORDS: [&str; 11] = [
    "$schema",
    "$defs",
    "id",
    "title",
    "description",
    "markdownDescription",
    "default",
    "examples",
    "deprecated",
    "format",
    "contentEncoding",
];

/// The keyword a [`SchemaError`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Type,
    Const,
    Enum,
    MinLength,
    MaxLength,
    Pattern,
    Minimum,
    Maximum,
    ExclusiveMinimum,
    ExclusiveMaximum,
    Required,
    AdditionalProperties,
    PropertyNames,
    AnyOf,
    OneOf,
    False,
    Deprecated,
}

impl Keyword {
    /// The keyword's name in a schema.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Keyword::Type => "type",
            Keyword::Const => "const",
            Keyword::Enum => "enum",
            Keyword::MinLength => "minLength",
            Keyword::MaxLength => "maxLength",
            Keyword::Pattern => "pattern",
            Keyword::Minimum => "minimum",
            Keyword::Maximum => "maximum",
            Keyword::ExclusiveMinimum => "exclusiveMinimum",
            Keyword::ExclusiveMaximum => "exclusiveMaximum",
            Keyword::Required => "required",
            Keyword::AdditionalProperties => "additionalProperties",
            Keyword::PropertyNames => "propertyNames",
            Keyword::AnyOf => "anyOf",
            Keyword::OneOf => "oneOf",
            Keyword::False => "false",
            Keyword::Deprecated => "deprecated",
        }
    }
}

/// Error or deprecation warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
}

/// Keyword-specific detail, such as the missing property or the allowed values.
#[derive(Debug, Clone, Default)]
pub struct ErrorParams<'s> {
    pub expected: Option<Vec<String>>,
    pub actual: Option<Value>,
    pub allowed: Option<Vec<Value>>,
    pub missing: Option<String>,
    pub unknown: Option<String>,
    pub limit: Option<f64>,
    pub pattern: Option<String>,
    /// `required`: the schema of the missing property, if declared.
    pub schema: Option<&'s Value>,
    pub renamed_to: Option<Value>,
    pub removed: bool,
    /// Set on the summary error an anyOf/oneOf reports when no branch was close.
    pub fallback: bool,
}

/// One validation error.
#[derive(Debug, Clone)]
pub struct SchemaError<'s> {
    /// Path into the validated value.
    pub path: Vec<Seg>,
    pub keyword: Keyword,
    pub severity: ErrorSeverity,
    pub message: String,
    /// The (sub)schema whose keyword failed; `None` for the `false` schema.
    pub schema: Option<&'s Map>,
    pub params: ErrorParams<'s>,
}

fn known_keyword(key: &str) -> bool {
    ASSERTION_KEYWORDS.contains(&key) || ANNOTATION_KEYWORDS.contains(&key)
}

/// Every keyword path in `schema` this validator does not understand.
#[must_use]
pub fn unsupported_keywords(schema: &Value) -> Vec<String> {
    fn visit(node: &Value, at: &str, out: &mut Vec<String>) {
        let Value::Object(map) = node else { return };
        for (key, value) in map {
            let here = format!("{at}/{key}");
            if !known_keyword(key) && !key.starts_with("x-") {
                out.push(here.clone());
            }
            match key.as_str() {
                "properties" | "$defs" => {
                    if let Value::Object(m) = value {
                        for (name, sub) in m {
                            visit(sub, &format!("{here}/{name}"), out);
                        }
                    }
                }
                "allOf" | "anyOf" | "oneOf" => {
                    if let Value::Array(items) = value {
                        for (i, sub) in items.iter().enumerate() {
                            visit(sub, &format!("{here}/{i}"), out);
                        }
                    }
                }
                "items" | "additionalProperties" | "propertyNames" => visit(value, &here, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    visit(schema, "", &mut out);
    out
}

/// Validate `value` against `root`: errors and deprecation warnings, in document order.
#[must_use]
pub fn validate<'s>(root: &'s Value, value: &Value) -> Vec<SchemaError<'s>> {
    let mut out = Vec::new();
    check(root, root, value, &[], &mut out);
    out
}

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, Option<Rc<regress::Regex>>>> = RefCell::new(HashMap::new());
}

/// A pattern compiled with the `u` flag, cached; `None` when it does not compile.
fn regex(source: &str) -> Option<Rc<regress::Regex>> {
    REGEX_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .entry(source.to_owned())
            .or_insert_with(|| regress::Regex::with_flags(source, "u").ok().map(Rc::new))
            .clone()
    })
}

/// Whether a pattern compiles as a unicode regex.
#[must_use]
pub fn pattern_compiles(source: &str) -> bool {
    regex(source).is_some()
}

/// JSON type of a value as JSON Schema names it (`integer` for whole numbers).
#[must_use]
pub fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Array(_) => "array",
        Value::Number(n) => {
            if n.is_finite() && n.fract() == 0.0 {
                "integer"
            } else {
                "number"
            }
        }
        Value::Bool(_) => "boolean",
        Value::String(_) => "string",
        Value::Object(_) => "object",
    }
}

fn type_matches(expected: &str, value: &Value) -> bool {
    let actual = json_type(value);
    actual == expected || (expected == "number" && actual == "integer")
}

/// Structural equality (`deepEqual`); key order is ignored and `NaN` is never equal.
#[must_use]
pub fn deep_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        #[expect(clippy::float_cmp, reason = "JavaScript === on numbers")]
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| deep_equal(p, q)),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| deep_equal(v, w)))
        }
        _ => false,
    }
}

/// Resolve a local `$ref` such as `#/$defs/ResourceConfig`.
///
/// # Errors
///
/// For a remote ref, or one that names nothing.
pub fn resolve_ref<'s>(root: &'s Value, reference: &str) -> Result<&'s Value, String> {
    let Some(rest) = reference.strip_prefix('#') else {
        return Err(format!("unsupported $ref {reference}: only local refs are allowed"));
    };
    let mut node: Option<&Value> = Some(root);
    for raw in rest.split('/').skip(1) {
        let seg = percent_decode(raw).replace("~1", "/").replace("~0", "~");
        node = node.and_then(|n| n.get(&seg));
    }
    match node {
        Some(v @ (Value::Bool(_) | Value::Object(_))) => Ok(v),
        _ => Err(format!("unresolvable $ref {reference}")),
    }
}

fn percent_decode(s: &str) -> String {
    if !s.contains('%') {
        return s.to_owned();
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(h) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(h);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn show(v: &Value) -> String {
    to_json(v)
}

static EMPTY_MAP: LazyLock<Map> = LazyLock::new(Map::new);

fn err<'s>(
    path: &[Seg],
    keyword: Keyword,
    schema: Option<&'s Map>,
    message: String,
    params: ErrorParams<'s>,
) -> SchemaError<'s> {
    SchemaError {
        path: path.to_vec(),
        keyword,
        severity: ErrorSeverity::Error,
        message,
        schema,
        params,
    }
}

fn check<'s>(root: &'s Value, schema: &'s Value, value: &Value, path: &[Seg], out: &mut Vec<SchemaError<'s>>) {
    let s = match schema {
        Value::Bool(true) => return,
        Value::Bool(false) => {
            out.push(err(
                path,
                Keyword::False,
                None,
                "no value is allowed here".to_owned(),
                ErrorParams::default(),
            ));
            return;
        }
        Value::Object(m) => m,
        _ => &EMPTY_MAP,
    };
    let fail = |out: &mut Vec<SchemaError<'s>>, keyword: Keyword, message: String, params: ErrorParams<'s>| {
        out.push(err(path, keyword, Some(s), message, params));
    };

    if let Some(Value::String(r)) = s.get("$ref")
        && let Ok(target) = resolve_ref(root, r)
    {
        check(root, target, value, path, out);
    }

    if let Some(t) = s.get("type") {
        let types: Vec<String> = match t {
            Value::Array(a) => a.iter().map(js_string).collect(),
            other => vec![js_string(other)],
        };
        if !types.iter().any(|t| type_matches(t, value)) {
            // Further keywords would only repeat the type mismatch.
            let actual = json_type(value);
            fail(
                out,
                Keyword::Type,
                format!("expected {}, got {actual}", types.join(" or ")),
                ErrorParams {
                    expected: Some(types),
                    actual: Some(Value::from(actual)),
                    ..Default::default()
                },
            );
            return;
        }
    }
    if let Some(c) = s.get("const")
        && !deep_equal(c, value)
    {
        fail(
            out,
            Keyword::Const,
            format!("expected {}, got {}", show(c), show(value)),
            ErrorParams {
                allowed: Some(vec![c.clone()]),
                actual: Some(value.clone()),
                ..Default::default()
            },
        );
    }
    if let Some(Value::Array(e)) = s.get("enum")
        && !e.iter().any(|x| deep_equal(x, value))
    {
        let list: Vec<String> = e.iter().map(show).collect();
        fail(
            out,
            Keyword::Enum,
            format!("expected one of {}, got {}", list.join(", "), show(value)),
            ErrorParams {
                allowed: Some(e.clone()),
                actual: Some(value.clone()),
                ..Default::default()
            },
        );
    }

    if let Value::String(v) = value {
        check_string(s, v, &mut |k, m, p| fail(out, k, m, p));
    }
    if let Value::Number(n) = value {
        check_number(s, *n, &mut |k, m, p| fail(out, k, m, p));
    }
    if let (Value::Array(items), Some(item_schema)) = (value, s.get("items")) {
        for (i, item) in items.iter().enumerate() {
            let mut p = path.to_vec();
            p.push(Seg::Index(i));
            check(root, item_schema, item, &p, out);
        }
    }
    if let Value::Object(obj) = value {
        check_object(root, s, obj, path, out);
    }

    if let Some(Value::Array(all)) = s.get("allOf") {
        for sub in all {
            check(root, sub, value, path, out);
        }
    }
    if let Some(Value::Array(_)) = s.get("anyOf") {
        check_any_of(root, s, "anyOf", value, path, out);
    }
    if let Some(Value::Array(_)) = s.get("oneOf") {
        check_any_of(root, s, "oneOf", value, path, out);
    }
}

type Fail<'a, 's> = dyn FnMut(Keyword, String, ErrorParams<'s>) + 'a;

fn check_string<'s>(s: &'s Map, value: &str, fail: &mut Fail<'_, 's>) {
    #[expect(clippy::cast_precision_loss, reason = "string lengths are far below 2^53")]
    let length = value.chars().count() as f64;
    if let Some(Value::Number(min)) = s.get("minLength")
        && length < *min
    {
        fail(
            Keyword::MinLength,
            format!("must be at least {} characters long", number_to_string(*min)),
            ErrorParams {
                limit: Some(*min),
                ..Default::default()
            },
        );
    }
    if let Some(Value::Number(max)) = s.get("maxLength")
        && length > *max
    {
        fail(
            Keyword::MaxLength,
            format!("must be at most {} characters long", number_to_string(*max)),
            ErrorParams {
                limit: Some(*max),
                ..Default::default()
            },
        );
    }
    if let Some(Value::String(pattern)) = s.get("pattern")
        && regex(pattern).is_none_or(|re| re.find(value).is_none())
    {
        fail(
            Keyword::Pattern,
            format!("{} does not match /{pattern}/", show(&Value::from(value))),
            ErrorParams {
                pattern: Some(pattern.clone()),
                ..Default::default()
            },
        );
    }
}

fn check_number<'s>(s: &'s Map, value: f64, fail: &mut Fail<'_, 's>) {
    let bound = |key: &str| match s.get(key) {
        Some(Value::Number(n)) => Some(*n),
        _ => None,
    };
    if let Some(min) = bound("minimum")
        && value < min
    {
        fail(
            Keyword::Minimum,
            format!("must be >= {}", number_to_string(min)),
            ErrorParams {
                limit: Some(min),
                ..Default::default()
            },
        );
    }
    if let Some(max) = bound("maximum")
        && value > max
    {
        fail(
            Keyword::Maximum,
            format!("must be <= {}", number_to_string(max)),
            ErrorParams {
                limit: Some(max),
                ..Default::default()
            },
        );
    }
    if let Some(min) = bound("exclusiveMinimum")
        && value <= min
    {
        fail(
            Keyword::ExclusiveMinimum,
            format!("must be > {}", number_to_string(min)),
            ErrorParams {
                limit: Some(min),
                ..Default::default()
            },
        );
    }
    if let Some(max) = bound("exclusiveMaximum")
        && value >= max
    {
        fail(
            Keyword::ExclusiveMaximum,
            format!("must be < {}", number_to_string(max)),
            ErrorParams {
                limit: Some(max),
                ..Default::default()
            },
        );
    }
}

fn check_object<'s>(root: &'s Value, s: &'s Map, value: &Map, path: &[Seg], out: &mut Vec<SchemaError<'s>>) {
    let properties = match s.get("properties") {
        Some(Value::Object(p)) => p,
        _ => &EMPTY_MAP,
    };
    if let Some(Value::Array(required)) = s.get("required") {
        for name in required {
            let name = js_string(name);
            if !value.contains_key(&name) {
                out.push(err(
                    path,
                    Keyword::Required,
                    Some(s),
                    format!("missing required key {}", show(&Value::from(name.as_str()))),
                    ErrorParams {
                        schema: properties.get(&name),
                        missing: Some(name),
                        ..Default::default()
                    },
                ));
            }
        }
    }
    for (key, child) in value {
        let mut child_path = path.to_vec();
        child_path.push(Seg::Key(key.clone()));
        if let Some(names) = s.get("propertyNames") {
            let mut name_errors = Vec::new();
            check(root, names, &Value::from(key.as_str()), &child_path, &mut name_errors);
            for e in name_errors {
                if e.severity == ErrorSeverity::Warning {
                    continue;
                }
                out.push(err(
                    &child_path,
                    Keyword::PropertyNames,
                    Some(s),
                    format!("invalid key {}: {}", show(&Value::from(key.as_str())), e.message),
                    ErrorParams::default(),
                ));
            }
        }
        if let Some(sub) = properties.get(key) {
            if let Value::Object(sub_map) = sub
                && sub_map.get("deprecated") == Some(&Value::Bool(true))
            {
                out.push(SchemaError {
                    path: child_path.clone(),
                    keyword: Keyword::Deprecated,
                    severity: ErrorSeverity::Warning,
                    schema: Some(sub_map),
                    message: format!("{} is deprecated", show(&Value::from(key.as_str()))),
                    params: ErrorParams {
                        renamed_to: sub_map.get("x-deprecated-renamed-to").cloned(),
                        removed: sub_map.get("x-deprecated-removed") == Some(&Value::Bool(true)),
                        ..Default::default()
                    },
                });
            }
            check(root, sub, child, &child_path, out);
        } else if s.get("additionalProperties") == Some(&Value::Bool(false)) {
            out.push(err(
                &child_path,
                Keyword::AdditionalProperties,
                Some(s),
                format!("unknown key {}", show(&Value::from(key.as_str()))),
                ErrorParams {
                    unknown: Some(key.clone()),
                    allowed: Some(properties.keys().map(|k| Value::from(k.as_str())).collect()),
                    ..Default::default()
                },
            ));
        } else if let Some(extra) = s.get("additionalProperties") {
            check(root, extra, child, &child_path, out);
        }
    }
}

fn is_error(e: &SchemaError<'_>) -> bool {
    e.severity == ErrorSeverity::Error
}

/// anyOf/oneOf: pass if a branch passes (exactly one for oneOf). On failure, report the errors
/// of the branch the value most plausibly meant (see the TypeScript source for the rules).
fn check_any_of<'s>(
    root: &'s Value,
    s: &'s Map,
    keyword: &str,
    value: &Value,
    path: &[Seg],
    out: &mut Vec<SchemaError<'s>>,
) {
    let Some(Value::Array(alternatives)) = s.get(keyword) else {
        return;
    };
    let kw = if keyword == "anyOf" {
        Keyword::AnyOf
    } else {
        Keyword::OneOf
    };
    let branches: Vec<Vec<SchemaError<'s>>> = alternatives
        .iter()
        .map(|sub| {
            let mut errors = Vec::new();
            check(root, sub, value, path, &mut errors);
            errors
        })
        .collect();
    let passing: Vec<&Vec<SchemaError<'s>>> = branches.iter().filter(|b| !b.iter().any(is_error)).collect();
    if kw == Keyword::OneOf && passing.len() > 1 {
        out.push(err(
            path,
            Keyword::OneOf,
            Some(s),
            format!("matches {} alternatives; exactly one is allowed", passing.len()),
            ErrorParams::default(),
        ));
        return;
    }
    if let Some(first) = passing.first() {
        out.extend((*first).iter().cloned());
        return;
    }
    let depth = path.len();
    let shallow_type = |e: &SchemaError<'_>| {
        e.path.len() == depth && (matches!(e.keyword, Keyword::Type | Keyword::False) || e.params.fallback)
    };
    let type_ok: Vec<&Vec<SchemaError<'s>>> = branches.iter().filter(|b| !b.iter().any(shallow_type)).collect();
    if type_ok.len() == 1 {
        out.extend(type_ok[0].iter().cloned());
        return;
    }
    let discriminator_miss =
        |e: &SchemaError<'_>| e.path.len() <= depth + 1 && matches!(e.keyword, Keyword::Const | Keyword::Enum);
    let mut candidates: Vec<&Vec<SchemaError<'s>>> = type_ok
        .into_iter()
        .filter(|b| !b.iter().any(discriminator_miss))
        .collect();
    let score = |b: &Vec<SchemaError<'_>>| b.iter().filter(|e| is_error(e)).map(|e| e.path.len()).max();
    let count = |b: &Vec<SchemaError<'_>>| b.iter().filter(|e| is_error(e)).count();
    // Stable, like Array.prototype.sort: deepest errors first, then fewest errors.
    candidates.sort_by(|a, b| score(b).cmp(&score(a)).then(count(a).cmp(&count(b))));
    if let Some(best) = candidates.first() {
        out.extend((*best).iter().cloned());
        return;
    }
    if let Some(allowed) = constants_of(root, alternatives) {
        let list: Vec<String> = allowed.iter().map(show).collect();
        out.push(err(
            path,
            Keyword::Enum,
            Some(s),
            format!("expected one of {}, got {}", list.join(", "), describe_value(value)),
            ErrorParams {
                allowed: Some(allowed),
                actual: Some(value.clone()),
                fallback: true,
                ..Default::default()
            },
        ));
        return;
    }
    let expected = describe_alternatives(root, alternatives);
    let actual = describe_value(value);
    // "expected an object, got an object" says nothing: name the mismatch instead.
    let message = if expected.iter().all(|e| *e == actual) {
        format!(
            "{actual} that matches none of the {} documented forms",
            alternatives.len()
        )
    } else {
        format!("expected {}, got {actual}", expected.join(" or "))
    };
    out.push(err(
        path,
        kw,
        Some(s),
        message,
        ErrorParams {
            expected: Some(expected),
            fallback: true,
            ..Default::default()
        },
    ));
}

/// The allowed values when every alternative is a `const` or `enum`, else `None`.
fn constants_of(root: &Value, alternatives: &[Value]) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    for alt in alternatives {
        let Value::Object(a) = alt else { return None };
        if let Some(c) = a.get("const") {
            out.push(c.clone());
        } else if let Some(Value::Array(e)) = a.get("enum") {
            out.extend(e.iter().cloned());
        } else if let Some(Value::String(r)) = a.get("$ref") {
            let target = resolve_ref(root, r).ok()?;
            out.extend(constants_of(root, std::slice::from_ref(target))?);
        } else if matches!(a.get("anyOf"), Some(Value::Array(_))) || matches!(a.get("oneOf"), Some(Value::Array(_))) {
            let inner = match a.get("anyOf").filter(|v| !v.is_null()).or_else(|| a.get("oneOf")) {
                Some(Value::Array(items)) => items.as_slice(),
                _ => &[],
            };
            out.extend(constants_of(root, inner)?);
        } else {
            return None;
        }
    }
    Some(out)
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Object(_) | Value::Array(_) => json_type(value).to_owned(),
        other => show(other),
    }
}

fn article(t: &str) -> String {
    let an = t.starts_with(['a', 'e', 'i', 'o', 'u']);
    format!("a{} {t}", if an { "n" } else { "" })
}

/// Short human list of what a set of alternatives accepts: consts, enums, then types.
#[must_use]
pub fn describe_alternatives(root: &Value, alternatives: &[Value]) -> Vec<String> {
    fn add(out: &mut Vec<String>, s: String) {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    fn visit(root: &Value, sub: &Value, budget: u32, out: &mut Vec<String>) {
        let s = match sub {
            Value::Bool(b) => {
                if *b {
                    add(out, "any value".to_owned());
                }
                return;
            }
            Value::Object(m) => m,
            _ => &EMPTY_MAP,
        };
        if let Some(Value::String(r)) = s.get("$ref")
            && budget > 0
            && let Ok(target) = resolve_ref(root, r)
        {
            visit(root, target, budget - 1, out);
        }
        if let Some(c) = s.get("const") {
            add(out, show(c));
        } else if let Some(Value::Array(e)) = s.get("enum") {
            for v in e {
                add(out, show(v));
            }
        } else if let Some(t) = s.get("type") {
            let types: Vec<String> = match t {
                Value::Array(a) => a.iter().map(js_string).collect(),
                other => vec![js_string(other)],
            };
            for t in types {
                add(out, article(&t));
            }
        }
        for k in ["anyOf", "oneOf"] {
            if let Some(Value::Array(inner)) = s.get(k)
                && budget > 0
            {
                for i in inner {
                    visit(root, i, budget - 1, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for sub in alternatives {
        visit(root, sub, 3, &mut out);
    }
    out
}
