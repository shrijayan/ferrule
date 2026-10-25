//! Request headers (`tests/headers/<shape>/`): header parameters and the client options sent as
//! headers, each emitted as Stainless does. Every file under `tests/golden/headers/<shape>/src/` is
//! compared byte for byte with the generated one; the other generated files are not pinned here.
//! The expected files follow the oracle SDKs named in `tests/headers/README.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrule_python_plugin::plugin::code_files;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn expected(shape: &str) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).expect("golden directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read_to_string(&path).expect("golden file"));
            }
        }
    }
    let root = dir().join("golden/headers").join(shape);
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

fn generated(shape: &str) -> BTreeMap<String, String> {
    let spec = fs::read(dir().join("headers").join(shape).join("annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None)
        .unwrap_or_else(|e| panic!("{shape} does not generate: {e}"))
        .into_iter()
        .collect()
}

fn check(shape: &str) -> BTreeMap<String, String> {
    let actual = generated(shape);
    let expected = expected(shape);
    assert!(!expected.is_empty(), "no golden files for {shape}");
    for (path, want) in &expected {
        let got = actual.get(path).unwrap_or_else(|| panic!("{path} is not generated"));
        assert_eq!(got, want, "{path}");
    }
    actual
}

/// Header parameters (ogx `models.list`/`retrieve`, stagehand `sessions.act`/`end`): keyword
/// arguments after the query and body ones (by wire name), in every overload; documented when
/// the spec describes them; fields of the params `TypedDict` after the others, aliased to the
/// header name; merged into `extra_headers` with `strip_not_given`, a string as is and anything
/// else as `str(x) if is_given(x) else not_given`. A method whose only keyword arguments are
/// headers has no params file.
#[test]
fn header_params_are_keyword_arguments_merged_into_the_extra_headers() {
    let actual = check("params");
    assert!(!actual.contains_key("src/fx/types/model_retrieve_params.py"));
    assert!(!actual.contains_key("src/fx/types/session_end_params.py"));
}
