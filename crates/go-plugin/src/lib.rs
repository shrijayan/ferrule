//! `ferrule-go-plugin`: the neutral SDK model -> a Go SDK in Stainless's Go v1 or v2 layout
//! ([`Layout`], ADR 0017).
//!
//! ```text
//! model::Client --namer + emitters--> generated files + runtime slots --ferrule_runtime--> SDK
//! ```
//!
//! Pure: no file, network or clock access, and no Forge types (the model comes from
//! `ferrule-sdk-model`). The printer writes gofmt's layout itself; gofmt only verifies it, in the
//! tests and in `ferrule generate`.
//!
//! The runtime (`internal/**` but `version.go`, `option/**`, `field.go`,
//! `default_http_client.go`, and v2's `packages/param` and `packages/respjson`) is not emitted
//! here: it is the template in `runtime/go` or `runtime/go-v2`, rendered by
//! `ferrule_runtime.render_go_files` with [`GoOutput::runtime`].

pub mod emit;
pub mod meta;
pub mod naming;
pub mod printer;
pub mod types;

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::Client;

pub use emit::runtime::RuntimeOptions;
pub use meta::{GoMeta, GoTarget, Layout, go_target};

use crate::emit::Gen;
use crate::naming::{Casings, exported, split_words};

/// What one generation produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoOutput {
    /// Generated files by path from the module root, sorted by path.
    pub files: Vec<(String, String)>,
    /// The runtime template's options and slots.
    pub runtime: RuntimeOptions,
}

/// The client name the User-Agent carries: `onebusaway-sdk` -> `OnebusawaySDK`.
#[must_use]
pub fn client_name(client: &Client, target: &GoTarget) -> String {
    let written = client
        .settings
        .as_ref()
        .map_or_else(|| target.package_name.clone(), |s| s.name.written.clone());
    exported(&split_words(&written), Casings(&client.casings))
}

/// Generates the Go SDK files of `client`.
///
/// # Errors
///
/// A FER diagnostic when the Go settings are invalid or a client setting cannot be sent.
pub fn generate(client: &Client, meta: &GoMeta) -> Result<GoOutput, FerruleError> {
    let target = go_target(client, meta)?;
    // Only what stainless.yml generates for Go (`skip`/`only`).
    let client = &client.for_target("go");
    let g = Gen::new(client, &target);
    let name = client_name(client, &target);
    let mut files = vec![
        emit::client::client_file(&g),
        emit::shared::aliases_file(&g),
        emit::api_md::api_md(&g),
        emit::runtime::version_file(&g),
    ];
    files.extend(emit::shared::shared_file(&g));
    files.extend(emit::pagination::pagination_file(&g));
    files.extend(emit::project::project_files(&g));
    for s in &g.services {
        files.push(emit::resource::resource_file(&g, s));
        files.extend(emit::tests::service_test_file(&g, s));
    }
    files.extend(emit::tests::usage_test_file(&g));
    files.extend(emit::tests::pagination_test_files(&g));
    files.extend(emit::tests::client_test_file(&g, &name));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut runtime = emit::runtime::runtime_options(&g, &name)?;
    // v2's `field.go` names its file helper `NewFile` when the root package declares `File`.
    runtime.new_file = g.target.v2()
        && files
            .iter()
            .filter(|(p, _)| !p.contains('/') && !p.ends_with("_test.go"))
            .any(|(_, text)| text.contains("\ntype File struct") || text.contains("\ntype File = "));
    Ok(GoOutput { files, runtime })
}
