//! The generated gem against real Stainless Ruby SDKs, byte for byte after `syntax_tree` and rubocop lay out
//! the `.rbi` and `.rbs` views (as the gem's `rake format` does) and the branding of the `.keep`
//! files. Skipped when `oracles/cache` is not fetched; the `.rbi` and `.rbs` files are compared
//! only when `stree` is available (`$FERRULE_STREE`, see `tests/common`).
//!
//! onebusaway-ruby (Apache-2.0) is the same generation as the proven Python pin: the md5 of its
//! `openapi.yml` and `stainless-config.yml` equal the `.stats.yml` hashes of `ruby-sdk` at
//! `46a574dd`.

mod common;

use std::path::PathBuf;

use ferrule_ruby_plugin::{RubyMeta, RubyOutput, generate};

/// Oracle files that differ on purpose or for a recorded gap, with the reason. The test fails
/// when one of them starts matching, so the list stays true.
const ONEBUSAWAY_KNOWN: &[(&str, &str)] = &[
    (
        "test/onebusaway_sdk/resources/arrival_and_departure_test.rb",
        "Forge gap G18: the path param's `example` (1_75403) is not in the model",
    ),
    (
        "test/onebusaway_sdk/resources/routes_for_agency_test.rb",
        "Forge gap G18: the path param's `example` (40) is not in the model",
    ),
    (
        "test/onebusaway_sdk/resources/schedule_for_route_test.rb",
        "Forge gap G18: the path param's `example` (1_100223) is not in the model",
    ),
];

/// Oracle files that are not generated output: release-please's, hand-written, or the runtime
/// template's (`runtime/ruby`, proven by `runtime/tests/test_render_oracles_ruby.py`).
fn not_generated(path: &str, lib: &str) -> bool {
    const RUNTIME: [&str; 4] = ["errors", "file_part", "internal", "request_options"];
    let runtime = ["lib", "rbi", "sig"].iter().any(|v| {
        let dir = format!("{v}/{lib}/");
        path.strip_prefix(&dir).is_some_and(|rest| {
            rest.starts_with("internal/") || RUNTIME.iter().any(|r| rest.split('.').next() == Some(r))
        })
    });
    let runtime_tests =
        path.starts_with(&format!("test/{lib}/internal/")) || path == format!("test/{lib}/file_part_test.rb");
    runtime || runtime_tests || path == "CHANGELOG.md"
}

/// Project files (README, CI, scripts, gemspec) the plugin does not emit yet.
fn project_file(path: &str) -> bool {
    !(path.starts_with("lib/") || path.starts_with("rbi/") || path.starts_with("sig/") || path.starts_with("test/"))
        && path != "examples/.keep"
}

fn onebusaway() -> Option<(PathBuf, RubyOutput)> {
    let oracle = common::oracle("onebusaway-ruby")?;
    let annotated = common::annotated("onebusaway-config", "openapi.yml", "stainless-config.yml")?;
    let meta = RubyMeta {
        version: Some("1.12.0".into()),
        example: Some((vec!["current_time".into()], "retrieve".into())),
        // The gem name comes from `targets.ruby.gem_name`.
        ..RubyMeta::default()
    };
    let out = generate(&common::model(&annotated), &meta).expect("generates");
    Some((oracle, out))
}

/// Compares the formatted `files` with `oracle`; returns (identical, differing).
fn compare(oracle: &std::path::Path, files: &[(String, String)], known: &[(&str, &str)]) -> (usize, Vec<String>) {
    common::dump("FERRULE_RUBY_DUMP", files);
    let mut identical = 0;
    let mut differing = Vec::new();
    for (path, text) in files {
        let raw = std::fs::read_to_string(oracle.join(path)).unwrap_or_else(|e| panic!("oracle has no {path}: {e}"));
        let expected = common::brand(&raw);
        let is_known = known.iter().find(|(p, _)| p == path);
        match (text == &expected, is_known) {
            (true, None) => identical += 1,
            (true, Some((p, _))) => panic!("{p} is a known difference but now matches; drop it"),
            (false, Some(_)) => {}
            (false, None) => differing.push(path.clone()),
        }
    }
    (identical, differing)
}

#[test]
fn onebusaway_matches_the_oracle() {
    let Some((oracle, out)) = onebusaway() else { return };
    let Some(files) = common::format(&out.files) else {
        eprintln!("skipped: the gem's formatters are not installed (FERRULE_STREE, FERRULE_RUBOCOP)");
        return;
    };
    let (identical, differing) = compare(&oracle, &files, ONEBUSAWAY_KNOWN);
    eprintln!("onebusaway-ruby: {identical}/{} generated files identical", files.len());
    assert!(differing.is_empty(), "differ from the oracle: {differing:?}");
    assert_eq!(identical + ONEBUSAWAY_KNOWN.len(), files.len());
}

#[test]
fn onebusaway_emits_every_generated_file() {
    let Some((oracle, out)) = onebusaway() else { return };
    let lib = &out.runtime.gem_name;
    let missing: Vec<String> = common::walk(&oracle)
        .into_iter()
        .filter(|p| !not_generated(p, lib) && !project_file(p))
        .filter(|p| !out.files.iter().any(|(f, _)| f == p))
        .collect();
    assert!(missing.is_empty(), "not emitted: {missing:?}");
}

#[test]
fn onebusaway_fills_the_runtime_options() {
    let Some((_, out)) = onebusaway() else { return };
    let r = &out.runtime;
    assert_eq!(
        (
            r.gem_name.as_str(),
            r.module_name.as_str(),
            r.array_format.as_str(),
            r.auth_in.as_str()
        ),
        ("onebusaway_sdk", "OnebusawaySDK", "repeat", "query")
    );
    assert!(r.enum_example.is_empty() && r.union_example.is_empty());
    assert!(
        r.base_model_example
            .contains("`references` is a `OnebusawaySDK::References`")
    );
}
