//! `FER11xx`: stainless.yml validation. FER1101, FER1102 and FER1103 (editions) live in
//! [`crate::editions`].
//! Codes are stable; never reuse a retired one.

use super::config::config_issues;
use super::json_schema::{ErrorSeverity, Keyword, SchemaError, validate};
use super::stainless_config_schema;
use super::suggest::closest;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::value::{Map, Value, js_string, to_json};
use crate::yaml::{Document, NodeKind, Pos, parse_document};
use crate::{Diagnostic, Severity};

/// The text is not valid YAML.
pub const YAML_SYNTAX: &str = "FER1100";
/// A value has the wrong JSON type.
pub const TYPE: &str = "FER1110";
/// A required key is missing.
pub const REQUIRED: &str = "FER1111";
/// A key the schema does not allow (`additionalProperties: false`).
pub const UNKNOWN_KEY: &str = "FER1112";
/// A value outside the allowed `enum`/`const` set.
pub const NOT_ALLOWED: &str = "FER1113";
/// A string fails `minLength`, `maxLength` or `pattern`.
pub const STRING_FORMAT: &str = "FER1114";
/// A number outside `minimum`/`maximum`.
pub const RANGE: &str = "FER1115";
/// A value that matches none (or, for oneOf, several) of the alternatives.
pub const ALTERNATIVES: &str = "FER1116";
/// A map key fails the schema's `propertyNames`.
pub const KEY_NAME: &str = "FER1117";
/// A deprecated key is used (warning).
pub const DEPRECATED: &str = "FER1118";
/// Schema-valid, but not in the shape Ferrule's typed reader expects.
pub const UNSUPPORTED_SHAPE: &str = "FER1119";

/// The result of [`validate_config`].
#[derive(Debug, Clone)]
pub struct ConfigValidation {
    /// The typed config, or `None` when the YAML or Ferrule's typed reader rejects it.
    pub config: Option<Value>,
    /// The plain value parsed from YAML (aliases resolved), or `None` on a syntax error.
    pub value: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
}

struct Located {
    diagnostic: Diagnostic,
    segments: Vec<Seg>,
    /// Point at the key rather than the value.
    on_key: bool,
}

fn at(pos: Option<Pos>) -> String {
    pos.map_or_else(String::new, |p| format!(" (line {}, column {})", p.line, p.col))
}

/// Parse and validate stainless.yml text against the public JSON Schema. Pure.
///
/// # Errors
///
/// Only where the TypeScript `validateConfig` throws: an alias to an undefined anchor.
pub fn validate_config(text: &str) -> Result<ConfigValidation, String> {
    let doc = parse_document(text);
    if !doc.errors.is_empty() {
        return Ok(ConfigValidation {
            config: None,
            value: None,
            diagnostics: doc
                .errors
                .iter()
                .map(|e| {
                    Diagnostic::new(
                        YAML_SYNTAX,
                        Severity::Error,
                        format!(
                            "invalid YAML: {}{}",
                            e.message.split('\n').next().unwrap_or(""),
                            at(Some(e.pos))
                        ),
                        "",
                        "Fix the YAML syntax at the reported position.",
                    )
                })
                .collect(),
        });
    }
    let value = doc.to_value()?;
    let value = if value.is_null() { Value::object() } else { value };
    let mut diagnostics: Vec<Diagnostic> = to_diagnostics(&validate(stainless_config_schema(), &value))
        .into_iter()
        .filter(|d| !is_legacy_transformation(&value, &d.segments))
        .map(|mut d| {
            let pos = offset_of(&doc, &d.segments, d.on_key);
            d.diagnostic.message.push_str(&at(pos));
            d.diagnostic
        })
        .collect();
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Ok(ConfigValidation {
            config: None,
            value: Some(value),
            diagnostics,
        });
    }
    let issues = config_issues(&value);
    if !issues.is_empty() {
        for issue in issues {
            let pos = offset_of(&doc, &issue.path, false);
            diagnostics.push(Diagnostic::new(
                UNSUPPORTED_SHAPE,
                Severity::Error,
                format!("Ferrule cannot read this value yet: {}{}", issue.message, at(pos)),
                to_json_pointer(&issue.path),
                "Open an issue with this config; the public schema accepts it but Ferrule does not.",
            ));
        }
        return Ok(ConfigValidation {
            config: None,
            value: Some(value),
            diagnostics,
        });
    }
    Ok(ConfigValidation {
        config: Some(value.clone()),
        value: Some(value),
        diagnostics,
    })
}

/// Legacy commands the deprecated `openapi.transformations` list still carries in real configs
/// (e.g. ogx's `oneOfToAnyOf`). The public schema no longer lists them; the transforms engine
/// checks them instead.
const LEGACY_TRANSFORMATIONS: [&str; 1] = ["oneOfToAnyOf"];

fn is_legacy_transformation(value: &Value, segments: &[Seg]) -> bool {
    let [Seg::Key(a), Seg::Key(b), Seg::Index(i), ..] = segments else {
        return false;
    };
    if a != "openapi" || b != "transformations" {
        return false;
    }
    let entry = value
        .get("openapi")
        .and_then(|o| o.get("transformations"))
        .and_then(|l| l.as_array()?.get(*i));
    let command = entry.and_then(|e| e.get("command")).and_then(Value::as_str);
    command.is_some_and(|c| LEGACY_TRANSFORMATIONS.contains(&c))
}

/// Validate an already-parsed value (no source positions).
#[must_use]
pub fn validate_config_value(value: &Value) -> Vec<Diagnostic> {
    to_diagnostics(&validate(stainless_config_schema(), value))
        .into_iter()
        .filter(|d| !is_legacy_transformation(value, &d.segments))
        .map(|d| d.diagnostic)
        .collect()
}

fn label(path: &[Seg]) -> String {
    if path.is_empty() {
        return "config".to_owned();
    }
    path.iter().map(ToString::to_string).collect::<Vec<_>>().join(".")
}

fn article(t: &str) -> String {
    let an = t.starts_with(['a', 'e', 'i', 'o', 'u']);
    format!("a{} {t}", if an { "n" } else { "" })
}

#[expect(
    clippy::too_many_lines,
    reason = "one arm per JSON Schema keyword, as in the TypeScript switch"
)]
fn to_diagnostics(errors: &[SchemaError<'_>]) -> Vec<Located> {
    errors
        .iter()
        .map(|e| {
            let severity = match e.severity {
                ErrorSeverity::Error => Severity::Error,
                ErrorSeverity::Warning => Severity::Warning,
            };
            let pointer = to_json_pointer(&e.path);
            let p = &e.params;
            let parent = &e.path[..e.path.len().saturating_sub(1)];
            let last = e.path.last().map(ToString::to_string).unwrap_or_default();
            let (code, message, suggestion, on_key) = match e.keyword {
                Keyword::Type => (
                    TYPE,
                    format!("{}: {}", label(&e.path), e.message),
                    format!(
                        "Change the value to {}.",
                        article(&p.expected.clone().unwrap_or_default().join(" or "))
                    ),
                    false,
                ),
                Keyword::Required => {
                    let name = p.missing.clone().unwrap_or_default();
                    let hint = p.schema.and_then(first_sentence_of);
                    let hint = hint.map_or_else(String::new, |h| format!(" ({h})"));
                    (
                        REQUIRED,
                        format!("{}: {}", label(&e.path), e.message),
                        format!("Add `{name}`{hint}."),
                        false,
                    )
                }
                Keyword::AdditionalProperties => {
                    let name = p.unknown.clone().unwrap_or_default();
                    let allowed: Vec<String> = p.allowed.iter().flatten().map(js_string).collect();
                    let refs: Vec<&str> = allowed.iter().map(String::as_str).collect();
                    let suggestion = closest(&name, &refs).map_or_else(
                        || {
                            let quoted: Vec<String> = allowed.iter().map(|k| format!("`{k}`")).collect();
                            format!("Remove `{name}`; allowed keys are {}.", quoted.join(", "))
                        },
                        |near| format!("Did you mean `{near}`?"),
                    );
                    (
                        UNKNOWN_KEY,
                        format!("{}: {}", label(parent), e.message),
                        suggestion,
                        true,
                    )
                }
                Keyword::Const | Keyword::Enum => {
                    let all = p.allowed.clone().unwrap_or_default();
                    let allowed: Vec<String> = all.iter().map(to_json).collect();
                    let strings: Vec<&str> = all.iter().filter_map(Value::as_str).collect();
                    let near = match &p.actual {
                        Some(Value::String(a)) => closest(a, &strings),
                        _ => None,
                    };
                    let suggestion = near.map_or_else(
                        || format!("Use one of {}.", allowed.join(", ")),
                        |n| format!("Did you mean \"{n}\"?"),
                    );
                    (
                        NOT_ALLOWED,
                        format!("{}: {}", label(&e.path), e.message),
                        suggestion,
                        false,
                    )
                }
                Keyword::MinLength | Keyword::MaxLength | Keyword::Pattern => (
                    STRING_FORMAT,
                    format!("{}: {}", label(&e.path), e.message),
                    describe(e.schema).unwrap_or_else(|| "Change the string to the documented format.".to_owned()),
                    false,
                ),
                Keyword::Minimum | Keyword::Maximum | Keyword::ExclusiveMinimum | Keyword::ExclusiveMaximum => (
                    RANGE,
                    format!("{}: {}", label(&e.path), e.message),
                    "Use a number inside the allowed range.".to_owned(),
                    false,
                ),
                Keyword::PropertyNames => (
                    KEY_NAME,
                    format!("{}: {}", label(parent), e.message),
                    "Rename the key to one the schema allows.".to_owned(),
                    true,
                ),
                Keyword::Deprecated => {
                    let suggestion = match &p.renamed_to {
                        Some(Value::String(to)) => format!("Rename `{last}` to `{to}`."),
                        _ => format!("Remove `{last}`; it no longer has any effect."),
                    };
                    (
                        DEPRECATED,
                        format!("{}: {}", label(&e.path), e.message),
                        suggestion,
                        true,
                    )
                }
                Keyword::AnyOf | Keyword::OneOf | Keyword::False => (
                    ALTERNATIVES,
                    format!("{}: {}", label(&e.path), e.message),
                    describe(e.schema)
                        .unwrap_or_else(|| "Change the value to one of the forms the schema documents.".to_owned()),
                    false,
                ),
            };
            Located {
                diagnostic: Diagnostic::new(code, severity, message, pointer, suggestion),
                segments: e.path.clone(),
                on_key,
            }
        })
        .collect()
}

/// The YAML source position for a path: the key node when `on_key`, else the value node.
fn offset_of(doc: &Document, segments: &[Seg], on_key: bool) -> Option<Pos> {
    let mut node = doc.root;
    let mut last = node.map(|n| doc.node(n).pos);
    for (i, seg) in segments.iter().enumerate() {
        let Some(mut current) = node else { return last };
        current = doc.resolve(current);
        let mut next = None;
        let mut key = None;
        match &doc.node(current).kind {
            NodeKind::Map(pairs) => {
                let wanted = seg.to_string();
                if let Some(&(k, v)) = pairs.iter().find(|&&(k, _)| match &doc.node(k).kind {
                    NodeKind::Scalar(kv) => js_string(kv) == wanted,
                    _ => false,
                }) {
                    next = Some(v);
                    key = Some(k);
                }
            }
            NodeKind::Seq(items) => {
                if let Seg::Index(ix) = seg {
                    next = items.get(*ix).copied();
                }
            }
            _ => {}
        }
        if on_key
            && i == segments.len() - 1
            && let Some(k) = key
        {
            return Some(doc.node(k).pos);
        }
        let Some(n) = next else {
            return key.map_or(last, |k| Some(doc.node(k).pos));
        };
        node = Some(n);
        last = Some(doc.node(n).pos);
    }
    last
}

fn describe(schema: Option<&Map>) -> Option<String> {
    first_sentence(schema?)
}

fn first_sentence_of(schema: &Value) -> Option<String> {
    first_sentence(schema.as_object()?)
}

/// The first sentence of a schema's description, without its full stop, if short.
fn first_sentence(schema: &Map) -> Option<String> {
    let Some(Value::String(description)) = schema.get("description") else {
        return None;
    };
    // split(/(?<=\.)\s|\n/)[0]
    let mut end = description.len();
    let mut prev = None;
    for (i, c) in description.char_indices() {
        if c == '\n' || (prev == Some('.') && crate::value::is_js_space(c)) {
            end = i;
            break;
        }
        prev = Some(c);
    }
    let sentence = description[..end].trim_matches(crate::value::is_js_space);
    let sentence = sentence.strip_suffix('.').unwrap_or(sentence);
    if sentence.is_empty() || crate::value::utf16_len(sentence) > 160 {
        return None;
    }
    Some(sentence.to_owned())
}
