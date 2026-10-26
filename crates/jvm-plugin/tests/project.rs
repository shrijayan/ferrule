//! The project files carry the SDK's own values, not OneBusAway's, for an API that is not
//! OneBusAway (`tests/synthetic/{openapi,stainless}.yml`).

mod common;

use std::path::Path;

use ferrule_jvm_plugin::{Flavor, JvmMeta, generate};

fn file(flavor: Flavor, meta: &JvmMeta, path: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("synthetic input");
    let annotated = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml"))
        .expect("converts")
        .output;
    let out = generate(&common::model(&annotated), flavor, meta).expect("generates");
    out.files
        .into_iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is not generated"), |(_, text)| text)
}

#[test]
fn security_md_names_the_sdks_own_contact() {
    let meta = JvmMeta {
        contact: Some("security@acme.test".into()),
        ..JvmMeta::default()
    };
    for flavor in [Flavor::Java, Flavor::Kotlin] {
        let text = file(flavor, &meta, "SECURITY.md");
        assert!(
            text.contains("Please contact security@acme.test for any questions"),
            "{text}"
        );
        assert!(!text.to_lowercase().contains("onebusaway"), "{text}");
    }
}
