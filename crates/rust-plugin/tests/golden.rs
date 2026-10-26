//! The generated crates against their committed golden trees (`tests/<fixture>/golden`, as
//! `ferrule-gen generate --target rust` writes them, rustfmt included), byte for byte:
//!
//! - `onebusaway`: the pinned OneBusAway oracle spec and config (`oracles/manifest.yaml`,
//!   `onebusaway-config` at `27cdbb11`, Apache-2.0), unchanged;
//! - `synthetic`: what OneBusAway does not have: every pagination scheme, streaming, JSON and
//!   binary bodies, a header param, a subresource, a `$client` method, unions, a recursive model
//!   and bearer auth.
//!
//! Rust has no Stainless oracle (ADR 0019), so these goldens are the record of the output, and
//! `crate_checks.rs` proves each one builds, lints clean and passes its own tests. Golden changes
//! are reviewed by a human: never rewrite a tree to make this pass.

mod common;

use common::{FIXTURES, dir, generated, walk};

#[test]
fn fixtures_match_their_golden_trees() {
    let crates: Vec<_> = FIXTURES.iter().map(|f| (*f, generated(f))).collect();
    // Debugging aid: FERRULE_RUST_DUMP=<dir> writes every generated crate there.
    if let Some(out) = std::env::var_os("FERRULE_RUST_DUMP") {
        for (fixture, files) in &crates {
            common::write(&std::path::Path::new(&out).join(fixture), files);
        }
    }
    for (fixture, files) in crates {
        let golden = dir(fixture).join("golden");
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(walk(&golden), paths, "{fixture}: the golden tree has other files");
        for (path, text) in &files {
            let expected = std::fs::read_to_string(golden.join(path)).expect("golden file");
            assert!(*text == expected, "{fixture}: {path} differs from the golden tree");
        }
    }
}

#[test]
fn generation_is_deterministic() {
    for fixture in FIXTURES {
        let model = common::model(fixture);
        let meta = ferrule_rust_plugin::RustMeta::default();
        let first = ferrule_rust_plugin::generate(&model, &meta).expect("generates");
        assert_eq!(first, ferrule_rust_plugin::generate(&model, &meta).expect("generates"));
    }
}
