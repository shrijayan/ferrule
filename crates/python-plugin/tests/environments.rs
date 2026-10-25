//! A client with several environments (`tests/environments/`, reduced from honcho's `demo`,
//! `local` and `production`). `_client.py` and `__init__.py` are compared byte for byte with
//! `tests/golden/environments/src/`, which follows lithic-python's `_client.py` and `__init__.py`
//! (the current generator era; honcho-python-core writes the same environment code).

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

#[test]
fn emits_environments_as_stainless_does() {
    let spec = fs::read(dir().join("environments/annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    let files = code_files(&forge, None).unwrap_or_else(|e| panic!("does not generate: {e}"));
    for path in ["src/fx/_client.py", "src/fx/__init__.py"] {
        let want = fs::read_to_string(dir().join("golden/environments").join(path)).expect("golden file");
        let got = files
            .iter()
            .find(|(p, _)| p == path)
            .map_or_else(|| panic!("{path} is not generated"), |(_, t)| t.as_str());
        assert_eq!(got, want, "{path}");
    }
}
