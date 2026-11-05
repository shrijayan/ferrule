//! Optional credentials (`tests/optional_auth/`, reduced from the opencode SDK's basic auth): the
//! client builds without them, so the tests do not expect its constructor to raise.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::emit::files::emit_project_files;
use ferrule_python_plugin::emit::files_input::{placeholder_meta, to_project_files_input};
use ferrule_python_plugin::plugin::to_model;

#[test]
fn optional_credentials_have_no_header_validation_test() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/optional_auth/annotated.json");
    let forge: forge::Forge = serde_json::from_slice(&fs::read(path).expect("fixture")).expect("a document");
    let client = to_model(&forge).expect("converts").expect("has operations");
    let input = to_project_files_input(&client, &placeholder_meta()).expect("project files");
    let files = emit_project_files(&input);
    let test_client = files
        .iter()
        .find(|(p, _)| p == "tests/test_client.py")
        .map(|(_, t)| t.as_str())
        .expect("tests/test_client.py");
    assert!(test_client.contains("password = \"My Password\""), "{test_client}");
    assert!(!test_client.contains("def test_validate_headers"), "{test_client}");
}
