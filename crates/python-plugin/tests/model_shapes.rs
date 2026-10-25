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

/// A union of a string and a titled object is an alias named after the field, its object
/// `<alias><title>` (knock-mgmt `workflows.run`: `Recipient`, `RecipientObjectRecipientReference`);
/// a nullable one is `Optional[<alias>]`, and a list of one that can be a string is
/// `SequenceNotStr[<alias>]`.
#[test]
fn a_union_of_a_string_and_a_titled_object_is_an_alias() {
    check("string_or_object");
    let files = generated("string_or_object");
    let resource = files.get("src/fx/resources/workflows.py").expect("workflows.py");
    for arg in [
        "        recipients: SequenceNotStr[workflow_run_params.Recipient],\n",
        "        actor: Optional[workflow_run_params.Actor] | Omit = omit,\n",
    ] {
        assert_eq!(resource.matches(arg).count(), 2, "{arg}{resource}");
    }
}

/// A model field named `schema` would shadow `BaseModel.schema`: Stainless names it `schema_`
/// with an alias (knock-mgmt `MessageTypeJsonField.settings.schema_`, imagekit
/// `CustomMetadataField.schema_`). A params `TypedDict` keeps `schema`.
#[test]
fn a_model_field_named_schema_is_schema_underscore() {
    check("reserved_field_name");
    let files = generated("reserved_field_name");
    let params = files.get("src/fx/types/field_update_params.py").expect("params");
    assert!(params.contains("\n    schema: str\n"), "{params}");
}

/// The items of a list are named in the singular, `-es` plurals included (knock-mgmt `branches`
/// is `Branch`), also in a map of lists (knock-mgmt `example_payloads` is `ExamplePayload`).
#[test]
fn list_items_are_named_in_the_singular() {
    check("plural_items");
}

/// An object with properties that also takes other keys types them as `__pydantic_extra__` and
/// a `__getattr__` stub (knock-mgmt `SourceEnvironmentSettings.settings`).
#[test]
fn an_object_with_extra_properties_declares_pydantic_extra() {
    check("extra_properties");
}
