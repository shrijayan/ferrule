//! `ferrule-csharp-plugin`: the neutral SDK model -> a .NET SDK in Stainless's C# layout
//! (ADR 0017).
//!
//! ```text
//! model::Client --namer + tree + emitters--> generated files + runtime slots --ferrule_runtime--> SDK
//! ```
//!
//! The SDK is `src/<Ns>/` (the client, `Services/**`, `Models/**`, `Core/`, `Exceptions/`) and
//! `src/<Ns>.Tests/`, with a solution and `Directory.Build.props`. Every service has an
//! interface and a raw-response view, every type is spelled by one mapper ([`types`]), and the
//! names of inline types come from one tree ([`tree`]).
//!
//! Pure: no file, network or clock access, and no Forge types (the model comes from
//! `ferrule-sdk-model`). The code is what the generator writes before the SDK's
//! `scripts/format` (csharpier) lays it out; `ferrule generate` runs csharpier on it.
//!
//! The runtime (`Core/` but `ClientOptions`, `EnvironmentUrl` and `ModelBase`, `Exceptions/`,
//! `IPageExtensions.cs`, `Shims.cs` and their tests) is not emitted here: it is the template in
//! `runtime/csharp`, rendered by `ferrule_runtime.render_csharp_files` with
//! [`CSharpOutput::runtime`].

pub mod doc;
pub mod emit;
pub mod meta;
pub mod naming;
pub mod tree;
pub mod types;

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::Client;

pub use emit::runtime::RuntimeOptions;
pub use meta::{CSharpMeta, CSharpTarget, csharp_target};

use crate::emit::Gen;

/// What one generation produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpOutput {
    /// Generated files by path from the SDK root, sorted by path.
    pub files: Vec<(String, String)>,
    /// The runtime template's options and slot.
    pub runtime: RuntimeOptions,
}

/// Generates the C# SDK files of `client`.
///
/// # Errors
///
/// A FER diagnostic when the C# settings are invalid.
pub fn generate(client: &Client, meta: &CSharpMeta) -> Result<CSharpOutput, FerruleError> {
    let target = csharp_target(client, meta)?;
    // Only what stainless.yml generates for C# (`skip`/`only`), with every model it names for
    // C#, also those no method reaches, as Stainless declares them (Phase 1 gap list), each
    // with the models left out for C# inlined.
    let csharp = client.for_target_with_models("csharp");
    let client = &csharp;
    let g = Gen::new(client, &target);
    let mut files = vec![
        emit::client::interface_file(&g),
        emit::client::client_file(&g),
        emit::options::client_options(&g),
        emit::options::environment_url(&g),
        emit::options::model_base(&g),
        emit::tests::test_base(&g),
    ];
    for s in &g.services {
        files.push(emit::service::interface_file(&g, s));
        files.push(emit::service::class_file(&g, s));
        files.extend(emit::tests::service_test(&g, s));
    }
    for f in &g.files.files {
        files.push(emit::model::model_file(&g, f));
    }
    files.extend(emit::project::project_files(&g));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);
    Ok(CSharpOutput {
        files,
        runtime: emit::runtime::runtime_options(&g),
    })
}
