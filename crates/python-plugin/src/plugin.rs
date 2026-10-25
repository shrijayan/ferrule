//! The plugin entry: Forge -> adapter -> [`SdkClient`] -> emitters, as a native [`forge::Plugin`]
//! (ADR 0014).
//!
//! Every file needs document-level data (Forge gaps G7, G10), so [`forge::Plugin::generate`] emits
//! them all and `generate_group` keeps the trait's default, which emits nothing. The plugin holds
//! no state, so the groups Forge runs in parallel cannot change the output.

use std::borrow::Cow;

use forge::Forge;

use crate::adapter::forge_to_model;
use crate::diagnostics::FerruleError;
use crate::emit::files_input::{ProjectMeta, placeholder_meta};
use crate::emit::project::{FileTree, emit_code_files, emit_root_files};
use crate::model::SdkClient;

/// One generated file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub path: String,
    pub content: String,
}

impl forge::File for OutputFile {
    fn path(&self) -> &str {
        &self.path
    }

    fn content(&self) -> Result<Cow<'_, [u8]>, forge::Err> {
        Ok(Cow::Borrowed(self.content.as_bytes()))
    }
}

/// The Python SDK generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonPlugin {
    /// What the project files need that the annotated spec does not carry yet.
    pub meta: ProjectMeta,
}

impl Default for PythonPlugin {
    /// The plugin with the placeholder metadata the TypeScript plugin uses.
    fn default() -> Self {
        Self {
            meta: placeholder_meta(),
        }
    }
}

/// The model of the whole document; `None` when it has no operations.
///
/// # Errors
///
/// A FER diagnostic from the adapter.
pub fn to_model(forge: &Forge) -> Result<Option<SdkClient>, FerruleError> {
    forge_to_model(forge)
}

/// The code files alone (`src/<pkg>/**` but the agent context), sorted by path.
///
/// # Errors
///
/// A FER diagnostic from the adapter or the emitters.
pub fn code_files(forge: &Forge, meta: Option<&ProjectMeta>) -> Result<FileTree, FerruleError> {
    Ok(match forge_to_model(forge)? {
        None => Vec::new(),
        Some(client) => emit_code_files(&client, meta)?,
    })
}

impl PythonPlugin {
    /// Every file of the SDK, sorted by path.
    ///
    /// # Errors
    ///
    /// A FER diagnostic from the adapter or the emitters.
    pub fn files(&self, forge: &Forge) -> Result<FileTree, FerruleError> {
        Ok(match forge_to_model(forge)? {
            None => Vec::new(),
            Some(client) => emit_root_files(&client, Some(&self.meta))?,
        })
    }
}

impl forge::Plugin for PythonPlugin {
    type File = OutputFile;

    fn generate(&self, forge: &Forge) -> Result<impl Iterator<Item = OutputFile>, forge::Err> {
        let files = self.files(forge)?;
        Ok(files.into_iter().map(|(path, content)| OutputFile { path, content }))
    }
}
