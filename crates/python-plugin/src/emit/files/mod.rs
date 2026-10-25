//! Project files (README, api.md, pyproject, tests, scripts). Port of
//! `python-plugin/src/emit/files/`.

pub mod api_tests;
pub mod input;

pub use input::ProjectFilesInput;

/// Every project file, keyed by its path in the SDK repository, sorted by path.
///
/// # Panics
///
/// Until the port lands (ADR 0014).
#[must_use]
pub fn emit_project_files(input: &ProjectFilesInput) -> Vec<(String, String)> {
    let _ = input;
    todo!("port of emit/files/index.ts emitProjectFiles")
}
