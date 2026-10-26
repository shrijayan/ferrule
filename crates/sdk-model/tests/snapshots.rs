//! The neutral model of two real APIs, committed for review: OneBusAway (the Python plugin's
//! annotated fixture) and isaacus (its config oracle, converted at test time; skipped when
//! `oracles/cache` is not fetched). Both are Apache-2.0.
//!
//! Snapshots are read, never written. A model change regenerates them with
//! `ferrule-gen model <annotated.json> > crates/sdk-model/tests/snapshots/<name>.json.snap`, and
//! the diff is reviewed like any golden change.

mod common;

use std::path::PathBuf;

fn snapshot(name: &str) -> (PathBuf, String) {
    let path = common::root().join(format!("crates/sdk-model/tests/snapshots/{name}.json.snap"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    (path, text)
}

fn model_json(annotated: &str) -> String {
    let client = ferrule_sdk_model::forge_to_model(&common::forge(annotated)).expect("converts");
    ferrule_sdk_model::to_json(client.as_ref())
}

fn assert_matches(name: &str, annotated: &str) {
    let (path, expected) = snapshot(name);
    let actual = model_json(annotated);
    assert!(
        actual == expected,
        "the model differs from {}; if the change is intended, regenerate it with ferrule-gen model and review the diff",
        path.display()
    );
}

#[test]
fn onebusaway_matches_the_committed_snapshot() {
    let fixtures = common::python_fixtures();
    let (_, oba) = fixtures.iter().find(|(n, _)| n == "onebusaway").expect("fixture");
    assert_matches("onebusaway", oba);
}

#[test]
fn isaacus_matches_the_committed_snapshot() {
    let pair = common::config_pairs()
        .into_iter()
        .find(|p| p.name == "isaacus-config")
        .expect("isaacus-config in the manifest");
    let Some(annotated) = common::annotated(&pair) else {
        eprintln!("skipped isaacus-config: oracle files not cached");
        return;
    };
    assert_matches("isaacus", &annotated);
}

#[test]
fn the_model_is_the_same_on_every_run() {
    let fixtures = common::python_fixtures();
    for (name, json) in &fixtures {
        assert_eq!(model_json(json), model_json(json), "{name}");
    }
}
