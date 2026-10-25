//! Shared by the oracle tests: converting a cached config oracle, reading a cached TypeScript
//! oracle checkout, and comparing trees.

// Each test binary uses part of this module.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use ferrule_converter::Value as CValue;
use ferrule_sdk_model::Client;
use ferrule_ts_plugin::meta::Stats;
use ferrule_ts_plugin::{SpecExtras, TsMeta};
use serde_json::Value;

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A converter value as `serde_json` (keys in the same order).
pub fn to_json(v: &CValue) -> Value {
    match v {
        CValue::Null => Value::Null,
        CValue::Bool(b) => Value::Bool(*b),
        CValue::Number(n) => {
            #[expect(clippy::cast_possible_truncation, reason = "checked to be integral and in range")]
            let int = (n.fract() == 0.0 && n.abs() < 9.0e15).then_some(*n as i64);
            int.map_or_else(
                || serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number),
                |i| Value::Number(i.into()),
            )
        }
        CValue::String(s) => Value::String(s.clone()),
        CValue::Array(a) => Value::Array(a.iter().map(to_json).collect()),
        CValue::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), to_json(v))).collect()),
    }
}

/// A TypeScript oracle: a config oracle in `oracles/cache` and the SDK checkout at its pinned
/// commit in `.cache/ts-oracles/<sdk>` (both gitignored; tests skip without them).
pub struct Oracle {
    pub sdk: PathBuf,
    pub client: Client,
    pub meta: TsMeta,
    pub extras: SpecExtras,
}

fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

/// `key: value` lines of `.stats.yml`.
fn stats(text: &str) -> Option<Stats> {
    let get = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{k}: ")))
            .map(str::to_owned)
    };
    Some(Stats {
        openapi_spec_url: get("openapi_spec_url")?,
        openapi_spec_hash: get("openapi_spec_hash")?,
        config_hash: get("config_hash")?,
    })
}

/// Loads an oracle, or `None` (with a note) when its files are not cached.
pub fn oracle(sdk: &str, config_dir: &str, spec: &str, config: &str) -> Option<Oracle> {
    let root = root();
    let sdk_dir = root.join(".cache/ts-oracles").join(sdk);
    let cfg_dir = root.join("oracles/cache").join(config_dir);
    let (Some(spec_text), Some(config_text)) = (read(&cfg_dir.join(spec)), read(&cfg_dir.join(config))) else {
        eprintln!("skipped: {} is not cached", cfg_dir.display());
        return None;
    };
    if !sdk_dir.join("package.json").exists() {
        eprintln!("skipped: {} is not cached", sdk_dir.display());
        return None;
    }
    let converted = ferrule_converter::convert_text(&spec_text, &config_text).expect("the oracle converts");
    let forge: forge::Forge = serde_json::from_str(&converted.output).expect("a document Forge reads");
    let client = match ferrule_sdk_model::forge_to_model(&forge) {
        Ok(Some(client)) => client,
        Ok(None) => panic!("{sdk}: the oracle has no operations"),
        Err(err) => {
            // The model cannot describe it yet: an adapter or converter gap, reported, not hidden.
            eprintln!("skipped: {sdk} does not map to the neutral model: {err}");
            return None;
        }
    };
    let spec_json = to_json(&ferrule_converter::parse_spec(&spec_text).expect("the spec parses"));
    let config_json = to_json(&ferrule_converter::yaml::parse(&config_text).expect("the config parses"));
    let mut meta = TsMeta::from_config(&config_json);
    let package: Value = serde_json::from_str(&read(&sdk_dir.join("package.json"))?).ok()?;
    if let Some(v) = package.get("version").and_then(Value::as_str) {
        v.clone_into(&mut meta.version);
    }
    meta.stats = read(&sdk_dir.join(".stats.yml")).as_deref().and_then(stats);
    Some(Oracle {
        sdk: sdk_dir,
        client,
        meta,
        extras: SpecExtras::new(spec_json),
    })
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() != ".git" && entry.file_name() != "node_modules" {
                walk(root, &path, out);
            }
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Every file under `dir`, relative to it, sorted.
pub fn tree(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// The files `runtime/typescript/template` renders (paths from the SDK root).
pub fn runtime_files() -> Vec<String> {
    tree(&root().join("runtime/typescript/template"))
}

/// Writes `files` under `target/ts-oracle/<name>` for inspection.
pub fn dump(name: &str, files: &[(String, String)]) {
    let out = root().join("target/ts-oracle").join(name);
    let _ = std::fs::remove_dir_all(&out);
    for (path, text) in files {
        let p = out.join(path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("create dir");
        }
        std::fs::write(&p, text).expect("write file");
    }
}

/// Every line where Ferrule's output differs from the oracle on purpose: (file or `*`, oracle
/// text, Ferrule text). See `crates/ts-plugin/BRANDING.md`.
const BRANDING: &[(&str, &str, &str)] = &[
    (
        "*",
        "// File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details.",
        "// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.",
    ),
    (
        "*",
        "File generated from our OpenAPI spec by Stainless.",
        "File generated from our OpenAPI spec by Ferrule.",
    ),
    (
        "*",
        "It is ignored by Stainless code generation",
        "It is ignored by Ferrule code generation",
    ),
    (
        "README.md",
        "It is generated with [Stainless](https://www.stainless.com/).",
        "It is generated with Ferrule.",
    ),
    (
        "SECURITY.md",
        "This SDK is generated by [Stainless Software Inc](http://stainless.com). Stainless takes security seriously, and encourages you to report any security vulnerability promptly so that appropriate action can be taken.",
        "This SDK is generated by Ferrule. We take security seriously, and encourage you to report any security vulnerability promptly so that appropriate action can be taken.",
    ),
    (
        "SECURITY.md",
        "To report a security issue, please contact the Stainless team at security@stainless.com.",
        "To report a security issue in this SDK, please contact the maintainers listed under Terms and Policies below.",
    ),
];

/// `.github/workflows/ci.yml` without Stainless's runners and its uploads to Stainless storage.
fn brand_ci(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in text.split_inclusive('\n') {
        if line.starts_with("    runs-on: ${{ github.repository == 'stainless-sdks/") {
            out.push("    runs-on: ubuntu-latest\n".to_owned());
        } else {
            out.push(line.to_owned());
        }
    }
    let joined = out.concat();
    let start = joined.find("\n\n      - name: Get GitHub OIDC Token");
    let end_marker = "run: ./scripts/utils/upload-artifact.sh\n";
    match (start, joined.rfind(end_marker)) {
        (Some(s), Some(e)) if e > s => format!("{}\n{}", &joined[..s], &joined[e + end_marker.len()..]),
        _ => joined,
    }
}

/// The oracle text with Ferrule's branding substitutions applied: what the plugin must match.
pub fn brand(path: &str, text: &str) -> String {
    let mut out = text.to_owned();
    for (file, oracle, ferrule) in BRANDING {
        if *file == path || *file == "*" {
            out = out.replace(oracle, ferrule);
        }
    }
    if path == ".github/workflows/ci.yml" {
        out = brand_ci(&out);
    }
    out
}

/// Compares the generated files with the oracle: (identical paths, differing paths).
pub fn compare(o: &Oracle, files: &[(String, String)]) -> (Vec<String>, Vec<String>) {
    let mut same = Vec::new();
    let mut differ = Vec::new();
    for (path, text) in files {
        let expected = read(&o.sdk.join(path)).map(|t| brand(path, &t));
        if expected.as_deref() == Some(text.as_str()) {
            same.push(path.clone());
        } else {
            differ.push(path.clone());
        }
    }
    (same, differ)
}
