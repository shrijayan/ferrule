//! `x-ferrule-targets` and `x-ferrule-targets-only` (ADR 0003, per-target amendment): every
//! stainless.yml target with its edition, and which targets a method or model is generated for.

use crate::editions::{TARGET_EDITIONS, apply_edition};
use crate::extensions::json::obj_of;
use crate::value::{Map, Value};

/// Target keys that `x-ferrule-targets` gives a member of their own; `publish` is left out, as no
/// generator reads it, and every other key goes to `options` as written.
const OWN_KEYS: [&str; 4] = ["package_name", "edition", "production_repo", "publish"];

/// `x-ferrule-targets`: every key under `targets`, with the edition whose defaults apply and the
/// target's other keys (after edition defaults, so Python's `options.use_uv` is filled in).
#[must_use]
pub fn build_targets(config: &Value) -> Value {
    let ed = apply_edition(config);
    let targets = obj_of(ed.config.get("targets"));
    let mut out = Map::new();
    for (key, edition) in &ed.target_editions {
        let target = obj_of(targets.get(key));
        // A name or repository that is not a string (stagehand's `production_repo: null`) is
        // left out, as if absent.
        let string = |k: &str| target.get(k).filter(|v| matches!(v, Value::String(_))).cloned();
        let mut entry = Map::new();
        if let Some(name) = string("package_name") {
            entry.insert("package_name", name);
        }
        if let Some(edition) = edition {
            entry.insert("edition", Value::from(edition.as_str()));
        }
        if let Some(repo) = string("production_repo") {
            entry.insert("production_repo", repo);
        }
        let mut options = Map::new();
        for (k, v) in target {
            if !OWN_KEYS.contains(&k.as_str()) {
                options.insert(k.clone(), v.clone());
            }
        }
        entry.insert("options", Value::Object(options));
        out.insert(key.clone(), Value::Object(entry));
    }
    Value::Object(out)
}

/// The targets an unrestricted item is generated for: the config's target keys in config order,
/// or, for a config without targets (a fragment, as in tests), every target the public schema
/// knows, so that a list still says which languages it keeps.
#[derive(Debug, Clone)]
pub struct Targets {
    configured: bool,
    all: Vec<String>,
}

impl Targets {
    #[must_use]
    pub fn of(config: &Value) -> Self {
        let keys: Vec<String> = obj_of(config.get("targets")).keys().cloned().collect();
        if keys.is_empty() {
            Self {
                configured: false,
                all: TARGET_EDITIONS.iter().map(|(t, _)| (*t).to_owned()).collect(),
            }
        } else {
            Self {
                configured: true,
                all: keys,
            }
        }
    }

    /// Whether the config has a target other than Python.
    #[must_use]
    pub fn beyond_python(&self) -> bool {
        self.configured && self.all.iter().any(|t| t != "python")
    }

    /// The targets `item` (a resource, method or model object) is generated for, given the ones
    /// its parent is generated for (`None`: every target, unrestricted). `only:` keeps the listed
    /// targets, `skip:` drops them, and `skip: true` drops all; `None` while nothing restricts.
    #[must_use]
    pub fn restrict(&self, parent: Option<&[String]>, item: &Map) -> Option<Vec<String>> {
        let only = match item.get("only") {
            Some(Value::Array(list)) => Some(list.as_slice()),
            _ => None,
        };
        let skip = item.get("skip");
        let skip_all = skip == Some(&Value::Bool(true));
        let skip = match skip {
            Some(Value::Array(list)) => Some(list.as_slice()),
            _ => None,
        };
        if parent.is_none() && only.is_none() && skip.is_none() && !skip_all {
            return None;
        }
        let listed = |list: &[Value], t: &str| list.iter().any(|v| v.as_str() == Some(t));
        Some(
            parent
                .map_or(self.all.as_slice(), |p| p)
                .iter()
                .filter(|_| !skip_all)
                .filter(|t| only.is_none_or(|o| listed(o, t)))
                .filter(|t| skip.is_none_or(|s| !listed(s, t)))
                .cloned()
                .collect(),
        )
    }
}

/// `x-ferrule-targets-only` for a target list.
#[must_use]
pub fn targets_only(targets: &[String]) -> Value {
    Value::Array(targets.iter().map(|t| Value::from(t.as_str())).collect())
}
