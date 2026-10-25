//! Config editions (`converter/src/editions/index.ts`). An edition pins the defaults Stainless
//! applies to keys a config leaves out, so a config keeps generating the same SDK when newer
//! defaults ship. Every value is cited from a description in the public schema.

use crate::Diagnostic;
use crate::schema::suggest::closest;
use crate::value::{Map, Value, to_json};

/// OpenAPI normalizer version (`/properties/edition/anyOf/0`, 2026-05-06: "Switches the default
/// normalizer to v2."). There is no config key for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Normalizer {
    V1,
    V2,
}

/// Top-level `edition` values, oldest first, with the normalizer each pins.
pub const EDITIONS: [(&str, Normalizer); 5] = [
    ("2025-10-08", Normalizer::V1),
    ("2025-10-10", Normalizer::V1),
    ("2026-01-30", Normalizer::V1),
    ("2026-02-23", Normalizer::V1),
    ("2026-05-06", Normalizer::V2),
];

/// `targets.python.edition` values, oldest first, with `use_uv` (`python.2025-11-20` switched
/// the default package manager from Rye to uv).
pub const PYTHON_EDITIONS: [(&str, bool); 2] = [("python.2025-10-08", false), ("python.2025-11-20", true)];

/// The edition `x-ferrule-config.edition` reported in Phase 1 for a config without one.
pub const DEFAULT_EDITION: &str = "2025-10-10";

/// A config without `edition` predates editions, so it gets the first edition's defaults.
pub const IMPLICIT_EDITION: &str = "2025-10-08";
pub const IMPLICIT_PYTHON_EDITION: &str = "python.2025-10-08";

/// The edition names, oldest first.
#[must_use]
pub fn edition_names() -> Vec<&'static str> {
    EDITIONS.iter().map(|(n, _)| *n).collect()
}

/// The python edition names, oldest first.
#[must_use]
pub fn python_edition_names() -> Vec<&'static str> {
    PYTHON_EDITIONS.iter().map(|(n, _)| *n).collect()
}

/// Whether `value` names a known top-level edition.
#[must_use]
pub fn is_edition(value: &Value) -> bool {
    value.as_str().is_some_and(|s| EDITIONS.iter().any(|(n, _)| *n == s))
}

/// Whether `value` names a known python edition.
#[must_use]
pub fn is_python_edition(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|s| PYTHON_EDITIONS.iter().any(|(n, _)| *n == s))
}

/// Every edition-dependent default, including ones with no config key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defaults {
    pub normalizer: Normalizer,
    pub use_uv: bool,
}

/// The result of [`apply_edition`].
#[derive(Debug, Clone)]
pub struct EditionResult {
    /// A copy of the input with edition-dependent defaults written where the config left them out.
    pub config: Value,
    /// The edition whose defaults were applied (the implicit one when absent or unknown).
    pub edition: &'static str,
    pub python_edition: &'static str,
    pub defaults: Defaults,
    pub diagnostics: Vec<Diagnostic>,
}

fn unknown(code: &str, pointer: &str, value: &Value, names: &[&str]) -> Diagnostic {
    let near = value.as_str().and_then(|v| closest(v, names));
    let list = names.join(", ");
    Diagnostic::error(
        code,
        pointer,
        format!("unknown edition {}; Ferrule knows {list}", to_json(value)),
        near.map_or_else(
            || format!("Use one of {list}, or upgrade Ferrule if this edition is newer."),
            |n| format!("Did you mean \"{n}\"?"),
        ),
    )
}

fn record(value: Option<&Value>) -> Option<&Map> {
    value.and_then(Value::as_object)
}

/// Fill edition-dependent defaults into a copy of `config`. Pure; keys the config sets
/// explicitly always win. Unknown editions are FER1101 (top level) and FER1102
/// (`targets.python.edition`); their defaults fall back to the implicit edition.
#[must_use]
pub fn apply_edition(config: &Value) -> EditionResult {
    let mut diagnostics = Vec::new();
    let edition = match config.get("edition") {
        None => IMPLICIT_EDITION,
        Some(v) => EDITIONS.iter().find(|(n, _)| Some(*n) == v.as_str()).map_or_else(
            || {
                diagnostics.push(unknown("FER1101", "/edition", v, &edition_names()));
                IMPLICIT_EDITION
            },
            |(n, _)| *n,
        ),
    };
    let python = record(config.get("targets").and_then(|t| t.get("python")));
    let python_edition = match python.and_then(|p| p.get("edition")) {
        None => IMPLICIT_PYTHON_EDITION,
        Some(v) => PYTHON_EDITIONS
            .iter()
            .find(|(n, _)| Some(*n) == v.as_str())
            .map_or_else(
                || {
                    diagnostics.push(unknown(
                        "FER1102",
                        "/targets/python/edition",
                        v,
                        &python_edition_names(),
                    ));
                    IMPLICIT_PYTHON_EDITION
                },
                |(n, _)| *n,
            ),
    };
    let normalizer = EDITIONS
        .iter()
        .find(|(n, _)| *n == edition)
        .map_or(Normalizer::V1, |(_, v)| *v);
    let use_uv = PYTHON_EDITIONS
        .iter()
        .find(|(n, _)| *n == python_edition)
        .is_some_and(|(_, v)| *v);
    let defaults = Defaults { normalizer, use_uv };

    let mut out = config.as_object().cloned().unwrap_or_default();
    let mut targets = match out.get("targets") {
        Some(Value::Object(t)) => t.clone(),
        _ => Map::new(),
    };
    let mut py = record(targets.get("python")).cloned().unwrap_or_default();
    let mut options = record(py.get("options")).cloned().unwrap_or_default();
    let uv = match options.get("use_uv") {
        None | Some(Value::Null) => Value::Bool(defaults.use_uv),
        Some(v) => v.clone(),
    };
    options.insert("use_uv", uv);
    py.insert("options", Value::Object(options));
    targets.insert("python", Value::Object(py));
    out.insert("targets", Value::Object(targets));
    EditionResult {
        config: Value::Object(out),
        edition,
        python_edition,
        defaults,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::stainless_config_schema;
    use proptest::prelude::*;
    use serde_json::json;

    fn schema_consts(node: &Value) -> Vec<String> {
        let mut out: Vec<String> = node
            .get("anyOf")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|b| b.get("const").and_then(Value::as_str).map(str::to_owned))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn tables_list_exactly_the_editions_the_public_schema_names() {
        let schema = stainless_config_schema();
        let top = schema.get("properties").and_then(|p| p.get("edition")).unwrap();
        let mut names: Vec<String> = edition_names().into_iter().map(str::to_owned).collect();
        names.sort();
        assert_eq!(names, schema_consts(top));
        let py = schema
            .get("properties")
            .and_then(|p| p.get("targets"))
            .and_then(|t| t.get("properties"))
            .and_then(|p| p.get("python"))
            .and_then(|p| p.get("properties"))
            .and_then(|p| p.get("edition"))
            .unwrap();
        let mut names: Vec<String> = python_edition_names().into_iter().map(str::to_owned).collect();
        names.sort();
        assert_eq!(names, schema_consts(py));
    }

    #[test]
    fn tables_are_ordered_oldest_first() {
        let mut sorted = edition_names();
        sorted.sort_unstable();
        assert_eq!(edition_names(), sorted);
        let mut sorted = python_edition_names();
        sorted.sort_unstable();
        assert_eq!(python_edition_names(), sorted);
    }

    #[test]
    fn switch_the_normalizer_at_2026_05_06_and_uv_at_python_2025_11_20() {
        let n: Vec<Normalizer> = EDITIONS.iter().map(|(_, n)| *n).collect();
        assert_eq!(
            n,
            [
                Normalizer::V1,
                Normalizer::V1,
                Normalizer::V1,
                Normalizer::V1,
                Normalizer::V2
            ]
        );
        assert_eq!(
            PYTHON_EDITIONS,
            [("python.2025-10-08", false), ("python.2025-11-20", true)]
        );
    }

    #[test]
    fn recognise_edition_names() {
        assert!(is_edition(&Value::from(DEFAULT_EDITION)));
        assert!(!is_edition(&Value::from("toString")));
        assert!(is_python_edition(&Value::from("python.2025-11-20")));
        assert!(!is_python_edition(&Value::Number(2025.0)));
    }

    #[test]
    fn gives_a_config_without_editions_the_first_editions_defaults() {
        let r = apply_edition(&Value::object());
        assert_eq!(r.edition, IMPLICIT_EDITION);
        assert_eq!(r.python_edition, "python.2025-10-08");
        assert_eq!(
            r.defaults,
            Defaults {
                normalizer: Normalizer::V1,
                use_uv: false
            }
        );
        assert_eq!(
            r.config,
            Value::from(json!({"targets": {"python": {"options": {"use_uv": false}}}}))
        );
        assert_eq!(r.diagnostics, Vec::<Diagnostic>::new());
    }

    #[test]
    fn applies_the_newest_editions() {
        let config = Value::from(json!({
            "edition": "2026-05-06",
            "targets": {"python": {"package_name": "acme", "edition": "python.2025-11-20"}}
        }));
        let r = apply_edition(&config);
        assert_eq!(
            r.defaults,
            Defaults {
                normalizer: Normalizer::V2,
                use_uv: true
            }
        );
        assert_eq!(
            r.config.get("targets").and_then(|t| t.get("python")),
            Some(&Value::from(
                json!({"package_name": "acme", "edition": "python.2025-11-20", "options": {"use_uv": true}})
            ))
        );
        assert!(
            config
                .get("targets")
                .and_then(|t| t.get("python"))
                .and_then(|p| p.get("options"))
                .is_none()
        );
    }

    #[test]
    fn never_overrides_an_explicit_setting() {
        let r = apply_edition(&Value::from(json!({
            "targets": {"python": {"edition": "python.2025-11-20", "options": {"use_uv": false, "x": 1}}}
        })));
        assert_eq!(
            r.config
                .get("targets")
                .and_then(|t| t.get("python"))
                .and_then(|p| p.get("options")),
            Some(&Value::from(json!({"use_uv": false, "x": 1})))
        );
    }

    #[test]
    fn reports_an_unknown_edition_as_fer1101_with_a_did_you_mean() {
        let r = apply_edition(&Value::from(json!({"edition": "2026-05-07"})));
        assert_eq!(r.edition, IMPLICIT_EDITION);
        assert_eq!(
            r.diagnostics,
            [Diagnostic::error(
                "FER1101",
                "/edition",
                "unknown edition \"2026-05-07\"; Ferrule knows 2025-10-08, 2025-10-10, 2026-01-30, 2026-02-23, 2026-05-06",
                "Did you mean \"2026-05-06\"?"
            )]
        );
    }

    #[test]
    fn suggests_upgrading_for_a_far_off_edition_and_reports_python_editions_as_fer1102() {
        let r = apply_edition(&Value::from(json!({
            "edition": "2030-01-01-preview",
            "targets": {"python": {"edition": 7}}
        })));
        let got: Vec<(&str, &str, &str)> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str(), d.pointer.as_str(), d.suggestion.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (
                    "FER1101",
                    "/edition",
                    "Use one of 2025-10-08, 2025-10-10, 2026-01-30, 2026-02-23, 2026-05-06, or upgrade Ferrule if this edition is newer."
                ),
                (
                    "FER1102",
                    "/targets/python/edition",
                    "Use one of python.2025-10-08, python.2025-11-20, or upgrade Ferrule if this edition is newer."
                ),
            ]
        );
    }

    proptest! {
        #[test]
        fn is_pure_and_idempotent(
            e in proptest::option::of(prop_oneof![Just("2025-10-08"), Just("2026-05-06"), Just("2026-01-30"), Just("nope")]),
            pe in proptest::option::of(prop_oneof![Just("python.2025-10-08"), Just("python.2025-11-20"), Just("nope")]),
            uv in proptest::option::of(any::<bool>()),
        ) {
            let mut python = serde_json::Map::new();
            if let Some(pe) = pe { python.insert("edition".into(), json!(pe)); }
            if let Some(uv) = uv { python.insert("options".into(), json!({"use_uv": uv})); }
            let mut config = serde_json::Map::new();
            if let Some(e) = e { config.insert("edition".into(), json!(e)); }
            config.insert("targets".into(), json!({"python": python}));
            let config = Value::from(serde_json::Value::Object(config));
            let before = config.clone();
            let once = apply_edition(&config);
            prop_assert_eq!(&config, &before);
            prop_assert_eq!(&apply_edition(&once.config).config, &once.config);
            let got = once.config.get("targets").and_then(|t| t.get("python")).and_then(|p| p.get("options")).and_then(|o| o.get("use_uv")).cloned();
            prop_assert_eq!(got, Some(Value::Bool(uv.unwrap_or(once.defaults.use_uv))));
        }
    }
}
