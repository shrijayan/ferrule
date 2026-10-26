//! Project files that follow stainless.yml settings, on the petstore fixture: the mock server the
//! resource tests need unless `settings.disable_mock_tests`, as onebusaway-typescript (`f3ca3f47`)
//! has it.

mod support;

use std::path::Path;

use ferrule_ts_plugin::generate;

/// Every file of the petstore SDK, its stainless.yml first passed through `edit`.
fn petstore(edit: impl FnOnce(String) -> String) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/petstore");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let (client, meta, extras) =
        support::load(&read("openapi.yml"), &edit(read("stainless.yml"))).expect("the fixture maps to the model");
    generate(&client, &meta, &extras)
}

fn file<'a>(files: &'a [(String, String)], path: &str) -> Option<&'a str> {
    files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str())
}

#[test]
fn mock_tests_start_a_mock_server() {
    let files = petstore(|c| c);
    let mock = file(&files, "scripts/mock").expect("scripts/mock is written");
    assert!(mock.contains("npm exec --package=@stdy/cli@0.22.1 -- steady --host 127.0.0.1 -p 4010"));
    let test = file(&files, "scripts/test").expect("scripts/test");
    assert!(
        test.contains("  ./scripts/mock --daemon\n"),
        "scripts/test starts no mock server"
    );
    let contributing = file(&files, "CONTRIBUTING.md").expect("CONTRIBUTING.md");
    assert!(contributing.contains("```sh\n$ ./scripts/mock\n```"));
}

#[test]
fn disabled_mock_tests_need_no_mock_server() {
    let files = petstore(|c| {
        c.replace(
            "  license: Apache-2.0\n",
            "  license: Apache-2.0\n  disable_mock_tests: true\n",
        )
    });
    assert!(file(&files, "scripts/mock").is_none());
    let test = file(&files, "scripts/test").expect("scripts/test");
    assert!(!test.contains("mock"), "{test}");
}
