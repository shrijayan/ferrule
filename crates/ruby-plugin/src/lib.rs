//! `ferrule-ruby-plugin`: the neutral SDK model -> a Ruby gem in Stainless's layout (ADR 0017).
//!
//! ```text
//! model::Client --namer + tree + emitters--> generated files + runtime slots --ferrule_runtime--> gem
//! ```
//!
//! Every model and resource has three views: `lib/**.rb` (the code, with YARD tags),
//! `rbi/**.rbi` (Sorbet) and `sig/**.rbs` (RBS). They print from one declaration tree
//! ([`tree`]) with a view flag, and every type is spelled by one mapper ([`types`]).
//!
//! Pure: no file, network or clock access, and no Forge types (the model comes from
//! `ferrule-sdk-model`). The files are what Stainless's generator writes before the gem's
//! `rake format`; `ferrule generate` then runs rubocop and `syntax_tree` on them as that task
//! does (see [`printer`]).
//!
//! The runtime (`internal/**`, `errors`, `file_part`, `request_options`, in each view, and their
//! tests) is not emitted here: it is the template in `runtime/ruby`, rendered by
//! `ferrule_runtime.render_ruby_files` with [`RubyOutput::runtime`].

pub mod doc;
pub mod emit;
pub mod meta;
pub mod naming;
pub mod printer;
pub mod tree;
pub mod types;

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::Client;

pub use emit::runtime::RuntimeOptions;
pub use meta::{RubyMeta, RubyTarget, ruby_target};

use crate::emit::{Gen, View};

/// What one generation produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyOutput {
    /// Generated files by path from the gem root, sorted by path.
    pub files: Vec<(String, String)>,
    /// The runtime template's options and slots.
    pub runtime: RuntimeOptions,
}

/// Generates the Ruby SDK files of `client`.
///
/// # Errors
///
/// A FER diagnostic when the Ruby settings are invalid.
pub fn generate(client: &Client, meta: &RubyMeta) -> Result<RubyOutput, FerruleError> {
    let target = ruby_target(client, meta)?;
    // Only what stainless.yml generates for Ruby (`skip`/`only`).
    let mut ruby = client.for_target("ruby");
    // Stainless declares every model stainless.yml names for the target, also those no method
    // reaches (`for_target` drops them; Phase 1 gap list).
    for d in client.models.iter().filter(|d| d.is_for("ruby")) {
        if !ruby.models.iter().any(|k| k.r#ref == d.r#ref) {
            ruby.models.push(d.clone());
        }
    }
    let client = &ruby;
    let g = Gen::new(client, &target);
    let mut files = vec![emit::index::lib_file(&g)];
    files.extend(emit::index::models_files(&g));
    files.extend(emit::index::version_files(&g));
    files.extend(emit::client::client_files(&g));
    for f in &g.models {
        for view in View::ALL {
            files.push(emit::model::model_file(&g, f, view));
        }
    }
    for r in &g.resources {
        for view in View::ALL {
            files.push(emit::resource::resource_file(&g, r, view));
        }
    }
    files.extend(emit::tests::helper_files(&g));
    files.extend(emit::tests::resource_tests(&g));
    files.extend(emit::tests::client_test(&g));
    files.extend(emit::project::project_files(&g));
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(RubyOutput {
        files,
        runtime: emit::runtime::runtime_options(&g),
    })
}
