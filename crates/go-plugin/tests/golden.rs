//! A small synthetic API (`tests/synthetic/{openapi,stainless}.yml`) against its committed golden
//! tree (`tests/synthetic/golden`, as `ferrule-gen generate --target go` writes it, the runtime
//! options included), byte for byte. It covers what OneBusAway does not: a JSON body with a
//! nested object, an enum and an array, a method without a response, an integer path param, a
//! subresource, a resource model and a custom casing.
//!
//! Golden changes are reviewed by a human: never rewrite the tree to make this pass.

mod common;

use std::path::Path;

use ferrule_go_plugin::{GoMeta, generate};

fn generated() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("synthetic input");
    let annotated = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml"))
        .expect("converts")
        .output;
    let out = generate(&common::model(&annotated), &GoMeta::default()).expect("generates");
    let mut files = out.files;
    files.push((".ferrule/go-runtime.json".to_owned(), out.runtime.to_json()));
    files.sort();
    files
}

#[test]
fn synthetic_matches_the_golden_tree() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic/golden");
    let files = generated();
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(common::walk(&golden), paths, "the golden tree has other files");
    for (path, text) in &files {
        let expected = std::fs::read_to_string(golden.join(path)).expect("golden file");
        assert!(*text == expected, "{path} differs from the golden tree");
    }
}

#[test]
fn synthetic_is_gofmt_clean_and_deterministic() {
    let files = generated();
    assert_eq!(files, generated());
    if common::gofmt_path().is_none() {
        eprintln!("skipped gofmt: not installed");
        return;
    }
    for (path, text) in files.iter().filter(|(p, _)| common::is_go(p)) {
        assert!(
            common::gofmt(text).as_deref() == Some(text.as_str()),
            "{path} is not gofmt-clean"
        );
    }
}
