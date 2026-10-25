//! Request bodies that are not a plain object (`tests/request_bodies/<shape>/`), each emitted as
//! Stainless does. Every file under `tests/golden/request-bodies/<shape>/src/` is compared byte for
//! byte with the generated one; the other generated files are not pinned here. The expected files
//! follow the oracle SDKs named in `tests/request_bodies/README.md`.

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
    let root = dir().join("golden/request-bodies").join(shape);
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

fn generated(shape: &str) -> BTreeMap<String, String> {
    let spec = fs::read(dir().join("request_bodies").join(shape).join("annotated.json")).expect("fixture");
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

/// `anyOf: [$ref Object, null]` (honcho `workspaces.list`): the object's fields are keyword args.
#[test]
fn a_nullable_object_body_becomes_keyword_arguments() {
    check("nullable_object");
}

/// A map or array body (honcho `sessions.peers.add/remove`): one `body` keyword argument.
#[test]
fn a_map_or_array_body_is_one_body_argument() {
    check("map_and_array");
}

/// A union of objects (knock-mgmt `guides.activate`, imagekit `accounts.origins.create`): an
/// `@overload` per variant, `@required_args`, and a `TypedDict` per variant.
#[test]
fn a_union_body_is_one_overload_per_variant() {
    check("union_of_objects");
}
