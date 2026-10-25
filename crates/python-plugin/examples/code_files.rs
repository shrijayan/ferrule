//! Writes the code files of an annotated spec (`src/<pkg>/**` without the agent context), to
//! diff the port against the TypeScript plugin while the project-file emitters are being ported:
//!
//! ```text
//! cargo run -p ferrule-python-plugin --example code_files -- annotated.json out/
//! ```

use std::process::ExitCode;

use ferrule_python_plugin::emit::files_input::placeholder_meta;
use ferrule_python_plugin::plugin::{OutputFile, code_files};

fn run(spec: &str, out: &str) -> Result<(), forge::Err> {
    let forge: forge::Forge = serde_json::from_slice(&std::fs::read(spec)?)?;
    let files = code_files(&forge, Some(&placeholder_meta()))?;
    forge::write(
        out,
        files
            .into_iter()
            .map(|(path, content)| OutputFile { path, content })
            .collect(),
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [spec, out] = args.as_slice() else {
        eprintln!("usage: code_files <annotated.json> <out>");
        return ExitCode::from(2);
    };
    match run(spec, out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("code_files: {err}");
            ExitCode::FAILURE
        }
    }
}
