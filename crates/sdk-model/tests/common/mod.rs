//! Inputs shared by the snapshot and cross-check tests: the Python plugin's annotated fixtures,
//! and the config oracles in `oracles/cache` (gitignored, so skipped when not fetched), converted
//! with the converter at test time.

// Each test binary that includes this module uses only part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn forge(json: &str) -> forge::Forge {
    serde_json::from_str(json).expect("a document Forge reads")
}

/// The annotated fixtures committed with the Python plugin, by name.
pub fn python_fixtures() -> Vec<(String, String)> {
    let dir = root().join("crates/python-plugin/tests");
    [
        ("onebusaway", "fixtures/onebusaway.json"),
        ("onebusaway-spike", "fixtures/onebusaway-spike.json"),
        ("cursor-page", "fixtures/cursor-page.json"),
        ("model-kinds", "model_kinds/annotated.json"),
        ("standalone", "standalone/annotated.json"),
    ]
    .into_iter()
    .map(|(name, path)| {
        let text = std::fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        (name.to_owned(), text)
    })
    .collect()
}

/// A config oracle: its name, spec and stainless.yml in the cache.
pub struct Pair {
    pub name: String,
    pub spec: PathBuf,
    pub config: PathBuf,
}

/// Every `kind: config` oracle of `oracles/manifest.yaml` (JSON with `#` comment lines).
pub fn config_pairs() -> Vec<Pair> {
    let cache = root().join("oracles/cache");
    let text = std::fs::read_to_string(root().join("oracles/manifest.yaml")).expect("manifest");
    let json: String = text
        .lines()
        .map(|l| if l.starts_with('#') { "" } else { l })
        .collect::<Vec<_>>()
        .join("\n");
    let data: serde_json::Value = serde_json::from_str(&json).expect("manifest JSON");
    let entries = data["oracles"].as_array().cloned().unwrap_or_default();
    entries
        .iter()
        .filter(|e| e["kind"] == "config")
        .filter_map(|e| {
            let name = e["name"].as_str()?.to_owned();
            let config = cache.join(&name).join(e["config_path"].as_str()?);
            let spec = if let Some(p) = e["spec"]["path"].as_str() {
                cache.join(&name).join(p)
            } else {
                let mut found: Vec<PathBuf> = std::fs::read_dir(cache.join("specs"))
                    .ok()?
                    .filter_map(|f| f.ok().map(|f| f.path()))
                    .filter(|p| {
                        p.file_name()
                            .and_then(|f| f.to_str())
                            .is_some_and(|f| f.starts_with(&format!("{name}.")))
                    })
                    .collect();
                found.sort();
                found.into_iter().next()?
            };
            Some(Pair { name, spec, config })
        })
        .collect()
}

/// The annotated spec of a cached oracle, or `None` when its files are not cached.
pub fn annotated(pair: &Pair) -> Option<String> {
    if !pair.spec.exists() || !pair.config.exists() {
        return None;
    }
    let spec = std::fs::read_to_string(&pair.spec).expect("spec");
    let config = std::fs::read_to_string(&pair.config).expect("config");
    let out = ferrule_converter::convert_text(&spec, &config).expect("converts");
    (!out.output.is_empty()).then_some(out.output)
}
