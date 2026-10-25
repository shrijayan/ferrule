//! onebusaway-typescript (OneBusAway/js-sdk 1.20.0, `f3ca3f47`, the `node` layout) regenerated
//! from its config oracle, byte for byte after the branding substitutions of
//! `crates/ts-plugin/BRANDING.md`. Skipped when the oracles are not cached
//! (`oracles/cache/onebusaway-config`, `oracles/cache/onebusaway-typescript`).

mod support;

use ferrule_ts_plugin::{Layout, generate};

/// Every code file (`src/**`, `api.md`, `tests/api-resources/**`) matches; the project files
/// are not the `node` layout's yet.
const CODE: usize = 64;

#[test]
fn onebusaway_code_matches_the_oracle() {
    let Some(o) = support::oracle(
        "onebusaway-typescript",
        "onebusaway-config",
        "openapi.yml",
        "stainless-config.yml",
    ) else {
        return;
    };
    assert_eq!(Layout::of(&o.client), Layout::Node);
    let files = generate(&o.client, &o.meta, &o.extras);
    support::dump("onebusaway", &files);
    let (same, differ) = support::compare(&o, &files);
    eprintln!("onebusaway-typescript: {}/{} identical", same.len(), files.len());
    let code: Vec<&String> = differ
        .iter()
        .filter(|p| p.starts_with("src/") || p.starts_with("tests/api-resources/") || *p == "api.md")
        .collect();
    assert!(code.is_empty(), "code files differ from the oracle: {code:#?}");
    let matched = same
        .iter()
        .filter(|p| p.starts_with("src/") || p.starts_with("tests/api-resources/") || *p == "api.md")
        .count();
    assert_eq!(matched, CODE);
}
