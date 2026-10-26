//! `skip_test_reason` (`tests/skip_test/`): every generated API test of the method is skipped
//! with the reason, as knock-python's `users.bulk.set_preferences` tests are.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::emit::files::api_tests::emit_api_tests;
use ferrule_python_plugin::emit::files_input::{placeholder_meta, to_project_files_input};
use ferrule_python_plugin::plugin::to_model;

#[test]
fn skips_each_test_of_a_method_with_a_reason() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/skip_test/annotated.json");
    let forge: forge::Forge = serde_json::from_slice(&fs::read(path).expect("fixture")).expect("a document");
    let client = to_model(&forge).expect("converts").expect("has operations");
    let input = to_project_files_input(&client, &placeholder_meta()).expect("project files");
    let tests = emit_api_tests(&input);
    let module = tests
        .iter()
        .find(|(p, _)| p == "tests/api_resources/test_terminals.py")
        .map(|(_, t)| t.as_str())
        .expect("a test module");
    let skip = "    @pytest.mark.skip(\n        reason=\"the mock server does not read integer query parameters through a $ref\"\n    )\n    @parametrize\n";
    // Method, with all params, raw and streaming response, sync and async; list is not skipped.
    assert_eq!(module.matches(skip).count(), 8, "{module}");
    assert!(
        module.contains(&format!("{skip}    def test_method_read(self")),
        "{module}"
    );
    assert!(
        module.contains("    @parametrize\n    def test_method_list(self"),
        "{module}"
    );
}
