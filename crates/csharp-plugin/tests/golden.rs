//! A small synthetic API (`tests/synthetic/{openapi,stainless}.yml`) against its committed golden
//! tree (`tests/synthetic/golden`, as `ferrule-gen generate --target csharp` writes it before
//! csharpier runs, the runtime options included), byte for byte. Its config has a C# target and
//! no Python one. It covers a JSON body with a nested object, an enum and an array, a method
//! without a response, an integer path param, a subresource, shared models (one a map of
//! objects) and a custom casing.
//!
//! Golden changes are reviewed by a human: never rewrite the tree to make this pass.

mod common;

use std::path::Path;

use ferrule_csharp_plugin::{CSharpMeta, generate};

fn generated() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("synthetic input");
    let converted = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml")).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    let out = generate(&common::model(&converted.output), &CSharpMeta::default()).expect("generates");
    let mut files = out.files;
    files.push((".ferrule/csharp-runtime.json".to_owned(), out.runtime.to_json()));
    files.sort();
    common::dump("FERRULE_CSHARP_DUMP_RAW", &files);
    files
}

#[test]
fn synthetic_matches_the_golden_tree() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic/golden");
    let files = generated();
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(common::walk(&golden), paths, "the golden tree has other files");
    for (path, text) in &files {
        let expected = std::fs::read(golden.join(path)).expect("golden file");
        assert!(
            text.as_bytes() == expected.as_slice(),
            "{path} differs from the golden tree"
        );
    }
}

/// The output is the same on every run, and csharpier accepts it and is settled after one pass
/// (formatting the formatted SDK changes nothing).
#[test]
fn synthetic_is_deterministic_and_formats() {
    let files = generated();
    assert_eq!(files, generated());
    let Some(formatted) = common::format(&files) else {
        eprintln!("skipped formatting: {}", common::NOT_INSTALLED);
        return;
    };
    assert_eq!(common::format(&formatted), Some(formatted));
}

fn file<'f>(files: &'f [(String, String)], path: &str) -> &'f str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is not generated"), |(_, t)| t.as_str())
}

/// The namespace and client come from `targets.csharp`, with no Python target to fall back on.
#[test]
fn names_the_sdk_after_the_csharp_target() {
    let files = generated();
    let client = file(&files, "src/Acme/AcmeClient.cs");
    assert!(client.contains("namespace Acme;"), "{client}");
    assert!(
        client.contains("public sealed class AcmeClient : IAcmeClient"),
        "{client}"
    );
    let options = file(&files, ".ferrule/csharp-runtime.json");
    assert!(options.contains("\"namespace\": \"Acme\""), "{options}");
}
