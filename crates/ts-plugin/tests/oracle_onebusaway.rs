//! onebusaway-typescript (OneBusAway/js-sdk 1.20.0, `f3ca3f47`, the `node` layout) regenerated
//! from its config oracle, byte for byte after the branding substitutions of
//! `crates/ts-plugin/BRANDING.md`. Skipped when the oracles are not cached
//! (`oracles/cache/onebusaway-config`, `oracles/cache/onebusaway-typescript`).

mod support;

use ferrule_ts_plugin::{Layout, generate};

/// Generated files that differ from the oracle on purpose, with the reason.
const DELIBERATE: &[(&str, &str)] = &[(
    "README.md",
    "the oracle's \"Examples and Sample Code\" section is OneBusAway's own edit: it links the \
     hand-written examples/README.md, which no other node-layout oracle has",
)];

/// Oracle files the plugin does not write, and why.
const NOT_EMITTED: &[(&str, &str)] = &[
    ("CHANGELOG.md", "written by release-please, not generated"),
    (
        "scripts/utils/upload-artifact.sh",
        "uploads to Stainless storage; BRANDING.md",
    ),
    (
        "package-lock.json",
        "added by OneBusAway: the generated project uses yarn.lock (lumaai, honcho)",
    ),
    (
        "examples/",
        "OneBusAway's hand-written example scripts; examples/.keep is generated",
    ),
];

#[test]
fn onebusaway_matches_the_oracle() {
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
    let unexpected: Vec<&String> = differ
        .iter()
        .filter(|p| !DELIBERATE.iter().any(|(d, _)| d == p))
        .collect();
    assert!(unexpected.is_empty(), "differ from the oracle: {unexpected:#?}");
    for (p, _) in DELIBERATE {
        assert!(differ.iter().any(|d| d == p), "{p} is listed as deliberate but matches");
    }
    // Every other oracle file is the runtime (rendered by ferrule_runtime) or listed above.
    let runtime = support::runtime_files(Layout::Node);
    let missing: Vec<String> = support::tree(&o.sdk)
        .into_iter()
        .filter(|p| !files.iter().any(|(f, _)| f == p))
        .filter(|p| !runtime.contains(p))
        .filter(|p| {
            !NOT_EMITTED
                .iter()
                .any(|(n, _)| p == n || (n.ends_with('/') && p.starts_with(n)))
        })
        .collect();
    assert!(missing.is_empty(), "not emitted: {missing:#?}");
}
