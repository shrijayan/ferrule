//! Settings each TypeScript target (`typescript`, `node`) sets for itself, on the petstore
//! fixture: a layout follows its own target's settings, never the other target's.

mod support;

use std::path::Path;

use ferrule_ts_plugin::{Layout, ts_settings};

/// The `typescript` target publishing by OIDC.
fn with_oidc_typescript_target(config: &str) -> String {
    config.replace(
        "    publish:\n      npm: true\n",
        "    publish:\n      npm:\n        auth_method: oidc\n",
    )
}

#[test]
fn npm_oidc_is_read_from_the_layouts_own_target() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/petstore");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let config = with_oidc_typescript_target(&read("stainless.yml"));
    assert!(
        config.contains("auth_method: oidc"),
        "the fixture's publish block moved"
    );
    let (client, meta, _) = support::load(&read("openapi.yml"), &config).expect("the fixture maps to the model");
    assert!(ts_settings(&client, &meta, Layout::Typescript).npm_oidc);
    assert!(
        !ts_settings(&client, &meta, Layout::Node).npm_oidc,
        "the node layout took the typescript target's OIDC"
    );
}
