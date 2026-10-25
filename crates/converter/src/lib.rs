//! stainless.yml + OpenAPI spec -> annotated spec with overlays and `x-ferrule-*` extensions.
//! Port of `converter/` (ADR 0014). Pure: only [`cli`] touches the file system.

pub mod cli;
pub mod diagnostic;
pub mod editions;
pub mod schema;
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
    /// Deterministic JSON: sorted keys, 2-space indent, trailing newline. Empty when the config
    /// fails validation.
    pub output: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Text in, deterministic JSON text out: validate the config, apply its edition, convert.
/// Byte-identical to `convertText` in `converter/src/index.ts`.
///
/// # Errors
///
/// When the spec is not a JSON or YAML object.
///
/// # Panics
///
/// Until the port lands.
pub fn convert_text(spec_text: &str, config_text: &str) -> Result<ConvertOutput, String> {
    let _ = (spec_text, config_text);
    todo!("port of convertText")
}
