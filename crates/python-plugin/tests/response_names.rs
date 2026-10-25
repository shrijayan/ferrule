//! Nested class names of a method's response model (`tests/response_names/`, reduced from honcho
//! `sessions.summaries`). A response that is a `$ref` to a schema the config does not name keeps
//! its nested classes unprefixed (honcho-python-core `session_summaries_response.py`:
//! `LongSummary`; imagekit-python `file_upload_response.py`: `AITag`); an inline response
//! prefixes them with its own name, as OneBusAway's do. The files under
//! `tests/golden/response-names/src/` are compared byte for byte.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

#[test]
fn prefixes_nested_classes_of_inline_responses_only() {
    let spec = fs::read(dir().join("response_names/annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    let files = code_files(&forge, None).unwrap_or_else(|e| panic!("does not generate: {e}"));
    for path in [
        "src/fx/types/session_summaries_response.py",
        "src/fx/types/session_context_response.py",
    ] {
        let want = fs::read_to_string(dir().join("golden/response-names").join(path)).expect("golden file");
        let got = files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str());
        assert_eq!(got, Some(want.as_str()), "{path}");
    }
}
