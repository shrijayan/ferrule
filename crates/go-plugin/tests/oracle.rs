//! The generated Go files against real Stainless Go SDKs, byte for byte after the branding
//! substitution of the header line. Skipped when `oracles/cache` is not fetched.
//!
//! onebusaway-go (Apache-2.0) is the same generation as the proven Python pin: md5 of its
//! `openapi.yml` and `stainless-config.yml` equal the `.stats.yml` hashes of `go-sdk` at
//! `d115209e`.

mod common;

use ferrule_go_plugin::{GoMeta, GoOutput, generate};

/// Oracle files that differ on purpose or for a recorded gap, with the reason. The test fails
/// when one of them starts matching, so the list stays true.
const ONEBUSAWAY_KNOWN: &[(&str, &str)] = &[
    (
        "arrivalanddeparture_test.go",
        "Forge gap G18: the path param's `example` (1_75403) is not in the model",
    ),
    (
        "scheduleforroute_test.go",
        "Forge gap G18: the path param's `example` (1_100223) is not in the model",
    ),
    (
        "routesforagency_test.go",
        "Forge gap G18: the path param's `example` (40) is not in the model",
    ),
    (
        "arrivalsanddeparturesforlocation_test.go",
        "Forge gap G18: `maxCount`'s `maximum` (1000) is not in the model",
    ),
];

/// Oracle files that are not generated output: hand-written (`examples/*/main.go`), release-please's
/// (`CHANGELOG.md`), or the runtime template's (`runtime/go`, proven by
/// `runtime/tests/test_render_oracles_go.py`).
fn not_generated(path: &str) -> bool {
    const RUNTIME_ROOT: [&str; 2] = ["field.go", "default_http_client.go"];
    (path.starts_with("examples/") && path != "examples/.keep")
        || path == "CHANGELOG.md"
        || RUNTIME_ROOT.contains(&path)
        || ((path.starts_with("internal/") || path.starts_with("option/")) && path != "internal/version.go")
}

/// Project files (README, CI, scripts, go.mod) the plugin does not emit yet.
fn project_file(path: &str) -> bool {
    !common::is_go(path) && path != "api.md"
}

fn onebusaway() -> Option<(std::path::PathBuf, GoOutput)> {
    let oracle = common::oracle("onebusaway-go")?;
    let annotated = common::annotated("onebusaway-config", "openapi.yml", "stainless-config.yml")?;
    let meta = GoMeta {
        version: Some("1.15.0".into()),
        example: Some((vec!["current_time".into()], "retrieve".into())),
        // The module path and package come from `targets.go`.
        ..GoMeta::default()
    };
    let out = generate(&common::model(&annotated), &meta).expect("generates");
    // Debugging aid: FERRULE_GO_DUMP=<dir> writes the generated tree there.
    if let Some(dir) = std::env::var_os("FERRULE_GO_DUMP") {
        for (path, text) in &out.files {
            let p = std::path::Path::new(&dir).join(path);
            std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            std::fs::write(p, text).expect("write");
        }
    }
    Some((oracle, out))
}

#[test]
fn onebusaway_matches_the_oracle() {
    let Some((oracle, out)) = onebusaway() else { return };
    let mut identical = 0;
    let mut differing = Vec::new();
    for (path, text) in &out.files {
        let raw = std::fs::read_to_string(oracle.join(path)).unwrap_or_else(|e| panic!("oracle has no {path}: {e}"));
        let expected = common::brand(&raw);
        let known = ONEBUSAWAY_KNOWN.iter().find(|(p, _)| p == path);
        match (text == &expected, known) {
            (true, None) => identical += 1,
            (true, Some((p, _))) => panic!("{p} is a known difference but now matches; drop it"),
            (false, Some(_)) => {}
            (false, None) => differing.push(path.clone()),
        }
    }
    eprintln!(
        "onebusaway-go: {identical}/{} generated files identical",
        out.files.len()
    );
    assert!(differing.is_empty(), "differ from the oracle: {differing:?}");
    assert_eq!(identical + ONEBUSAWAY_KNOWN.len(), out.files.len());
}

#[test]
fn onebusaway_emits_every_generated_go_file() {
    let Some((oracle, out)) = onebusaway() else { return };
    let missing: Vec<String> = common::walk(&oracle)
        .into_iter()
        .filter(|p| !not_generated(p) && !project_file(p))
        .filter(|p| !out.files.iter().any(|(f, _)| f == p))
        .collect();
    assert!(missing.is_empty(), "not emitted: {missing:?}");
}

#[test]
fn onebusaway_fills_the_runtime_slots_as_the_oracle_has_them() {
    let Some((oracle, out)) = onebusaway() else { return };
    let read = |p: &str| std::fs::read_to_string(oracle.join(p)).expect("oracle file");
    let option = read("option/requestoption.go");
    assert!(
        option.ends_with(&out.runtime.client_options),
        "{}",
        out.runtime.client_options
    );
    let config = read("internal/requestconfig/requestconfig.go");
    assert!(
        config.contains(&out.runtime.config_fields),
        "{}",
        out.runtime.config_fields
    );
    assert!(
        config.contains(&out.runtime.config_clone),
        "{}",
        out.runtime.config_clone
    );
    assert_eq!(
        (
            out.runtime.client_name.as_str(),
            out.runtime.org_name.as_str(),
            out.runtime.max_retries
        ),
        ("OnebusawaySDK", "onebusaway-sdk", 2)
    );
}

/// Page types of knock-go's `pagination.go` that differ, with the reason.
/// None since the converter writes `x-ferrule-pagination`'s keys in stainless.yml's order.
const KNOCK_PAGINATION_KNOWN: &[(&str, &str)] = &[];

/// The top-level declarations of a Go file by exported type, each up to the next exported
/// `type` line (its JSON metadata and methods with it); the text before the first is keyed `""`.
fn type_blocks(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![(String::new(), String::new())];
    for line in text.split_inclusive('\n') {
        if let Some(rest) = line
            .strip_prefix("type ")
            .filter(|r| r.starts_with(|c: char| c.is_ascii_uppercase()))
        {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            out.push((name, String::new()));
        }
        out.last_mut().expect("one block").1.push_str(line);
    }
    out
}

/// knock-go (Apache-2.0, `4b86a0d6`) is a 2025 generation (`json:"x,required"` tags), so only
/// its `packages/pagination/pagination.go` is compared: page types are tagged without options
/// in every generation.
#[test]
fn knock_pagination_matches_the_oracle() {
    let Some(oracle) = common::oracle("knock-go") else {
        return;
    };
    let Some(annotated) = common::annotated(
        "knock-config",
        "../specs/knock-config.yml",
        "data/specs/api/stainless.yml",
    ) else {
        return;
    };
    let Some(model) = common::paginated_model(&annotated, "knock-config") else {
        return;
    };
    let out = generate(&model, &GoMeta::default()).expect("generates");
    let path = "packages/pagination/pagination.go";
    let text = &out
        .files
        .iter()
        .find(|(p, _)| p == path)
        .expect("emits pagination.go")
        .1;
    let expected = common::brand(&std::fs::read_to_string(oracle.join(path)).expect("oracle file"));
    let (got, want) = (type_blocks(text), type_blocks(&expected));
    let names = |b: &[(String, String)]| b.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    assert_eq!(names(&got), names(&want), "page types");
    for ((name, g), (_, w)) in got.iter().zip(&want) {
        let known = KNOCK_PAGINATION_KNOWN.iter().any(|(k, _)| k == name);
        assert!(
            (g == w) != known,
            "{name}: {}",
            if known { "now matches" } else { "differs" }
        );
    }
}

#[test]
fn generated_files_are_gofmt_clean() {
    let Some((_, out)) = onebusaway() else { return };
    if common::gofmt_path().is_none() {
        eprintln!("skipped: gofmt is not installed");
        return;
    }
    for (path, text) in out.files.iter().filter(|(p, _)| common::is_go(p)) {
        let formatted = common::gofmt(text).expect("gofmt");
        assert!(formatted == *text, "{path} is not gofmt-clean");
    }
}
