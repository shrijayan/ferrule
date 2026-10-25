//! Forge -> [`crate::model`] (ADR 0002). With [`crate::plugin`], the only code that imports
//! `forge::*`; a test fails the build if anything else does.

pub mod extensions;
pub mod to_model;

pub use to_model::{forge_to_model, group_to_model};
