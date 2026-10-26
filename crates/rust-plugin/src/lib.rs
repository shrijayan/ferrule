//! `ferrule-rust-plugin`: the neutral SDK model -> a Rust SDK crate (ADR 0019), Ferrule's first
//! target that Stainless does not have.
//!
//! ```text
//! model::Client --namer + emitters--> generated files + runtime/rust/template --> crate
//! ```
//!
//! Pure: no file, network or clock access, and no Forge types (the model comes from
//! `ferrule-sdk-model`). The output is not laid out yet: `ferrule-gen` runs rustfmt over it, as
//! the tests do, so the printer only has to write valid code.

pub mod context;
pub mod doc;
pub mod emit;
pub mod fake;
pub mod meta;
pub mod naming;
pub mod runtime;
pub mod types;

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::Client;

pub use meta::{Names, RustMeta};

/// The Rust crate of `client`: every file by path from the crate root, sorted by path, the
/// runtime's included.
///
/// # Errors
///
/// A FER diagnostic for a crate name with no letters or something the target cannot generate.
pub fn generate(client: &Client, meta: &RustMeta) -> Result<Vec<(String, String)>, FerruleError> {
    let client = client.for_target("rust");
    let names = Names::of(&client, meta)?;
    let mut files = emit::generate(&context::Gen::new(&client, names))?;
    files.extend(runtime::FILES.iter().map(|(p, t)| ((*p).to_owned(), (*t).to_owned())));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}
