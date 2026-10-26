//! `custom_casings` and `standalone_api` (`tests/standalone/`, reduced from the isaacus oracle):
//! the expected text is the oracle's, with `isaacus` read as `acme`.

use std::fs;
use std::path::Path;

use ferrule_python_plugin::PythonPlugin;
use ferrule_python_plugin::emit::common::HEADER;
use ferrule_python_plugin::plugin::to_model;
use serde_json::Value;

fn annotated() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/standalone/annotated.json");
    serde_json::from_slice(&fs::read(path).expect("fixture")).expect("JSON")
}

fn forge_of(doc: &Value) -> forge::Forge {
    serde_json::from_value(doc.clone()).expect("a document Forge reads")
}

fn files() -> Vec<(String, String)> {
    PythonPlugin::default().files(&forge_of(&annotated())).expect("emits")
}

fn get<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("no file {path}"), |(_, t)| t.as_str())
}

fn has(files: &[(String, String)], path: &str) -> bool {
    files.iter().any(|(p, _)| p == path)
}

#[test]
fn spells_class_names_with_custom_casings() {
    let f = files();
    let qa = get(&f, "src/acme/resources/extractions/qa/qa.py");
    assert!(qa.contains("class QAResource(SyncAPIResource):"), "{qa}");
    assert!(qa.contains("class AsyncQAResourceWithStreamingResponse:"), "{qa}");
    assert!(
        get(&f, "src/acme/types/extractions/qa/qa_create_params.py")
            .contains("class QACreateParams(TypedDict, total=False):")
    );
    assert!(get(&f, "src/acme/types/ilgs/v1/id_number.py").contains("class IDNumber(BaseModel):"));
    let client = get(&f, "src/acme/_client.py");
    assert!(client.contains("class AcmeQA(SyncAPIClient):"), "{client}");
    assert!(client.contains("class AsyncAcmeQA(AsyncAPIClient):"), "{client}");
    assert!(client.contains("raise AcmeQAError("), "{client}");
    let tests = get(&f, "tests/api_resources/extractions/test_qa.py");
    assert!(
        tests.contains("class TestQA:") && tests.contains("class TestAsyncQA:"),
        "{tests}"
    );
}

#[test]
fn lays_out_a_standalone_api_as_its_own_package() {
    let f = files();
    assert!(!has(&f, "src/acme/resources/extractions/qa.py"));
    let init = get(&f, "src/acme/resources/extractions/qa/__init__.py");
    assert!(init.contains("from .qa import (") && init.contains("    \"AsyncQAResourceWithStreamingResponse\",\n"));
    let parent_init = get(&f, "src/acme/resources/extractions/__init__.py");
    assert!(!parent_init.contains("QA"), "{parent_init}");
    assert!(parent_init.contains("\"ExtractionsResource\","));
    let parent = get(&f, "src/acme/resources/extractions/extractions.py");
    assert!(parent.contains("from .qa.qa import"), "{parent}");
    assert!(parent.contains("def qa(self) -> QAResource:"), "{parent}");
    let types = get(&f, "src/acme/types/extractions/qa/__init__.py");
    assert!(types.contains("from .qa_create_params import QACreateParams as QACreateParams\n"));
    assert!(
        types
            .contains("from .answer_extraction_response import AnswerExtractionResponse as AnswerExtractionResponse\n")
    );
    assert_eq!(
        get(&f, "src/acme/types/extractions/__init__.py"),
        format!("{HEADER}\n\nfrom __future__ import annotations\n")
    );
    let resource = get(&f, "src/acme/resources/extractions/qa/qa.py");
    assert!(resource.contains("from ....types.extractions.qa import qa_create_params\n"));
    assert!(
        resource.contains("from ....types.extractions.qa.answer_extraction_response import AnswerExtractionResponse\n")
    );
    assert_eq!(
        get(&f, "tests/api_resources/extractions/qa/__init__.py"),
        format!("{HEADER}\n")
    );
    assert!(has(&f, "tests/api_resources/extractions/__init__.py"));
    assert!(
        get(&f, "tests/api_resources/extractions/test_qa.py")
            .contains("from acme.types.extractions.qa import AnswerExtractionResponse\n")
    );
}

#[test]
fn emits_a_standalone_api_without_methods_as_an_empty_package() {
    let f = files();
    assert_eq!(get(&f, "src/acme/resources/ilgs/v1/__init__.py"), format!("{HEADER}\n"));
    let under_ilgs: Vec<&str> = f
        .iter()
        .map(|(p, _)| p.as_str())
        .filter(|p| p.starts_with("src/acme/resources/ilgs"))
        .collect();
    assert_eq!(
        under_ilgs,
        [
            "src/acme/resources/ilgs/v1/__init__.py",
            "src/acme/resources/ilgs/v1/api.md"
        ]
    );
    assert!(
        !get(&f, "src/acme/resources/__init__.py")
            .to_lowercase()
            .contains("ilgs")
    );
    assert!(!get(&f, "src/acme/_client.py").to_lowercase().contains("ilgs"));
    assert_eq!(
        get(&f, "src/acme/types/ilgs/__init__.py"),
        format!("{HEADER}\n\nfrom __future__ import annotations\n")
    );
    let types = get(&f, "src/acme/types/ilgs/v1/__init__.py");
    assert!(types.contains("from .document import Document as Document\n"));
    assert!(types.contains("from .id_number import IDNumber as IDNumber\n"));
    assert_eq!(
        get(&f, "tests/api_resources/ilgs/v1/__init__.py"),
        format!("{HEADER}\n")
    );
    assert!(!has(&f, "tests/api_resources/ilgs/__init__.py"));
}

#[test]
fn links_standalone_apis_from_api_md_and_lists_them_in_their_own() {
    let f = files();
    assert_eq!(
        get(&f, "api.md"),
        "# Extractions

## [QA](src/acme/resources/extractions/api.md)

# Enrichments

Types:

```python
from acme.types import EnrichmentCreateResponse
```

Methods:

- <code title=\"post /enrichments\">client.enrichments.<a href=\"./src/acme/resources/enrichments.py\">create</a>(\\*\\*<a href=\"src/acme/types/enrichment_create_params.py\">params</a>) -> <a href=\"./src/acme/types/enrichment_create_response.py\">EnrichmentCreateResponse</a></code>

# ILGS

## [v1](src/acme/resources/api.md)
"
    );
    assert_eq!(
        get(&f, "src/acme/resources/extractions/qa/api.md"),
        "# QA

Types:

```python
from acme.types.extractions.qa import AnswerExtractionResponse
```

Methods:

- <code title=\"post /extractions/qa\">client.extractions.qa.<a href=\"./src/acme/resources/extractions/qa/qa.py\">create</a>(\\*\\*<a href=\"src/acme/types/extractions/qa/qa_create_params.py\">params</a>) -> <a href=\"./src/acme/types/extractions/qa/answer_extraction_response.py\">AnswerExtractionResponse</a></code>
"
    );
    assert_eq!(
        get(&f, "src/acme/resources/ilgs/v1/api.md"),
        "# v1\n\nTypes:\n\n```python\nfrom acme.types.ilgs.v1 import Document, IDNumber\n```\n"
    );
}

/// Casings are spliced into class names, so the adapter accepts identifier characters only.
#[test]
fn rejects_a_casing_that_is_not_an_identifier_fragment() {
    let mut doc = annotated();
    let op = doc
        .pointer_mut("/paths/~1enrichments/post")
        .and_then(Value::as_object_mut)
        .expect("operation");
    op.insert(
        "x-ferrule-casings".into(),
        serde_json::json!({ "qa": { "pascal": "Q\"A" } }),
    );
    let err = to_model(&forge_of(&doc)).expect_err("rejected");
    assert_eq!(err.code.as_str(), "FER101");
    assert!(err.pointer.ends_with("/x-ferrule-casings"), "{}", err.pointer);
}

/// `types/extractions/__init__.py` is empty, as the oracle has it, so it loads nothing below it:
/// the rebuild of a recursive model in `types/extractions/qa/` imports that model's module
/// itself rather than reaching it through `extractions`.
#[test]
fn rebuilds_a_recursive_model_of_a_standalone_api_from_its_own_module() {
    let mut doc = annotated();
    doc.pointer_mut("/components/schemas/AnswerExtractionResponse/properties")
        .and_then(Value::as_object_mut)
        .expect("properties")
        .insert(
            "children".into(),
            serde_json::json!({ "type": "array", "items": { "$ref": "#/components/schemas/AnswerExtractionResponse" } }),
        );
    let f = PythonPlugin::default().files(&forge_of(&doc)).expect("emits");
    let types = get(&f, "src/acme/types/__init__.py");
    assert!(
        types.contains("from .extractions.qa import answer_extraction_response\n"),
        "{types}"
    );
    assert!(
        types.contains(
            "    answer_extraction_response.AnswerExtractionResponse.model_rebuild(_parent_namespace_depth=0)\n"
        ),
        "{types}"
    );
    assert!(
        !types.contains("extractions.qa.answer_extraction_response.AnswerExtractionResponse"),
        "{types}"
    );
    assert_eq!(
        get(&f, "src/acme/types/extractions/__init__.py"),
        format!("{HEADER}\n\nfrom __future__ import annotations\n")
    );
}

/// A spelling would start `QAResource` as `123Resource`; only a word led by a digit (`3ds`) may
/// keep a digit first, as it cannot start a class name either way.
#[test]
fn rejects_a_casing_that_puts_a_digit_first() {
    let with_casings = |casings: Value| {
        let mut doc = annotated();
        for op in ["/paths/~1enrichments/post", "/paths/~1extractions~1qa/post"] {
            doc.pointer_mut(op)
                .and_then(Value::as_object_mut)
                .expect("operation")
                .insert("x-ferrule-casings".into(), casings.clone());
        }
        to_model(&forge_of(&doc))
    };
    let err = with_casings(serde_json::json!({ "qa": { "pascal": "123" } })).expect_err("rejected");
    assert_eq!(err.code.as_str(), "FER101");
    assert!(err.pointer.ends_with("/x-ferrule-casings"), "{}", err.pointer);
    assert!(with_casings(serde_json::json!({ "3ds": { "pascal": "3DS" } })).is_ok());
}
