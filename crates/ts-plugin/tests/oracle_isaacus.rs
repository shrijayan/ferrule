//! isaacus-typescript (0.23.0, `c56af182`) regenerated from its config oracle, byte for byte
//! after the branding substitutions of `crates/ts-plugin/BRANDING.md`. Skipped when the oracles
//! are not cached (`oracles/cache/isaacus-config`, `.cache/ts-oracles/isaacus-typescript`).

mod support;

use ferrule_ts_plugin::generate;

/// Generated files that differ from the oracle on purpose, with the reason.
const DELIBERATE: &[(&str, &str)] = &[];

/// Oracle files the plugin does not write, and why.
const NOT_EMITTED: &[(&str, &str)] = &[
    ("CHANGELOG.md", "written by release-please, not generated"),
    (
        "scripts/utils/upload-artifact.sh",
        "uploads to Stainless storage; BRANDING.md",
    ),
    ("packages/mcp-server/", "the MCP server package is not generated yet"),
];

#[test]
fn isaacus_matches_the_oracle() {
    let Some(o) = support::oracle(
        "isaacus-typescript",
        "isaacus-config",
        "stainless/openapi.yml",
        "config/stainless.yaml",
    ) else {
        return;
    };
    let files = generate(&o.client, &o.meta, &o.extras);
    support::dump("isaacus", &files);
    let (same, differ) = support::compare(&o, &files);
    eprintln!("isaacus-typescript: {}/{} identical", same.len(), files.len());
    let unexpected: Vec<&String> = differ
        .iter()
        .filter(|p| !DELIBERATE.iter().any(|(d, _)| d == p))
        .collect();
    assert!(unexpected.is_empty(), "differ from the oracle: {unexpected:#?}");
    for (p, _) in DELIBERATE {
        assert!(differ.iter().any(|d| d == p), "{p} is listed as deliberate but matches");
    }
    // Every other oracle file is the runtime (rendered by ferrule_runtime) or listed above.
    let runtime = support::runtime_files();
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
