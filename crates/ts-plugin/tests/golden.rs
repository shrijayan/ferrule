//! The code files of a small synthetic SDK (`tests/fixtures/petstore`: a model, a `$shared`
//! model, query, body and path parameters, a nested inline response, a subresource with two path
//! parameters) against `tests/golden/petstore`, byte for byte.
//!
//! Golden changes are approved by a human. To write the goldens after an approved change, run
//! with `FERRULE_TS_BLESS=1` and review the diff.

mod support;

use std::path::Path;

use ferrule_ts_plugin::generate;

/// The files the golden covers: the code, not the templated project files.
fn is_code(path: &str) -> bool {
    path.starts_with("src/") || path.starts_with("tests/api-resources/") || path == "api.md"
}

#[test]
fn petstore_matches_its_golden() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let (client, meta, extras) = support::load(
        &read("fixtures/petstore/openapi.yml"),
        &read("fixtures/petstore/stainless.yml"),
    )
    .expect("the fixture maps to the model");
    let files: Vec<(String, String)> = generate(&client, &meta, &extras)
        .into_iter()
        .filter(|(p, _)| is_code(p))
        .collect();
    let golden = dir.join("golden/petstore");
    if std::env::var_os("FERRULE_TS_BLESS").is_some() {
        let _ = std::fs::remove_dir_all(&golden);
        for (p, text) in &files {
            let path = golden.join(p);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, text).expect("write");
        }
    }
    let expected = support::tree(&golden);
    let paths: Vec<&String> = files.iter().map(|(p, _)| p).collect();
    assert_eq!(paths, expected.iter().collect::<Vec<_>>(), "the file set changed");
    for (p, text) in &files {
        let want = std::fs::read_to_string(golden.join(p)).expect("golden file");
        assert!(*text == want, "{p} differs from its golden");
    }
}
