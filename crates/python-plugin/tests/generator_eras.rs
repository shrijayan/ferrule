//! Older generator styles an adopted SDK keeps (`tests/generator_eras/`, reduced from honcho and
//! knock): `x-ferrule-style` on every operation switches client resources to eager attributes,
//! paths to f-strings and the optional-argument sentinel to `NOT_GIVEN`. Without it, or with
//! today's values spelled out, the output is today's. The files under
//! `tests/golden/generator-eras/` are compared byte for byte.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::emit::project::FileTree;
use ferrule_python_plugin::plugin::code_files;
use serde_json::{Value, json};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// The fixture with `style` as `x-ferrule-style` on every operation (none when `None`).
fn generate(style: Option<&Value>) -> Result<FileTree, String> {
    generate_from("generator_eras/annotated.json", style)
}

/// [`generate`] for the annotated spec at `fixture` under `tests/`.
fn generate_from(fixture: &str, style: Option<&Value>) -> Result<FileTree, String> {
    let spec = fs::read(dir().join(fixture)).expect("fixture");
    let mut doc: Value = serde_json::from_slice(&spec).expect("JSON");
    if let Some(style) = style {
        for item in doc["paths"].as_object_mut().expect("paths").values_mut() {
            for op in item.as_object_mut().expect("path item").values_mut() {
                op["x-ferrule-style"] = style.clone();
            }
        }
    }
    let forge: forge::Forge = serde_json::from_value(doc).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).map_err(|e| format!("{} {}: {}", e.code, e.pointer, e.detail))
}

fn assert_golden(files: &FileTree, golden: &str, paths: &[&str]) {
    for path in paths {
        let want =
            fs::read_to_string(dir().join("golden/generator-eras").join(golden).join(path)).expect("golden file");
        let got = files.iter().find(|(p, _)| p == path).map(|(_, t)| t.as_str());
        assert_eq!(got, Some(want.as_str()), "{golden}/{path}");
    }
}

#[test]
fn keeps_todays_style_when_spelled_out() {
    let today = generate(None).expect("generates");
    let spelled = json!({"sentinel": "omit", "resources": "lazy", "paths": "path_template"});
    assert_eq!(generate(Some(&spelled)).expect("generates"), today);
}

#[test]
fn sets_resources_in_init_and_writes_paths_as_f_strings() {
    let style = json!({"resources": "eager", "paths": "fstring"});
    let files = generate(Some(&style)).expect("generates");
    assert_golden(
        &files,
        "eager-fstring",
        &[
            "src/fx/_client.py",
            "src/fx/resources/workspaces/workspaces.py",
            "src/fx/resources/workspaces/peers.py",
        ],
    );
    let today = generate(None).expect("generates");
    let keys = |t: &FileTree| t.iter().find(|(p, _)| p == "src/fx/resources/keys.py").cloned();
    assert_eq!(
        keys(&files),
        keys(&today),
        "a method without path parameters is unchanged"
    );
}

#[test]
fn defaults_optional_arguments_to_not_given() {
    let style = json!({"sentinel": "NOT_GIVEN", "resources": "eager", "paths": "fstring"});
    let files = generate(Some(&style)).expect("generates");
    assert_golden(
        &files,
        "not-given",
        &[
            "src/fx/__init__.py",
            "src/fx/_client.py",
            "src/fx/resources/keys.py",
            "src/fx/resources/workspaces/workspaces.py",
        ],
    );
}

#[test]
fn rejects_an_unknown_style() {
    for style in [
        json!({"sentinel": "none"}),
        json!({"resources": true}),
        json!({"colour": "blue"}),
        json!("eager"),
    ] {
        let err = generate(Some(&style)).expect_err("malformed");
        assert!(err.starts_with("FER101 "), "{style}: {err}");
        assert!(err.contains("/x-ferrule-style"), "{style}: {err}");
    }
}

/// A runtime whose `_files.py` has no `deepcopy_with_paths` copies a multipart body with
/// `_utils.deepcopy_minimal` (`multipart: deepcopy_minimal`), as honcho-python-core
/// `resources/workspaces/sessions/messages.py` does; the fixture is `request_bodies/multipart`.
#[test]
fn copies_a_multipart_body_with_deepcopy_minimal() {
    let style = json!({"multipart": "deepcopy_minimal", "paths": "fstring"});
    let files = generate_from("request_bodies/multipart/annotated.json", Some(&style)).expect("generates");
    let resource = files
        .iter()
        .find(|(p, _)| p == "src/fx/resources/messages.py")
        .map(|(_, t)| t.as_str())
        .expect("messages.py");
    let sync = r#"        if not session_id:
            raise ValueError(f"Expected a non-empty value for `session_id` but received {session_id!r}")
        body = deepcopy_minimal(
            {
                "file": file,
                "peer_id": peer_id,
            }
        )
        files = extract_files(cast(Mapping[str, object], body), paths=[["file"]])
        # It should be noted that the actual Content-Type header that will be
        # sent to the server will contain a `boundary` parameter, e.g.
        # multipart/form-data; boundary=---abc--
        extra_headers = {"Content-Type": "multipart/form-data", **(extra_headers or {})}
        return self._post(
            f"/sessions/{session_id}/messages/upload",
"#;
    assert!(resource.contains(sync), "{resource}");
    assert!(
        resource
            .contains("from .._utils import extract_files, maybe_transform, deepcopy_minimal, async_maybe_transform\n"),
        "{resource}"
    );
    assert!(!resource.contains("from .._files import"), "{resource}");
    let today = generate_from("request_bodies/multipart/annotated.json", None).expect("generates");
    let spelled = json!({"multipart": "deepcopy_with_paths"});
    let spelled = generate_from("request_bodies/multipart/annotated.json", Some(&spelled)).expect("generates");
    assert_eq!(spelled, today);
}
