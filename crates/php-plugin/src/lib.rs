//! `ferrule-php-plugin`: the neutral SDK model -> a Composer package in Stainless's PHP layout
//! (ADR 0017).
//!
//! ```text
//! model::Client --namer + tree + emitters--> generated files + runtime options --ferrule_runtime--> package
//! ```
//!
//! Every model, enum and union is a class in its own file under `src/`, nested by namespace as
//! Stainless nests them ([`tree`]); each resource is a service with a raw service and two
//! contracts. Every type is spelled by one mapper ([`types`]) and every list in brackets is laid
//! out by one printer ([`layout`]).
//!
//! Pure: no file, network or clock access, and no Forge types (the model comes from
//! `ferrule-sdk-model`). The files are what Stainless's generator writes before the package's
//! `scripts/format`; `ferrule generate` then runs php-cs-fixer on them as that script does.
//!
//! The runtime (`src/Core.php`, `src/Core/**`, `src/RequestOptions.php` and `tests/Core/**`) is
//! not emitted here: it is the template in `runtime/php`, rendered by
//! `ferrule_runtime.render_php_files` with [`PhpOutput::runtime`].

pub mod emit;
pub mod layout;
pub mod meta;
pub mod naming;
pub mod tree;
pub mod types;

use ferrule_sdk_model::model::{Client, Resource};
use ferrule_sdk_model::{FerCode, FerruleError};

pub use emit::runtime::RuntimeOptions;
pub use meta::{PhpMeta, PhpTarget, php_target};

use crate::emit::Gen;

/// What one generation produces.
#[derive(Debug, Clone, PartialEq)]
pub struct PhpOutput {
    /// Generated files by path from the package root, sorted by path.
    pub files: Vec<(String, String)>,
    /// The runtime template's options.
    pub runtime: RuntimeOptions,
}

/// FER201 for a method whose request body is not an object or a union of objects: its params
/// class would have nothing to send (the Python plugin reports the same).
fn unsupported_bodies(client: &Client, resources: &[Resource]) -> Result<(), FerruleError> {
    for r in resources {
        for m in &r.methods {
            if m.body.as_ref().is_some_and(|b| !tree::body_spreads(client, &b.ty)) {
                return Err(FerruleError::new(
                    FerCode::Fer201,
                    m.pointer.clone(),
                    "a request body that is not an object is not generated yet",
                    "leave the operation out, or file an issue with the spec fragment",
                ));
            }
        }
        unsupported_bodies(client, &r.subresources)?;
    }
    Ok(())
}

/// Generates the PHP SDK files of `client`.
///
/// # Errors
///
/// A FER diagnostic when the PHP settings are invalid, and FER201 for a request body that is not
/// an object.
pub fn generate(client: &Client, meta: &PhpMeta) -> Result<PhpOutput, FerruleError> {
    let target = php_target(client, meta)?;
    // Only what stainless.yml generates for PHP (`skip`/`only`).
    let mut php = client.for_target("php");
    // Stainless declares every model stainless.yml names for the target, also those no method
    // reaches (`for_target` drops them; Phase 1 gap list).
    for d in client.models.iter().filter(|d| d.is_for("php")) {
        if !php.models.iter().any(|k| k.r#ref == d.r#ref) {
            php.models.push(d.clone());
        }
    }

    unsupported_bodies(&php, &php.resources)?;

    let g = Gen::new(&php, client, &target);
    let mut files = Vec::new();
    for d in &g.decls {
        files.push(emit::model::model_file(&g, d));
    }
    files.extend(emit::service::service_files(&g));
    files.extend(emit::client::client_files(&g));
    files.extend(emit::tests::test_files(&g));
    files.extend(emit::project::project_files(&g));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);
    Ok(PhpOutput {
        files,
        runtime: emit::runtime::runtime_options(&g),
    })
}
