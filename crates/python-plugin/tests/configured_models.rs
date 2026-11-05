//! Models `stainless.yml` lists are generated wherever Forge reached their schema
//! (`tests/configured_models/`): an error response's model is declared and exported too.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn generated() -> BTreeMap<String, String> {
    let spec = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/configured_models/annotated.json"))
        .expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).expect("generates").into_iter().collect()
}

#[test]
#[ignore = "Forge builds schemas only 2XX responses reach (docs/forge-gaps.md G29, docs/HUMAN-ACTIONS.md 16)"]
fn a_model_only_an_error_response_reaches_is_generated() {
    let files = generated();
    let model = files
        .get("src/fx/types/not_found_error.py")
        .expect("not_found_error.py");
    let want = r#"# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Optional

from .._models import BaseModel

__all__ = ["NotFoundError"]


class NotFoundError(BaseModel):
    """The body of a 404."""

    message: str

    resource: Optional[str] = None
"#;
    assert_eq!(model, want);
    let init = files.get("src/fx/types/__init__.py").expect("types/__init__.py");
    assert!(
        init.contains("from .not_found_error import NotFoundError as NotFoundError\n"),
        "{init}"
    );
}
