//! Forge -> [`crate::model`] (ADR 0002, ADR 0017). The only code in this crate that imports
//! `forge::*`, and the crate is the only one language plugins need; a test fails the build if
//! anything else here names Forge.

pub mod extensions;
pub mod to_model;

pub use to_model::{forge_to_model, group_to_model, methods_to_model};
