//! A method returning a named union model (`tests/union_response/`, knock-mgmt-python
//! `audiences`): the request casts it, since a union is not a type the helpers take.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

const COMMENT: &str = "  # Union types cannot be passed in as arguments in the type system";

#[test]
fn casts_a_union_response_and_page_item() {
    let spec = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/union_response/annotated.json"))
        .expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    let files = code_files(&forge, None).expect("generates");
    let text = files
        .iter()
        .find(|(p, _)| p == "src/fx/resources/audiences.py")
        .map(|(_, t)| t.as_str())
        .expect("audiences.py");
    assert!(
        text.contains("        return cast(\n            Audience,\n            self._get(\n"),
        "{text}"
    );
    assert!(
        text.contains("        return cast(\n            Audience,\n            await self._get(\n"),
        "{text}"
    );
    let retrieve = format!("                cast_to=cast(Any, Audience),{COMMENT}\n            ),\n        )\n");
    assert_eq!(text.matches(&retrieve).count(), 2, "{text}");
    let list = format!("            model=cast(Any, Audience),{COMMENT}\n        )\n");
    assert_eq!(text.matches(&list).count(), 2, "{text}");
    assert!(text.contains("from typing import Any, cast\n"), "{text}");
    // A stream of a union is not cast as a whole (the Stainless opencode SDK's `event.list`).
    let watch = format!("            cast_to=cast(Any, Audience),{COMMENT}\n            stream=True,\n");
    assert_eq!(text.matches(&watch).count(), 2, "{text}");
}
