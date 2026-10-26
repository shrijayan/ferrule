//! A named schema with no type (`tests/any_model/`, reduced from the opencode SDK's compaction
//! payload) is an `object` alias; a named `type: object` stays a map of any values.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

#[test]
fn a_typeless_model_is_an_object_alias() {
    let spec =
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/any_model/annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    let files: BTreeMap<String, String> = code_files(&forge, None).expect("generates").into_iter().collect();
    let payload = files.get("src/fx/types/shared/payload.py").expect("payload.py");
    assert!(payload.ends_with("Payload: TypeAlias = object\n"), "{payload}");
    let empty = files.get("src/fx/types/shared/empty.py").expect("empty.py");
    assert!(empty.ends_with("Empty: TypeAlias = Dict[str, object]\n"), "{empty}");
}
