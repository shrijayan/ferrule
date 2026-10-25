//! The Forge plugin that emits a Stainless-identical Python SDK (ADR 0002, ADR 0014).
//!
//! Only [`adapter`] may import Forge types; emitters see [`model`] only.

pub mod emit;
pub mod model;
