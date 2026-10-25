//! The generated Go files against Stainless Go v2 SDKs (`targets.go.options.enable_v2`), byte
//! for byte after the branding substitution of the header line. Skipped when `oracles/cache` is
//! not fetched.
//!
//! - imagekit-go (Apache-2.0, `bc22fec7`, 2.5.0): its `.stats.yml` hashes equal the pinned
//!   imagekit-config's spec and config, so every difference is the plugin's.
//! - knock-mgmt-go (Apache-2.0, `85ae774c`, 0.2.0): generated from a later spec and config than
//!   the pinned knock-mgmt-config (`.stats.yml` hashes differ), so part of its difference is
//!   input drift.
//!
//! Each oracle holds a floor of identical files: raise it when more match, never lower it.

mod common;

use ferrule_go_plugin::{GoMeta, GoOutput, generate};

struct Oracle {
    name: &'static str,
    /// Config oracle directory, spec and config paths in it.
    config: (&'static str, &'static str, &'static str),
    version: &'static str,
    /// `readme.example_requests.default`: resource keys and method.
    example: (&'static [&'static str], &'static str),
    /// Identical generated files, at least.
    floor: usize,
}

const IMAGEKIT: Oracle = Oracle {
    name: "imagekit-go",
    config: (
        "imagekit-config",
        "../specs/imagekit-config.yml",
        "stainless-config/main.yaml",
    ),
    version: "2.5.0",
    example: (&["files"], "upload"),
    floor: 12,
};

const KNOCK_MGMT: Oracle = Oracle {
    name: "knock-mgmt-go",
    config: (
        "knock-mgmt-config",
        "data/specs/mapi/openapi.yml",
        "data/specs/mapi/stainless.yml",
    ),
    version: "0.2.0",
    example: (&["workflows"], "list"),
    floor: 7,
};

fn generated(o: &Oracle) -> Option<(std::path::PathBuf, GoOutput)> {
    let oracle = common::oracle(o.name)?;
    let (dir, spec, config) = o.config;
    let annotated = common::annotated(dir, spec, config)?;
    let model = common::paginated_model(&annotated, dir)?;
    let meta = GoMeta {
        version: Some(o.version.into()),
        example: Some((
            o.example.0.iter().map(|s| (*s).to_owned()).collect(),
            o.example.1.into(),
        )),
        // `settings.disable_mock_tests: true` in both configs.
        disable_mock_tests: true,
        ..GoMeta::default()
    };
    let out = generate(&model, &meta).expect("generates");
    // Debugging aid: FERRULE_GO_DUMP=<dir> writes the generated tree to <dir>/<oracle>.
    if let Some(dump) = std::env::var_os("FERRULE_GO_DUMP") {
        for (path, text) in &out.files {
            let p = std::path::Path::new(&dump).join(o.name).join(path);
            std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            std::fs::write(p, text).expect("write");
        }
    }
    Some((oracle, out))
}

fn matches_the_oracle(o: &Oracle) {
    let Some((oracle, out)) = generated(o) else { return };
    assert!(
        out.runtime.to_json().contains("\"layout\": \"v2\""),
        "{}: enable_v2 selects the v2 runtime",
        o.name
    );
    let identical: Vec<&str> = out
        .files
        .iter()
        .filter(|(path, text)| std::fs::read_to_string(oracle.join(path)).is_ok_and(|raw| common::brand(&raw) == *text))
        .map(|(p, _)| p.as_str())
        .collect();
    eprintln!(
        "{}: {}/{} generated files identical: {identical:?}",
        o.name,
        identical.len(),
        out.files.len()
    );
    assert!(
        identical.len() >= o.floor,
        "{}: {} identical, below the floor of {}",
        o.name,
        identical.len(),
        o.floor
    );
}

#[test]
fn imagekit_matches_the_oracle() {
    matches_the_oracle(&IMAGEKIT);
}

#[test]
fn knock_mgmt_matches_the_oracle() {
    matches_the_oracle(&KNOCK_MGMT);
}

#[test]
fn v2_files_are_gofmt_clean() {
    if common::gofmt_path().is_none() {
        eprintln!("skipped: gofmt is not installed");
        return;
    }
    for o in [&IMAGEKIT, &KNOCK_MGMT] {
        let Some((_, out)) = generated(o) else { continue };
        for (path, text) in out.files.iter().filter(|(p, _)| common::is_go(p)) {
            let formatted = common::gofmt(text).expect("gofmt");
            assert!(formatted == *text, "{}: {path} is not gofmt-clean", o.name);
        }
    }
}
