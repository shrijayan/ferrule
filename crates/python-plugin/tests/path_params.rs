//! Methods with several path parameters (`tests/path_params/`, reduced from honcho
//! `workspaces.peers.card`): only the last is positional, the others are required keyword
//! arguments that lead the params `TypedDict` (honcho-python-core `peers.py` and
//! `peer_card_params.py`, imagekit-python `versions.delete`, knock-mgmt
//! `StepPreviewTemplateParams`). A method with path parameters alone has no params file. The
//! files under `tests/golden/path-params/src/` are compared byte for byte.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

#[test]
fn makes_only_the_last_path_parameter_positional() {
    let spec = fs::read(dir().join("path_params/annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    let files = code_files(&forge, None).unwrap_or_else(|e| panic!("does not generate: {e}"));
    let get = |path: &str| files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str());
    for path in ["src/fx/resources/peers.py", "src/fx/types/peer_card_params.py"] {
        let want = fs::read_to_string(dir().join("golden/path-params").join(path)).expect("golden file");
        assert_eq!(get(path), Some(want.as_str()), "{path}");
    }
    assert_eq!(get("src/fx/types/peer_delete_params.py"), None);
}
