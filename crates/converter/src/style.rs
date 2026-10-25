//! `x-ferrule-style`: the spellings of the Stainless generator an adopted SDK's runtime comes
//! from, where they differ from today's. An adopted SDK keeps its own runtime, which supports only
//! what its generator emitted, so the runtime decides; the config's `edition` cannot (the oracles
//! from both styles name none). Pure: the CLI reads the files.

use crate::value::{Map, Value};

/// The runtime modules of the SDK package that decide the style; `None` for a missing file.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeFiles<'a> {
    /// `_types.py`.
    pub types: Option<&'a str>,
    /// `_utils/__init__.py`.
    pub utils: Option<&'a str>,
    /// `_client.py`, as the SDK's generator last wrote it.
    pub client: Option<&'a str>,
    /// `_files.py`.
    pub files: Option<&'a str>,
}

/// Whether `text` assigns the module-level name `name` (`omit = Omit()`).
fn assigns(text: &str, name: &str) -> bool {
    text.lines().any(|l| {
        l.strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with(['=', ':']))
    })
}

/// The keys of `x-ferrule-style` whose value differs from today's, sorted: `multipart:
/// deepcopy_minimal` for a `_files.py` without `deepcopy_with_paths`, `paths: fstring` for a
/// `_utils` without `path_template`, `resources: eager` for a `_client.py` without
/// `cached_property`, `sentinel: NOT_GIVEN` for a `_types.py` without `omit`. Empty for today's
/// runtime, or files that are missing.
#[must_use]
pub fn runtime_style(files: &RuntimeFiles<'_>) -> Map {
    let mut style = Map::new();
    if files.files.is_some_and(|t| !t.contains("def deepcopy_with_paths")) {
        style.insert("multipart", Value::from("deepcopy_minimal"));
    }
    if files.utils.is_some_and(|t| !t.contains("path_template")) {
        style.insert("paths", Value::from("fstring"));
    }
    if files.client.is_some_and(|t| !t.contains("@cached_property")) {
        style.insert("resources", Value::from("eager"));
    }
    if files.types.is_some_and(|t| !assigns(t, "omit")) {
        style.insert("sentinel", Value::from("NOT_GIVEN"));
    }
    style
}

/// Writes `style` as `x-ferrule-style` at the root and on every operation (and alias) that
/// carries `x-ferrule-package`, as the other root extensions are mirrored (Forge gap G2).
/// Nothing for an empty `style`.
pub fn apply_style(spec: &mut Value, style: &Map) {
    if style.is_empty() {
        return;
    }
    let value = Value::Object(style.clone());
    let Some(root) = spec.as_object_mut() else { return };
    if root.contains_key("x-ferrule-config") {
        root.insert("x-ferrule-style", value.clone());
    }
    let Some(Value::Object(paths)) = root.get_mut("paths") else {
        return;
    };
    for item in paths.values_mut() {
        let Value::Object(item) = item else { continue };
        for op in item.values_mut() {
            let Value::Object(op) = op else { continue };
            if op.contains_key("x-ferrule-package") {
                op.insert("x-ferrule-style", value.clone());
            }
            if let Some(Value::Array(aliases)) = op.get_mut("x-forge-aliases") {
                for alias in aliases {
                    if let Value::Object(alias) = alias
                        && alias.contains_key("x-ferrule-package")
                    {
                        alias.insert("x-ferrule-style", value.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeFiles, assigns, runtime_style};

    #[test]
    fn reads_each_switch_from_its_own_file() {
        let old = RuntimeFiles {
            types: Some("NOT_GIVEN = NotGiven()\nomitted = 1\n"),
            utils: Some("from ._utils import is_given as is_given\n"),
            client: Some("self.a = a.AResource(self)\n"),
            files: Some("def to_httpx_files(files):\n"),
        };
        let style = runtime_style(&old);
        let keys: Vec<&str> = style.keys().map(String::as_str).collect();
        assert_eq!(keys, ["multipart", "paths", "resources", "sentinel"]);
        let new = RuntimeFiles {
            types: Some("omit = Omit()\n"),
            utils: Some("from ._path import path_template as path_template\n"),
            client: Some("    @cached_property\n"),
            files: Some("def deepcopy_with_paths(item, paths):\n"),
        };
        assert!(runtime_style(&new).is_empty());
        assert!(runtime_style(&RuntimeFiles::default()).is_empty());
    }

    #[test]
    fn finds_a_module_level_assignment_only() {
        assert!(assigns("omit = Omit()", "omit"));
        assert!(assigns("omit: Omit = Omit()", "omit"));
        assert!(!assigns("    omit = Omit()", "omit"));
        assert!(!assigns("omitted = 1", "omit"));
        assert!(!assigns("# omit = Omit()", "omit"));
    }
}
