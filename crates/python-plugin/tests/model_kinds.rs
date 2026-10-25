//! Named models that are not objects (`tests/model_kinds/`): unions, literals, maps, arrays and a
//! recursive union, each emitted as Stainless does. Every file under
//! `tests/golden/model-kinds/src/` is compared byte for byte with the generated one; the other
//! generated files are not pinned here. The expected files were written by hand from the oracle
//! SDKs (knock-mgmt `workflow_step.py`, `condition_group.py`, `audience.py`; imagekit
//! `shared/extensions.py`, `shared/streaming_resolution.py`, `types/__init__.py`; knock
//! `inline_channel_data_request_param.py`), not generated.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrule_python_plugin::plugin::code_files;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn expected() -> BTreeMap<String, String> {
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
    let root = dir().join("golden/model-kinds");
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

fn generated() -> BTreeMap<String, String> {
    let spec = fs::read(dir().join("model_kinds/annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).expect("generates").into_iter().collect()
}

#[test]
fn emits_named_models_that_are_not_objects_as_stainless_does() {
    let actual = generated();
    let expected = expected();
    assert!(expected.len() >= 9, "{:?}", expected.keys());
    for (path, want) in &expected {
        let got = actual.get(path).unwrap_or_else(|| panic!("{path} is not generated"));
        assert_eq!(got, want, "{path}");
    }
}
