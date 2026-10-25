//! Reads the pinned Forge git rev from the workspace `Cargo.lock` into `FERRULE_FORGE_REV`, so
//! `ferrule-gen --version` can name the Forge commit it was built with. `ferrule.lock` records
//! that rev: unlike a hash of the binary, it is the same on every platform.

use std::path::Path;
use std::{env, fs};

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
