//! Methods whose parameters would share a Python name, a `SyntaxError` that sinks every test
//! importing the module (Tier 1 corpus run, 2026-10-26). Each fixture directory's README says
//! where it was reduced from and which Stainless SDK shows the expected output.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn generate(fixture: &str) -> Vec<(String, String)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join(fixture).join("annotated.json");
    let spec = fs::read(path).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).unwrap_or_else(|e| panic!("does not generate: {e}"))
}

fn file<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files.iter().find(|(p, _)| p == path).map_or_else(|| panic!("no {path}"), |(_, t)| t.as_str())
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn merges_an_allof_property_declared_twice_into_one_parameter() {
    let files = generate("allof_merge");
    let resource = file(&files, "src/fx/resources/completions.py");
    // Once in the sync and once in the async method.
    assert_eq!(count(resource, "        top_logprobs: Optional[int] | Omit = omit,\n"), 2, "{resource}");
    assert_eq!(count(resource, "top_logprobs: int"), 0, "{resource}");
    assert_eq!(count(resource, "top_logprobs: Outer description."), 2, "{resource}");
    let params = file(&files, "src/fx/types/completion_create_params.py");
    assert_eq!(count(params, "    top_logprobs: Optional[int]\n    \"\"\"Outer description.\"\"\"\n"), 1, "{params}");
    assert_eq!(count(params, "top_logprobs:"), 1, "{params}");
}
