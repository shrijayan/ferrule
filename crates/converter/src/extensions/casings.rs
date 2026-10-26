//! `custom_casings` (stainless.yml): how single words are spelled in class names. The converter
//! applies them to model names (`x-ferrule-models`) and carries them to the plugin as
//! `x-ferrule-casings`, which spells resource, method-type and client class names with them.
//!
//! Shape written: `Record<word, { initialism: boolean } | { pascal: string }>`, in config order
//! (the output is key-sorted anyway). Only what Python spells differently is carried: `camel` and
//! `capital` are for other targets, and `snake` changes nothing for one lowercase word.

use crate::Diagnostic;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::value::{Map, Value};

/// Words Stainless spells upper-case in class names unless `custom_casings` says otherwise, as
/// listed in the `initialism` description of the public stainless.yml schema
/// (`converter/schema/stainless-config.schema.json`).
pub const DEFAULT_INITIALISMS: [&str; 39] = [
    "3ds", "ach", "acl", "ai", "api", "ats", "cpu", "crm", "db", "dns", "e2e", "eeoc", "gb", "gpu", "hd", "hris",
    "html", "http", "https", "id", "ip", "iso", "jsonl", "kb", "kyb", "kyc", "lfs", "mb", "mp3", "mp4", "nft", "saml",
    "sdk", "sku", "sms", "ss", "ssh", "sso", "url",
];

/// How one word is spelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Casing {
    /// `initialism: true` spells it upper-case; `false` takes it off the default list.
    Initialism(bool),
    /// The `pascal` spelling, used as written: `LumaAI`, `v1`.
    Pascal(String),
}

/// The usable `custom_casings` entries, by lowercase word, in config order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Casings {
    words: Vec<(String, Casing)>,
}

fn pointer(key: &str, rest: &[&str]) -> String {
    let mut p = vec![Seg::from("custom_casings"), Seg::from(key)];
    p.extend(rest.iter().map(|&s| Seg::from(s)));
    to_json_pointer(&p)
}

fn is_word(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

fn is_spelling(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().collect::<String>() + chars.as_str())
}

/// Reads `custom_casings`; entries Ferrule cannot apply are `FER17xx` warnings and left out.
#[must_use]
pub fn read_casings(config: &Value) -> (Casings, Vec<Diagnostic>) {
    let mut casings = Casings::default();
    let mut diagnostics = Vec::new();
    let Some(Value::Object(entries)) = config.get("custom_casings") else {
        return (casings, diagnostics);
    };
    for (key, entry) in entries {
        let shape = || {
            Diagnostic::warning(
                "FER1703",
                pointer(key, &[]),
                format!(
                    "Custom casing \"{key}\" is neither `{{ initialism: boolean }}` nor `{{ pascal, camel, snake, capital }}`; it is ignored."
                ),
                format!("Write `{key}: {{ initialism: true }}`, or give the four spellings of the word."),
            )
        };
        if !is_word(key) {
            diagnostics.push(Diagnostic::warning(
                "FER1701",
                pointer(key, &[]),
                format!(
                    "Custom casing \"{key}\" is not one lowercase word; Ferrule matches single words, so it is ignored."
                ),
                "Use one lowercase alphanumeric word as the key, such as `qa`, or remove the entry.",
            ));
            continue;
        }
        let Value::Object(m) = entry else {
            diagnostics.push(shape());
            continue;
        };
        if let Some(Value::Bool(b)) = m.get("initialism") {
            casings.words.push((key.clone(), Casing::Initialism(*b)));
        } else if m.contains_key("singular") || m.contains_key("plural") {
            diagnostics.push(Diagnostic::warning(
                "FER1702",
                pointer(key, &[]),
                format!(
                    "Custom casing \"{key}\" uses the singular/plural form, which is not supported yet; the word keeps its default casing."
                ),
                format!("Give `{key}` its `pascal`, `camel`, `snake` and `capital` spellings, or `initialism: true`."),
            ));
        } else if let Some(Value::String(p)) = m.get("pascal") {
            if is_spelling(p) {
                casings.words.push((key.clone(), Casing::Pascal(p.clone())));
            } else {
                diagnostics.push(Diagnostic::warning(
                    "FER1703",
                    pointer(key, &["pascal"]),
                    format!(
                        "The pascal spelling \"{p}\" of custom casing \"{key}\" is not letters, digits and underscores; it is ignored."
                    ),
                    "Spell the word with ASCII letters and digits only, such as `LumaAI`.",
                ));
            }
        } else {
            diagnostics.push(shape());
        }
    }
    (casings, diagnostics)
}

impl Casings {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    fn get(&self, word: &str) -> Option<&Casing> {
        self.words.iter().find(|(w, _)| w == word).map(|(_, c)| c)
    }

    fn is_initialism(&self, word: &str) -> bool {
        match self.get(word) {
            Some(Casing::Initialism(b)) => *b,
            Some(Casing::Pascal(_)) => false,
            None => DEFAULT_INITIALISMS.contains(&word),
        }
    }

    /// `x-ferrule-casings`.
    #[must_use]
    pub fn extension(&self) -> Value {
        let mut out = Map::new();
        for (word, casing) in &self.words {
            let mut entry = Map::new();
            match casing {
                Casing::Initialism(b) => entry.insert("initialism", Value::Bool(*b)),
                Casing::Pascal(p) => entry.insert("pascal", Value::from(p.as_str())),
            };
            out.insert(word.clone(), Value::Object(entry));
        }
        Value::Object(out)
    }

    /// The class name of a `models:` key: `id_number` -> `IDNumber`, `lumaai_thing` ->
    /// `LumaAIThing`. Each word takes its custom spelling, else upper case if it is an initialism
    /// (a plural one keeps a lowercase `s`: `ids` -> `IDs`), else a capital first letter.
    #[must_use]
    pub fn class_name(&self, key: &str) -> String {
        key.split('_')
            .filter(|w| !w.is_empty())
            .map(|w| {
                let lower = w.to_ascii_lowercase();
                match self.get(&lower) {
                    Some(Casing::Pascal(p)) => return p.clone(),
                    // Not read as a plural either: `https` off stays `Https`, not `HTTPs`.
                    Some(Casing::Initialism(false)) => return capitalize(w),
                    _ => {}
                }
                if self.is_initialism(&lower) {
                    return lower.to_ascii_uppercase();
                }
                match lower.strip_suffix('s') {
                    Some(stem) if !stem.is_empty() && self.is_initialism(stem) => {
                        format!("{}s", stem.to_ascii_uppercase())
                    }
                    _ => capitalize(w),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn casings(j: &serde_json::Value) -> Casings {
        read_casings(&Value::from(&json!({ "custom_casings": j }))).0
    }

    #[test]
    fn spells_model_names_like_stainless() {
        let none = Casings::default();
        assert_eq!(none.class_name("id_number"), "IDNumber");
        assert_eq!(none.class_name("sms_template"), "SMSTemplate");
        assert_eq!(none.class_name("workflow_ai_agent_step"), "WorkflowAIAgentStep");
        assert_eq!(none.class_name("route_ids"), "RouteIDs");
        assert_eq!(none.class_name("status"), "Status");
        assert_eq!(none.class_name("response_wrapper"), "ResponseWrapper");
        assert_eq!(none.class_name("thing__x"), "ThingX");
        let custom =
            casings(&json!({ "qa": { "initialism": true }, "id": { "initialism": false }, "v1": { "pascal": "v1" } }));
        assert_eq!(custom.class_name("qa_answer"), "QAAnswer");
        assert_eq!(custom.class_name("id_number"), "IdNumber");
        assert_eq!(custom.class_name("v1_thing"), "v1Thing");
    }
}
