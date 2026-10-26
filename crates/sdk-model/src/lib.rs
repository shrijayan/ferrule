//! `ferrule-sdk-model`: the language-neutral SDK model and the one adapter from Forge to it
//! (ADR 0002, ADR 0017).
//!
//! Language plugins depend on this crate and add their own namer and emitters; only [`adapter`]
//! imports Forge types, so the rule "Forge stays behind the adapter" holds once for every
//! language.
//!
//! ```text
//! annotated spec --Forge--> Forge's IR --adapter--> model::Client --namer+emitters--> SDK
//! ```

pub mod adapter;
pub mod diagnostics;
pub mod js;
pub mod json;
pub mod model;
pub mod target;

pub use adapter::{forge_to_model, group_to_model, methods_to_model};
pub use diagnostics::{FerCode, FerruleError};
pub use json::to_json;
pub use model::Client;
