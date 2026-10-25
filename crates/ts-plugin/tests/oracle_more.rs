//! imagekit-typescript (imagekit-nodejs `9161f99f`, 2026-05-17) and knock-mgmt-typescript
//! (knock-mgmt-node 0.32.0, `ca88fdba`) against their config oracles. Neither is regenerated whole
//! yet: each test reports identical/total and fails when fewer files match than before (the
//! floor). knock-mgmt-typescript does not map to the neutral model yet (FER101 on its pagination),
//! so it is skipped with that note. Skipped too when the oracles are not cached.

mod support;

use ferrule_ts_plugin::generate;

fn report(name: &str, o: &support::Oracle, floor: usize) {
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
fn imagekit_report() {
    let Some(o) = support::oracle(
        "imagekit-typescript",
        "imagekit-config",
        "../specs/imagekit-config.yml",
        "stainless-config/main.yaml",
    ) else {
        return;
    };
    report("imagekit", &o, 63);
}

#[test]
fn knock_mgmt_report() {
    let Some(o) = support::oracle(
        "knock-mgmt-typescript",
        "knock-mgmt-config",
        "data/specs/mapi/openapi.yml",
        "data/specs/mapi/stainless.yml",
    ) else {
        return;
    };
    report("knock-mgmt", &o, 0);
}
