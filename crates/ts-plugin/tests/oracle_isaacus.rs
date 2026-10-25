//! isaacus-typescript (0.23.0, `c56af182`) regenerated from its config oracle, byte for byte
//! after the branding substitution of the header line. Skipped when the oracles are not cached
//! (`oracles/cache/isaacus-config`, `.cache/ts-oracles/isaacus-typescript`).

mod support;

use ferrule_ts_plugin::generate;

/// Generated files that differ from the oracle on purpose, with the reason.
const DELIBERATE: &[(&str, &str)] = &[];

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
}
