//! Example calls: a parameter typed by a named string alias (`tests/alias_examples/`, reduced
//! from the opencode SDK's IDs) gets a string, and one typed by a named object model its fields,
//! not `{}`, in the API tests and the agent recipe.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::emit::agent::emit_agent_files;
use ferrule_python_plugin::emit::files::api_tests::emit_api_tests;
use ferrule_python_plugin::emit::files_input::{ReadmeRequest, placeholder_meta, to_project_files_input};
use ferrule_python_plugin::model::SdkClient;
use ferrule_python_plugin::plugin::to_model;

fn model() -> SdkClient {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/alias_examples/annotated.json");
    let forge: forge::Forge = serde_json::from_slice(&fs::read(path).expect("fixture")).expect("a document");
    to_model(&forge).expect("converts").expect("has operations")
}

#[test]
fn a_named_type_parameter_gets_an_example_of_that_type() {
    let client = model();
    let recipes = emit_agent_files(&client, Some(&placeholder_meta()));
    let recipe = recipes
        .iter()
        .find(|(p, _)| p == "examples/agent/sessions_retrieve.py")
        .map(|(_, t)| t.as_str())
        .expect("a recipe");
    assert!(
        recipe.contains("client.sessions.retrieve(session_id=\"sessionID\")"),
        "{recipe}"
    );
    let mut meta = placeholder_meta();
    meta.readme_request = Some(ReadmeRequest {
        endpoint: "get /sessions/{sessionID}".to_owned(),
        params: serde_json::from_str(r#"{"sessionID": "ses_1"}"#).expect("params"),
    });
    let input = to_project_files_input(&client, &meta).expect("project files");
    let tests = emit_api_tests(&input);
    let module = tests
        .iter()
        .find(|(p, _)| p == "tests/api_resources/test_sessions.py")
        .map(|(_, t)| t.as_str())
        .expect("a test module");
    assert!(
        module.contains("retrieve(\n            \"sessionID\",\n        )"),
        "{module}"
    );
    assert!(!module.contains("{},"), "{module}");
    // A named object model lists its fields, as lithic-python's tests write `address`.
    assert!(module.contains("\"providerID\": \"string\","), "{module}");
}
