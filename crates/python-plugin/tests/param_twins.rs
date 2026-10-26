//! `TypedDict` twins of the named models a request uses (`tests/param_twins/`), as
//! knock-mgmt-python writes them: `chat_template_param.py` and `shared_params/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn generated() -> BTreeMap<String, String> {
    let spec =
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/param_twins/annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).expect("generates").into_iter().collect()
}

#[test]
fn a_shared_model_in_a_request_has_a_shared_params_twin() {
    let files = generated();
    let twin = files
        .get("src/fx/types/shared_params/model_ref.py")
        .expect("model_ref twin");
    let want = r#"# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing_extensions import Required, Annotated, TypedDict

from ..._utils import PropertyInfo

__all__ = ["ModelRef"]


class ModelRef(TypedDict, total=False):
    """A model, by provider."""

    id: Required[str]

    provider_id: Required[Annotated[str, PropertyInfo(alias="providerID")]]
"#;
    assert_eq!(twin, want);
    let init = files
        .get("src/fx/types/shared_params/__init__.py")
        .expect("shared_params");
    assert!(init.contains("from .model_ref import ModelRef as ModelRef\n"), "{init}");
    // The response keeps the model.
    assert!(files.contains_key("src/fx/types/shared/model_ref.py"));
}

#[test]
fn a_resource_model_in_a_request_has_a_param_twin() {
    let files = generated();
    let twin = files
        .get("src/fx/types/chat_template_param.py")
        .expect("chat_template twin");
    for want in [
        "from .shared_params.model_ref import ModelRef\n",
        "__all__ = [\"ChatTemplateParam\", \"Settings\"]\n",
        "class Settings(TypedDict, total=False):\n    \"\"\"How the template renders.\"\"\"\n\n    compact: bool\n",
        // A field takes its model's description, as in the model file.
        "class ChatTemplateParam(TypedDict, total=False):\n    markdown_body: Required[str]\n\n    model: ModelRef\n    \"\"\"A model, by provider.\"\"\"\n\n    settings: Settings\n",
    ] {
        assert!(twin.contains(want), "{want}\n---\n{twin}");
    }
    let params = files.get("src/fx/types/session_create_params.py").expect("params");
    for want in [
        "from .shared_params.model_ref import ModelRef\n",
        "from .chat_template_param import ChatTemplateParam\n",
        "    model: Required[ModelRef]\n",
        "    template: ChatTemplateParam\n",
    ] {
        assert!(params.contains(want), "{want}\n---\n{params}");
    }
    let resource = files.get("src/fx/resources/sessions.py").expect("sessions.py");
    for want in [
        "from ..types.shared_params.model_ref import ModelRef as SharedParamsModelRef\n",
        "        model: SharedParamsModelRef,\n",
        "        template: ChatTemplateParam | Omit = omit,\n",
        "    ) -> ModelRef:\n",
    ] {
        assert!(resource.contains(want), "{want}\n---\n{resource}");
    }
    let init = files.get("src/fx/types/__init__.py").expect("types init");
    assert!(
        init.contains("from .chat_template_param import ChatTemplateParam as ChatTemplateParam\n"),
        "{init}"
    );
}
