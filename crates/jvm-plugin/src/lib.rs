//! `ferrule-jvm-plugin`: the neutral SDK model -> a Java or Kotlin SDK in Stainless's JVM
//! layout (ADR 0017).
//!
//! ```text
//! model::Client --namer + emitters--> Kotlin sources + Gradle files + runtime options
//!   --ferrule_runtime.render_java_files--> runtime --ktfmt--> SDK
//! ```
//!
//! Stainless writes both its `java` and `kotlin` targets in Kotlin with the same Gradle layout;
//! [`Flavor`] is the one mode flag between them. Pure: no file, network or clock access, and no
//! Forge types (the model comes from `ferrule-sdk-model`). The Kotlin text carries Stainless's
//! tokens, comments and blank lines; `ktfmt --kotlinlang-style` lays it out afterwards (in
//! `ferrule generate` and in the tests), see [`kt`].
//!
//! The runtime (`-core`'s `core/**` but `ClientOptions.kt`, `errors/**`, the `ProGuard` rules and
//! `OkHttpClient.kt`) is not emitted here: it is the template in `runtime/java`, rendered by
//! `ferrule_runtime.render_java_files` with [`JvmOutput::runtime`].

pub mod emit;
pub mod kt;
pub mod meta;
pub mod naming;
pub mod types;

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::Client;

pub use emit::runtime::RuntimeOptions;
pub use meta::{Flavor, JvmMeta, JvmTarget, jvm_target};

use crate::emit::Gen;

/// What one generation produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmOutput {
    /// Generated files by path from the Gradle root, sorted by path. Kotlin files are not yet
    /// formatted (see [`kt`]).
    pub files: Vec<(String, String)>,
    /// The runtime template's options (`ferrule_runtime.JavaOptions`).
    pub runtime: RuntimeOptions,
}

/// Generates the `flavor` SDK files of `client`.
///
/// # Errors
///
/// A FER diagnostic when the JVM settings are invalid.
pub fn generate(client: &Client, flavor: Flavor, meta: &JvmMeta) -> Result<JvmOutput, FerruleError> {
    let target = jvm_target(client, flavor, meta)?;
    // Only what stainless.yml generates for this target (`skip`/`only`).
    let client = &client.for_target(flavor.key());
    let g = Gen::new(client, &target);
    let mut files = Vec::new();
    for s in &g.services {
        for m in s.methods() {
            files.push(emit::params::params_file(&g, s, m));
            files.extend(emit::model::response_file(&g, s, m));
        }
        for async_ in [false, true] {
            files.push(emit::service::interface_file(&g, s, async_));
            files.push(emit::service::impl_file(&g, s, async_));
        }
    }
    for d in &client.models {
        files.extend(emit::model::model_file(&g, d));
    }
    files.extend(emit::client::client_files(&g));
    files.extend(emit::tests::test_files(&g));
    files.extend(emit::project::project_files(&g));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(JvmOutput {
        files,
        runtime: emit::runtime::runtime_options(&g),
    })
}
