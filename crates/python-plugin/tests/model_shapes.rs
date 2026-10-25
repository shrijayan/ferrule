//! Model and params shapes reduced from knock-mgmt (`tests/model_shapes/<shape>/`), each emitted
//! as Stainless does. Every file under `tests/golden/model-shapes/<shape>/src/` is compared byte
//! for byte with the generated one; the other generated files are not pinned here. The expected
//! files follow the oracle SDKs named in `tests/model_shapes/README.md`.

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
    let root = dir().join("golden/model-shapes").join(shape);
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

fn generated(shape: &str) -> BTreeMap<String, String> {
    let spec = fs::read(dir().join("model_shapes").join(shape).join("annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None)
        .unwrap_or_else(|e| panic!("{shape} does not generate: {e}"))
        .into_iter()
        .collect()
}

fn check(shape: &str) {
    let actual = generated(shape);
    let expected = expected(shape);
    assert!(!expected.is_empty(), "no golden files for {shape}");
    for (path, want) in &expected {
        let got = actual.get(path).unwrap_or_else(|| panic!("{path} is not generated"));
        assert_eq!(got, want, "{path}");
    }
}

/// Union variants named after a schema spell its words as Stainless capitalises class names,
/// default initialisms upper-case (knock-mgmt `EmailHtmlBlock` is `VisualBlockEmailHTMLBlock`).
#[test]
fn variants_named_after_a_schema_spell_initialisms_upper_case() {
    check("variant_names");
}

/// A titled array variant beside a string names its items `<field><title>` (knock-mgmt
/// `RequestTemplate.headers`: `HeadersRequestTemplateHeadersArray`).
#[test]
fn a_titled_array_variant_names_its_items_after_the_title() {
    check("titled_array_variant");
}
