//! A resource with only subresources (`tests/container_resource/`, groq-python `chat.py`)
//! imports nothing its methods would need.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

#[test]
fn a_resource_without_methods_imports_only_what_it_uses() {
    let spec = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/container_resource/annotated.json"))
        .expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    let files = code_files(&forge, None).expect("generates");
    let chat = files
        .iter()
        .find(|(p, _)| p == "src/fx/resources/chat/chat.py")
        .map(|(_, t)| t.as_str())
        .expect("chat.py");
    let head = "from __future__ import annotations\n\nfrom ..._compat import cached_property\n";
    assert!(chat.contains(head), "{chat}");
    assert!(!chat.contains("httpx"), "{chat}");
    assert!(!chat.contains("NotGiven"), "{chat}");
}
