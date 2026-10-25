//! `ferrule-ts-plugin`: emits the TypeScript SDK Stainless generates for the `typescript` target
//! (its current layout), from the language-neutral model (ADR 0017).
//!
//! The crate never sees Forge: it reads `ferrule_sdk_model::Client` and the inputs the model does
//! not carry yet ([`meta::TsMeta`], [`extras::SpecExtras`]), for the `typescript` target only
//! (`Client::for_target`). Emitters are pure and deterministic;
//! formatting is prettier's algorithm reproduced in [`doc`] and [`pretty`].

pub mod comment;
pub mod ctx;
pub mod doc;
pub mod emit;
pub mod examples;
pub mod extras;
pub mod js;
pub mod meta;
pub mod naming;
pub mod pretty;
pub mod types;

use ferrule_sdk_model::Client;

pub use extras::SpecExtras;
pub use meta::{TsMeta, TsSettings, ts_settings};

/// Every file the plugin writes for `client`, sorted by path.
#[must_use]
pub fn generate(client: &Client, meta: &TsMeta, extras: &SpecExtras) -> Vec<emit::File> {
    let client = client.for_target(meta::TARGET);
    let ctx = ctx::Ctx::new(&client, ts_settings(&client, meta), extras);
    let mut files = emit::resource_files(&ctx);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}
