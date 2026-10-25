//! Golden test: each `tests/fixtures/*.json` through Forge and the plugin, compared byte for
//! byte with `tests/golden/<fixture>/` (the `src/**` files). Goldens are approved by a human and never
//! regenerated to make this pass.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrule_python_plugin::PythonPlugin;
use ferrule_python_plugin::emit::agent::emit_agent_files;
use ferrule_python_plugin::emit::files_input::placeholder_meta;
use ferrule_python_plugin::plugin::{code_files, to_model};

fn plugin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixtures() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(plugin_dir().join("tests/fixtures"))
        .expect("fixtures directory")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    out.sort();
    out
}

fn name(fixture: &Path) -> String {
    fixture
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("file name")
        .to_owned()
}

fn forge_of(fixture: &Path) -> forge::Forge {
    serde_json::from_slice(&fs::read(fixture).expect("fixture")).expect("an OpenAPI document Forge reads")
}

/// Every file under `dir`, relative path -> content.
fn read_tree(dir: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(dir).expect("directory") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .expect("under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read_to_string(&p).expect("utf-8 file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    if dir.exists() {
        walk(dir, dir, &mut out);
    }
    out
}

/// Differences between `actual` and `expected`, one message per path (`compareTrees`).
fn compare_trees(actual: &BTreeMap<String, String>, expected: &BTreeMap<String, String>) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, content) in actual {
        match expected.get(path) {
            None => problems.push(format!("unexpected file {path}")),
            Some(e) if e != content => problems.push(format!("content differs: {path}")),
            Some(_) => {}
        }
    }
    for path in expected.keys() {
        if !actual.contains_key(path) {
            problems.push(format!("missing file {path}"));
        }
    }
    problems
}

/// The agent context under `src/<pkg>/_agent/` is not part of [`code_files`]; it is checked
/// through `forge::run` and `agent_files_match_the_golden`.
fn is_agent_file(path: &str) -> bool {
    path.split('/').nth(2) == Some("_agent")
}

fn golden(fixture: &Path) -> BTreeMap<String, String> {
    read_tree(&plugin_dir().join("tests/golden").join(name(fixture)))
}

#[test]
fn every_fixture_has_a_golden_tree() {
    let all = fixtures();
    assert_eq!(
        all.iter().map(|f| name(f)).collect::<Vec<_>>(),
        ["cursor-page", "onebusaway-spike", "onebusaway"]
    );
    for fixture in &all {
        assert!(!golden(fixture).is_empty(), "{}", name(fixture));
    }
}

#[test]
fn code_files_match_the_goldens() {
    let meta = placeholder_meta();
    let mut compared = Vec::new();
    for fixture in fixtures() {
        let actual: BTreeMap<String, String> = code_files(&forge_of(&fixture), Some(&meta))
            .expect("generates")
            .into_iter()
            .collect();
        let expected: BTreeMap<String, String> = golden(&fixture)
            .into_iter()
            .filter(|(p, _)| !is_agent_file(p))
            .collect();
        let problems = compare_trees(&actual, &expected);
        assert!(problems.is_empty(), "{}: {problems:#?}", name(&fixture));
        compared.push((name(&fixture), actual.len()));
    }
    // The goldens hold 5, 7 and 85 files; onebusaway's three `_agent` files are checked below.
    let expected = [
        ("cursor-page".to_owned(), 5),
        ("onebusaway-spike".to_owned(), 7),
        ("onebusaway".to_owned(), 82),
    ];
    assert_eq!(compared, expected);
}

#[test]
fn code_files_are_deterministic() {
    for fixture in fixtures() {
        let forge = forge_of(&fixture);
        assert_eq!(
            code_files(&forge, None).expect("generates"),
            code_files(&forge, None).expect("generates")
        );
    }
}

#[test]
fn forge_run_matches_the_goldens() {
    for fixture in fixtures() {
        let files = forge::run(&forge_of(&fixture), &PythonPlugin::default()).expect("generates");
        let actual: BTreeMap<String, String> = files
            .into_iter()
            .filter(|f| f.path.starts_with("src/"))
            .map(|f| (f.path, f.content))
            .collect();
        let problems = compare_trees(&actual, &golden(&fixture));
        assert!(problems.is_empty(), "{}: {problems:#?}", name(&fixture));
    }
}

#[test]
fn agent_files_match_the_golden() {
    let fixture = plugin_dir().join("tests/fixtures/onebusaway.json");
    let client = to_model(&forge_of(&fixture))
        .expect("converts")
        .expect("has operations");
    let meta = placeholder_meta();
    let actual: BTreeMap<String, String> = emit_agent_files(&client, Some(&meta)).into_iter().collect();
    let text = fs::read_to_string(plugin_dir().join("tests/agent/onebusaway.golden.json")).expect("golden");
    let expected: BTreeMap<String, String> = serde_json::from_str(&text).expect("a JSON object of strings");
    assert_eq!(actual, expected);
}
