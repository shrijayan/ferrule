//! Unwrapped response envelopes (`tests/response_unwrap/`), as ogx-client-python writes them:
//! `settings.unwrap_response_fields` makes a method return its envelope's `data`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn generated() -> BTreeMap<String, String> {
    let spec = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/response_unwrap/annotated.json"))
        .expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).expect("generates").into_iter().collect()
}

#[test]
fn an_unwrapped_method_returns_the_envelope_field() {
    let files = generated();
    let providers = files.get("src/fx/resources/providers.py").expect("providers.py");
    let request = r#"    ) -> ProviderListResponse:
        """List all available providers."""
        return self._get(
            "/providers",
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                post_parser=DataWrapper[ProviderListResponse]._unwrapper,
            ),
            cast_to=cast(Type[ProviderListResponse], DataWrapper[ProviderListResponse]),
        )
"#;
    assert!(providers.contains(request), "{providers}");
    assert!(providers.contains("from typing import Type, cast\n"), "{providers}");
    assert!(
        providers.contains("from .._wrappers import DataWrapper\n"),
        "{providers}"
    );
    let alias = files
        .get("src/fx/types/provider_list_response.py")
        .expect("provider_list_response.py");
    assert!(
        alias.ends_with("ProviderListResponse: TypeAlias = List[ProviderInfo]\n"),
        "{alias}"
    );
    let wrappers = files.get("src/fx/_wrappers.py").expect("_wrappers.py");
    let want = r#"# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Generic, TypeVar

from ._models import GenericModel

__all__ = ["DataWrapper"]

_T = TypeVar("_T")


class DataWrapper(GenericModel, Generic[_T]):
    data: _T

    @staticmethod
    def _unwrapper(obj: "DataWrapper[_T]") -> _T:
        return obj.data
"#;
    assert_eq!(wrappers, want);
}

#[test]
fn a_two_field_envelope_unwraps_to_its_model() {
    let files = generated();
    let sessions = files.get("src/fx/resources/sessions.py").expect("sessions.py");
    for want in [
        "    ) -> SessionInfo:\n",
        "                post_parser=DataWrapper[SessionInfo]._unwrapper,\n",
        "            cast_to=cast(Type[SessionInfo], DataWrapper[SessionInfo]),\n",
    ] {
        assert!(sessions.contains(want), "{want}\n---\n{sessions}");
    }
    assert!(!files.contains_key("src/fx/types/session_retrieve_response.py"));
}
