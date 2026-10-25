//! stainless.yml + OpenAPI spec -> annotated spec with overlays and `x-ferrule-*` extensions.
//! Rust since ADR 0014. Pure: only [`cli`] touches the file system.

pub mod cli;
pub mod diagnostic;
pub mod editions;
pub mod extensions;
pub mod normalize;
pub mod schema;
pub mod style;
pub mod swagger2;
pub mod transforms;
pub mod unicode;
pub mod value;
pub mod yaml;
pub mod zod;

pub use value::{Map, Value};

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Every error carries a FER code, a JSON pointer into the spec or config, and a suggested fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Such as `FER0001`.
    pub code: String,
    pub severity: Severity,
    pub message: String,
    /// RFC 6901 JSON pointer, such as `/resources/users/methods/list`.
    pub pointer: String,
    pub suggestion: String,
}

/// The result of [`convert_text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertOutput {
    /// Deterministic JSON: sorted keys (except the maps in
    /// [`extensions::pagination::CONFIG_ORDER_MAPS`], which keep config order), 2-space indent,
    /// trailing newline. Empty when the config fails validation.
    pub output: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// The result of [`convert`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertResult {
    pub spec: Value,
    pub diagnostics: Vec<Diagnostic>,
}

/// Convert Swagger 2.0 to OpenAPI 3, apply `openapi.transforms`, normalise constructs Forge
/// cannot read, then annotate the result with Fern and `x-ferrule-*` extensions from `config`.
/// Pure; the inputs are never mutated.
#[must_use]
pub fn convert(spec: &Value, config: &Value) -> ConvertResult {
    let (upgraded, mut diagnostics) =
        swagger2::swagger2_to_openapi3(spec).unwrap_or_else(|| (spec.clone(), Vec::new()));
    let transformed = transforms::apply_config_transforms(&upgraded, config);
    diagnostics.extend(transformed.diagnostics);
    let (normalized, ds) = normalize::normalize(&transformed.spec);
    diagnostics.extend(ds);
    let (annotated, ds) = transforms::annotate::annotate(&normalized, config);
    diagnostics.extend(ds);
    ConvertResult {
        spec: annotated,
        diagnostics,
    }
}

/// Parse OpenAPI text (JSON or YAML) into an object (`parseSpec`).
///
/// # Errors
///
/// When the text does not parse, or is not an object.
pub fn parse_spec(text: &str) -> Result<Value, String> {
    // Plain JSON parses to the same value as YAML does; the strict reader is only faster.
    let value = match value::parse_json(text) {
        Some(v) => v,
        None => yaml::parse(text)?,
    };
    if !value.is_object() {
        return Err("OpenAPI document must be a JSON or YAML object".to_owned());
    }
    Ok(value)
}

/// Text in, deterministic JSON text out: validate the config, apply its edition, convert.
///
/// # Errors
///
/// When the spec is not a JSON or YAML object, or the YAML cannot be read (where the TypeScript
/// converter throws).
pub fn convert_text(spec_text: &str, config_text: &str) -> Result<ConvertOutput, String> {
    convert_text_with_style(spec_text, config_text, &Map::new())
}

/// [`convert_text`] that also writes `style` (from [`style::runtime_style`]) as
/// `x-ferrule-style`; an empty `style` changes nothing.
///
/// # Errors
///
/// As [`convert_text`].
pub fn convert_text_with_style(spec_text: &str, config_text: &str, style: &Map) -> Result<ConvertOutput, String> {
    let v = schema::validate::validate_config(config_text)?;
    let Some(config) = v.config else {
        return Ok(ConvertOutput {
            output: String::new(),
            diagnostics: v.diagnostics,
        });
    };
    let ed = editions::apply_edition(&config);
    let mut result = convert(&parse_spec(spec_text)?, &ed.config);
    style::apply_style(&mut result.spec, style);
    let mut diagnostics = v.diagnostics;
    diagnostics.extend(ed.diagnostics);
    diagnostics.extend(result.diagnostics);
    Ok(ConvertOutput {
        output: value::stable_stringify_keeping(&result.spec, &extensions::pagination::CONFIG_ORDER_MAPS),
        diagnostics,
    })
}
