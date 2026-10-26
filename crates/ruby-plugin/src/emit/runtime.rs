//! What the Ruby runtime template needs from the plugin (`runtime/ruby`, rendered by
//! `ferrule_runtime.render_ruby_files`): its options and the text of its three `@example` slots.

use std::fmt::Write as _;

use ferrule_sdk_model::model::ArrayFormat;

use super::Gen;
use crate::naming::{snake, split_words};
use crate::printer::literal;
use crate::tree::Decl;

/// `RubyOptions` of the runtime renderer, field for field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub gem_name: String,
    pub module_name: String,
    /// `comma` or `repeat`.
    pub array_format: String,
    /// `headers` or `query`.
    pub auth_in: String,
    pub base_model_example: String,
    pub enum_example: String,
    pub union_example: String,
}

impl RuntimeOptions {
    /// The options as the JSON object `ferrule generate` passes to `RubyOptions(**options)`.
    #[must_use]
    pub fn to_json(&self) -> String {
        let value = serde_json::json!({
            "gem_name": self.gem_name,
            "module_name": self.module_name,
            "array_format": self.array_format,
            "auth_in": self.auth_in,
            "base_model_example": self.base_model_example,
            "enum_example": self.enum_example,
            "union_example": self.union_example,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// `BaseModel#deconstruct_keys`'s example: the first named model class and its first three
/// fields.
fn base_model_example(g: &Gen<'_>) -> String {
    let Some(c) = g.models.iter().find_map(|f| match &f.decl {
        Decl::Class(c) if c.r.aliased && !c.params && c.r.segs.len() == 1 => Some(c),
        _ => None,
    }) else {
        return String::new();
    };
    let var = snake(&split_words(c.r.name()));
    let mut out = String::from("        #\n        # @example\n");
    let _ = writeln!(out, "        #   # `{var}` is a `{}`", c.r.code(&g.m));
    let fields: Vec<String> = c.fields.iter().take(3).map(|f| format!("{0}: {0}", f.name)).collect();
    let _ = writeln!(out, "        #   {var} => {{");
    let _ = writeln!(out, "        #     {}", fields.join(",\n        #     "));
    out.push_str("        #   }\n");
    out
}

/// `Enum`'s examples: the first named enum and its first three members.
fn enum_example(g: &Gen<'_>) -> String {
    let Some(e) = g.models.iter().find_map(|f| match &f.decl {
        Decl::Enum(e) if e.r.aliased && e.r.segs.len() == 1 => Some(e),
        _ => None,
    }) else {
        return String::new();
    };
    let var = snake(&split_words(e.r.name()));
    let code = e.r.code(&g.m);
    let mut out = String::from("      #\n      # @example\n");
    let _ = writeln!(out, "      #   # `{var}` is a `{code}`");
    let _ = writeln!(out, "      #   case {var}");
    for m in e.members.iter().take(3) {
        let _ = writeln!(out, "      #   when {code}::{}\n      #     # ...", m.constant);
    }
    let _ = writeln!(out, "      #   else\n      #     puts({var})\n      #   end");
    out.push_str("      #\n      # @example\n");
    let _ = writeln!(out, "      #   case {var}");
    for m in e.members.iter().take(3) {
        let _ = writeln!(out, "      #   in {}\n      #     # ...", literal(&m.value));
    }
    let _ = writeln!(out, "      #   else\n      #     puts({var})\n      #   end");
    out
}

/// The runtime options of `g`.
#[must_use]
pub fn runtime_options(g: &Gen<'_>) -> RuntimeOptions {
    let array_format = match g.client.settings.as_ref().map(|s| s.array_format) {
        Some(ArrayFormat::Repeat) => "repeat",
        _ => "comma",
    };
    let auth_in = match super::client::auth(g) {
        Some(a) if a.method == "auth_query" => "query",
        _ => "headers",
    };
    RuntimeOptions {
        gem_name: g.target.lib.clone(),
        module_name: g.m.clone(),
        array_format: array_format.to_owned(),
        auth_in: auth_in.to_owned(),
        base_model_example: base_model_example(g),
        enum_example: enum_example(g),
        union_example: String::new(),
    }
}
