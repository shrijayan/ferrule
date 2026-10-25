//! stainless.yml validation: the public JSON Schema, Ferrule's typed reader and `FER11xx`
//! diagnostics.

pub mod config;
pub mod json_schema;
pub mod suggest;
pub mod validate;

use crate::value::Value;
use std::sync::LazyLock;

/// The public stainless.yml JSON Schema (draft 2020-12), bundled so validation stays pure.
static STAINLESS_CONFIG_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    crate::value::parse_json(include_str!("../../schema/stainless-config.schema.json")).unwrap_or(Value::Bool(true))
});

/// The bundled public schema.
#[must_use]
pub fn stainless_config_schema() -> &'static Value {
    &STAINLESS_CONFIG_SCHEMA
}
