//! Test inputs and tools: annotated specs from the converter, the neutral model from them, and
//! rustfmt. This is the only place in the crate that names Forge (to parse the annotated spec
//! for `ferrule_sdk_model::forge_to_model`); `src/` never does (see `boundary.rs`).

// Each test binary that includes this module uses only part of it.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ferrule_rust_plugin::{RustMeta, generate};
use ferrule_sdk_model::model::Client;

/// The fixtures with a golden crate: `tests/<fixture>/{openapi,stainless}.yml` and
/// `tests/<fixture>/golden`.
pub const FIXTURES: [&str; 2] = ["onebusaway", "synthetic"];

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn dir(fixture: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(fixture)
}

/// The neutral model of a fixture's spec and config, through the converter.
pub fn model(fixture: &str) -> Client {
    let dir = dir(fixture);
    let read = |f: &str| fs::read_to_string(dir.join(f)).expect("fixture input");
    let annotated = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml"))
        .expect("converts")
        .output;
    let forge: forge::Forge = serde_json::from_str(&annotated).expect("a document Forge reads");
    ferrule_sdk_model::forge_to_model(&forge)
        .expect("maps to the model")
        .expect("has operations")
}

/// The crate `ferrule-gen generate --target rust` writes for `fixture`, laid out by rustfmt.
pub fn generated(fixture: &str) -> Vec<(String, String)> {
    let files = generate(&model(fixture), &RustMeta::default()).expect("generates");
    let work = scratch(&format!("rustfmt-{fixture}"));
    write(&work, &files);
    let sources: Vec<PathBuf> = files
        .iter()
        .filter(|(p, _)| Path::new(p).extension().is_some_and(|e| e == "rs"))
        .map(|(p, _)| work.join(p))
        .collect();
    let status = Command::new("rustfmt")
        .arg("--config-path")
        .arg(work.join("rustfmt.toml"))
        .args(&sources)
        .status()
        .expect("rustfmt runs (rust-toolchain.toml installs it)");
    assert!(status.success(), "rustfmt failed on the {fixture} crate");
    files
        .into_iter()
        .map(|(p, _)| {
            let text = fs::read_to_string(work.join(&p)).expect("formatted file");
            (p, text)
        })
        .collect()
}

/// An empty directory under the workspace's `target/`, kept between runs.
pub fn scratch(name: &str) -> PathBuf {
    let dir = root().join("target/rust-plugin-tests").join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clears the scratch directory");
    }
    fs::create_dir_all(&dir).expect("creates the scratch directory");
    dir
}

pub fn write(dir: &Path, files: &[(String, String)]) {
    for (path, text) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("creates the directory");
        fs::write(path, text).expect("writes the file");
    }
}

/// Every file under `dir`, relative and sorted.
pub fn walk(dir: &Path) -> Vec<String> {
    fn go(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                go(base, &path, out);
            } else {
                let rel = path.strip_prefix(base).expect("under base");
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    go(dir, dir, &mut out);
    out.sort();
    out
}
