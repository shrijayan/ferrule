//! ADR 0002 / ADR 0017: Forge stays behind `ferrule-sdk-model`'s adapter. The Rust plugin's sources
//! never name it, and it is not a (non-dev) dependency.

use std::fs;
use std::path::{Path, PathBuf};

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).expect("directory") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            out.extend(files(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[test]
fn no_source_file_names_forge() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let offenders: Vec<PathBuf> = files(&src)
        .into_iter()
        .filter(|f| {
            let text = fs::read_to_string(f).expect("source file");
            text.contains("forge::") || text.contains("use forge")
        })
        .collect();
    assert_eq!(offenders, Vec::<PathBuf>::new());
}

#[test]
fn forge_is_only_a_dev_dependency() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).expect("manifest");
    let deps = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    assert!(!deps.contains("forge"), "{deps}");
}
