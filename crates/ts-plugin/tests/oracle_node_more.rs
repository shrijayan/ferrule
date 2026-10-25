//! lumaai-typescript (lumaai-node 1.21.0, `4dbfae0f`, 2026-04) and honcho-typescript
//! (honcho-node-core `91d25806`, 2025-10), both the `node` layout, against their config oracles.
//! Neither is regenerated whole yet (older generations; honcho has pagination and an MCP server
//! package): each test reports identical/total and fails when fewer files match than before (the
//! floor). Skipped when the oracles are not cached.

mod support;

use ferrule_ts_plugin::{Layout, generate};

fn report(name: &str, o: &support::Oracle, floor: usize) {
    assert_eq!(Layout::of(&o.client), Layout::Node);
    let files = generate(&o.client, &o.meta, &o.extras);
    support::dump(name, &files);
    let (same, differ) = support::compare(o, &files);
    eprintln!("{name}: {}/{} identical; differ: {differ:#?}", same.len(), files.len());
    assert!(
        same.len() >= floor,
        "{name}: {} files identical, fewer than the {floor} that matched before",
        same.len()
    );
}

#[test]
fn lumaai_report() {
    let Some(o) = support::oracle(
        "lumaai-typescript",
        "lumaai-config",
        "openapi.yaml",
        "openapi.stainless.yml",
    ) else {
        return;
    };
    report("lumaai", &o, 49);
}

#[test]
fn honcho_report() {
    let Some(o) = support::oracle(
        "honcho-typescript",
        "honcho-config",
        "openapi.json",
        "openapi.stainless.yml",
    ) else {
        return;
    };
    report("honcho", &o, 32);
}
