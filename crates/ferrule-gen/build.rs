//! Reads the pinned Forge git rev from the workspace `Cargo.lock` into `FERRULE_FORGE_REV`, so
//! `ferrule-gen --version` can name the Forge commit it was built with. `ferrule.lock` records
//! that rev: unlike a hash of the binary, it is the same on every platform.
//!
//! It also embeds the Python runtime template (`runtime/template`, ADR 0020): it writes
//! `$OUT_DIR/python_template.rs`, one `(path, include_str!(..))` entry per file, walked as
//! `ferrule_runtime`'s `_walk` walks it, so `ferrule-gen generate --spec` renders the runtime
//! without the Python package.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Entries `ferrule_runtime` never renders: caches, and `ruff.toml` (a lint opt-out, not shipped).
const NOT_TEMPLATE: [&str; 2] = ["__pycache__", "ruff.toml"];

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let lock = Path::new(&manifest).join("../../Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = fs::read_to_string(&lock).unwrap_or_else(|err| panic!("cannot read {}: {err}", lock.display()));
    match forge_rev(&text) {
        Some(rev) => println!("cargo:rustc-env=FERRULE_FORGE_REV={rev}"),
        None => panic!(
            "{}: no `forge` package with a git source pinned to a 40-hex rev",
            lock.display()
        ),
    }
    embed_python_template(&Path::new(&manifest).join("../../runtime/template"));
}

/// Writes `$OUT_DIR/python_template.rs`: `PYTHON_TEMPLATE`, every template file in walk order.
fn embed_python_template(root: &Path) {
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(root, "", &mut files);
    let mut code = String::from("pub const PYTHON_TEMPLATE: &[(&str, &str)] = &[\n");
    for (rel, path) in files {
        let path = path.canonicalize().expect("template file");
        let _ = writeln!(code, "    ({rel:?}, include_str!({:?})),", path.display().to_string());
    }
    code.push_str("];\n");
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("python_template.rs");
    fs::write(&out, code).unwrap_or_else(|err| panic!("cannot write {}: {err}", out.display()));
}

/// `_walk`: entries sorted by name, skipping [`NOT_TEMPLATE`] and dot files, directories in place.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", dir.display()))
        .map(|e| e.expect("directory entry").path())
        .collect();
    entries.sort_by_key(|p| p.file_name().map(std::ffi::OsStr::to_os_string));
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).expect("UTF-8 file name");
        if NOT_TEMPLATE.contains(&name) || name.starts_with('.') {
            continue;
        }
        let rel = format!("{prefix}{name}");
        if path.is_dir() {
            walk(&path, &format!("{rel}/"), out);
        } else {
            out.push((rel, path));
        }
    }
}

/// The commit of the `forge` package's git source in a `Cargo.lock`: the text after `#` in
/// `source = "git+https://...?rev=<rev>#<commit>"`.
fn forge_rev(lock: &str) -> Option<&str> {
    let mut in_forge = false;
    for line in lock.lines().map(str::trim) {
        if line == "[[package]]" {
            in_forge = false;
        } else if line == r#"name = "forge""# {
            in_forge = true;
        } else if in_forge && let Some(source) = line.strip_prefix("source = \"git+") {
            let rev = source.strip_suffix('"')?.rsplit_once('#')?.1;
            let hex = rev.len() == 40 && rev.bytes().all(|b| b.is_ascii_hexdigit());
            return hex.then_some(rev);
        }
    }
    None
}
