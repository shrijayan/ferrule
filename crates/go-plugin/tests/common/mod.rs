//! Test inputs: annotated specs from the converter and the neutral model from them. This is the
//! only place in the crate that names Forge (to parse the annotated spec for
//! `ferrule_sdk_model::forge_to_model`); `src/` never does (see `boundary.rs`).

// Each test binary that includes this module uses only part of it.
#![allow(dead_code)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ferrule_sdk_model::model::Client;

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The neutral model of an annotated spec.
pub fn model(annotated: &str) -> Client {
    let forge: forge::Forge = serde_json::from_str(annotated).expect("a document Forge reads");
    ferrule_sdk_model::forge_to_model(&forge)
        .expect("converts")
        .expect("has operations")
}

/// The annotated spec of a config oracle in `oracles/cache`, or `None` when it is not fetched.
pub fn annotated(config_dir: &str, spec: &str, config: &str) -> Option<String> {
    let dir = root().join("oracles/cache").join(config_dir);
    let spec = std::fs::read_to_string(dir.join(spec)).ok()?;
    let config = std::fs::read_to_string(dir.join(config)).ok()?;
    Some(
        ferrule_converter::convert_text(&spec, &config)
            .expect("converts")
            .output,
    )
}

/// An oracle SDK checkout, or `None` (with a note) when it is not fetched.
pub fn oracle(name: &str) -> Option<PathBuf> {
    let dir = root().join("oracles/cache").join(name);
    if dir.join("go.mod").is_file() {
        Some(dir)
    } else {
        eprintln!("skipped: oracles/cache/{name} is not fetched");
        None
    }
}

/// `gofmt`, from `$GOFMT` or the `PATH`, if installed.
pub fn gofmt_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GOFMT") {
        return Some(PathBuf::from(p));
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join("gofmt"))
            .find(|p| p.is_file())
    })
}

/// `gofmt` of `text`, or `None` when gofmt is not installed. Panics on a syntax error.
pub fn gofmt(text: &str) -> Option<String> {
    let path = gofmt_path()?;
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("gofmt runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(text.as_bytes())
        .expect("writes");
    let out = child.wait_with_output().expect("gofmt finishes");
    assert!(out.status.success(), "gofmt: {}", String::from_utf8_lossy(&out.stderr));
    Some(String::from_utf8(out.stdout).expect("utf-8"))
}

/// Ferrule's branding of the oracle's text (`crates/python-plugin/src/emit/files/BRANDING.md`):
/// the header line and the `.keep` files.
pub fn brand(text: &str) -> String {
    text.replace(
        "// File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details.",
        ferrule_go_plugin::printer::HEADER,
    )
    .replace(
        "File generated from our OpenAPI spec by Stainless.\n",
        "File generated from our OpenAPI spec by Ferrule.\n",
    )
    .replace(
        "It is ignored by Stainless code generation",
        "It is ignored by Ferrule code generation",
    )
}

/// Every file under `dir`, relative and `/`-separated, without `.git`.
pub fn walk(dir: &Path) -> Vec<String> {
    fn go(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).expect("readable") {
            let e = e.expect("entry");
            let p = e.path();
            if p.is_dir() {
                if e.file_name() != ".git" {
                    go(root, &p, out);
                }
            } else {
                out.push(
                    p.strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    go(dir, dir, &mut out);
    out.sort();
    out
}

/// Whether `path` is a Go source file.
pub fn is_go(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e == "go")
}
