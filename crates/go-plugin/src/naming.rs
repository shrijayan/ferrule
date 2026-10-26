//! The Go namer: word lists from the neutral model spelled as Go identifiers, the way Stainless's
//! Go generator spells them.
//!
//! Exported names are `PascalCase` with Go initialisms upper-cased (`fareUrl` -> `FareURL`,
//! `stopIds` -> `StopIDs`); `x-ferrule-casings` (`custom_casings`) overrides single words. Types
//! the generator synthesizes are named by position: a service's types start with its resource
//! path (each segment singular, `trip-details` -> `TripDetail`), then the method (`Get`), then
//! `Response` or `Params`, then the field path down to an inline object (array items singular).

use ferrule_sdk_model::model::{Casing, CustomCasing, Name};

/// Words Stainless spells upper-case in Go identifiers; a plural `s` stays lower-case (`IDs`).
/// `ai`, `html`, `mp4`, `sms` and `sso` are from imagekit-go and knock-mgmt-go. `json` is not
/// one: ogx-go and knock-mgmt-go spell it `Json` (`JsonSchema`, `JsonBody`,
/// `MessageTypeJsonField`, `TranslationFormatJson`), which also keeps a property named `json`
/// clear of a response's `JSON` metadata field.
const INITIALISMS: [&str; 15] = [
    "id", "ids", "url", "uri", "api", "http", "https", "ip", "uuid", "sdk", "ai", "html", "mp4", "sms", "sso",
];

/// Words Stainless spells in a fixed mixed case (knock-mgmt-go's `OAuthContext`).
const MIXED: [(&str, &str); 1] = [("oauth", "OAuth")];

/// Go's keywords, which no identifier may be.
const KEYWORDS: [&str; 25] = [
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// The words of `x-ferrule-casings`, looked up by lower-case word.
#[derive(Debug, Clone, Copy)]
pub struct Casings<'a>(pub &'a [CustomCasing]);

impl Casings<'_> {
    fn get(&self, word: &str) -> Option<&Casing> {
        self.0.iter().find(|c| c.word == word).map(|c| &c.casing)
    }
}

fn clean<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    words
        .iter()
        .map(|w| {
            w.as_ref()
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// `w[0].toUpperCase() + w.slice(1)` on an ASCII word.
fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_ascii_uppercase().to_string() + chars.as_str())
}

/// One word of an exported identifier.
fn exported_word(w: &str, casings: Casings<'_>) -> String {
    let lower = w.to_ascii_lowercase();
    match casings.get(&lower) {
        Some(Casing::Pascal(p)) => return p.clone(),
        Some(Casing::Initialism(true)) => return lower.to_ascii_uppercase(),
        Some(Casing::Initialism(false)) => return capitalize(&lower),
        None => {}
    }
    if let Some(stem) = lower.strip_suffix('s')
        && matches!(casings.get(stem), Some(Casing::Initialism(true)))
    {
        return format!("{}s", stem.to_ascii_uppercase());
    }
    if let Some((_, spelled)) = MIXED.iter().find(|(word, _)| *word == lower) {
        (*spelled).to_owned()
    } else if !INITIALISMS.contains(&lower.as_str()) {
        capitalize(w)
    } else if let Some(stem) = lower.strip_suffix('s')
        && lower != "https"
        && INITIALISMS.contains(&stem)
    {
        // A plural of an initialism (`IDs`); `sms` is one word (`SMS`).
        format!("{}s", stem.to_ascii_uppercase())
    } else {
        lower.to_ascii_uppercase()
    }
}

/// An exported identifier: `["fare", "url"]` -> `FareURL`, `["stop", "ids"]` -> `StopIDs`.
#[must_use]
pub fn exported<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let name: String = clean(words).iter().map(|w| exported_word(w, casings)).collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// A local identifier: `["agency", "id"]` -> `agencyID`, `["trip"]` -> `trip`.
#[must_use]
pub fn local<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let words = clean(words);
    let Some((first, rest)) = words.split_first() else {
        return "_".to_owned();
    };
    let mut name = first.to_ascii_lowercase();
    name.extend(rest.iter().map(|w| exported_word(w, casings)));
    if name.starts_with(|c: char| c.is_ascii_digit()) || KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

/// The unexported twin of an exported type name, for its JSON metadata struct:
/// `AgencyGetResponse` -> `agencyGetResponse`.
#[must_use]
pub fn unexported(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_ascii_lowercase().to_string() + chars.as_str())
}

fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// The words with the last one singular, as Stainless names array items and services:
/// `agencies` -> `agency`, `trip details` -> `trip detail`.
#[must_use]
pub fn singular<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    let mut out: Vec<String> = words.iter().map(|w| w.as_ref().to_owned()).collect();
    let Some(last) = out.pop() else { return out };
    let s = if ends_with_ignore_case(&last, "ies") {
        format!("{}y", &last[..last.len() - 3])
    } else if ["ches", "shes", "sses", "xes"]
        .iter()
        .any(|x| ends_with_ignore_case(&last, x))
    {
        // `branches` -> `branch` (knock-mgmt-go's `BranchService`).
        last[..last.len() - 2].to_owned()
    } else if ["ss", "us", "is"].iter().any(|x| ends_with_ignore_case(&last, x)) {
        last
    } else if ends_with_ignore_case(&last, "s") {
        last[..last.len() - 1].to_owned()
    } else {
        last
    };
    out.push(s);
    out
}

/// Split an identifier like `listUsers`, `list_users` or `HTTPServer` into words (the same
/// splitting as the Python plugin's namer and Stainless's).
#[must_use]
pub fn split_words(identifier: &str) -> Vec<String> {
    let chars: Vec<char> = identifier.chars().collect();
    let mut first = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c.is_ascii_lowercase() || c.is_ascii_digit()) && chars.get(i + 1).is_some_and(char::is_ascii_uppercase) {
            first.extend([c, ' ', chars[i + 1]]);
            i += 2;
        } else {
            first.push(c);
            i += 1;
        }
    }
    let mut second = Vec::with_capacity(first.len());
    let mut i = 0;
    while i < first.len() {
        if !first[i].is_ascii_uppercase() {
            second.push(first[i]);
            i += 1;
            continue;
        }
        let mut end = i;
        while end < first.len() && first[end].is_ascii_uppercase() {
            end += 1;
        }
        if end - i >= 2 && first.get(end).is_some_and(char::is_ascii_lowercase) {
            second.extend_from_slice(&first[i..end - 1]);
            second.extend([' ', first[end - 1], first[end]]);
            i = end + 1;
        } else {
            second.extend_from_slice(&first[i..end]);
            i = end;
        }
    }
    second
        .split(|c| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.iter().collect())
        .collect()
}

/// The words of a name as written: a field's wire name split as Stainless splits it.
#[must_use]
pub fn words(name: &Name) -> Vec<String> {
    let split = split_words(&name.written);
    if split.is_empty() { name.words.clone() } else { split }
}

/// The Go method name of a stainless.yml method: the standard methods use Stainless's verbs.
#[must_use]
pub fn method_name(name: &Name, casings: Casings<'_>) -> String {
    match name.written.as_str() {
        "retrieve" => "Get".to_owned(),
        "create" => "New".to_owned(),
        other => exported(&split_words(other), casings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Casings<'static> = Casings(&[]);

    #[test]
    fn spells_exported_names_with_initialisms() {
        assert_eq!(exported(&split_words("fareUrl"), NONE), "FareURL");
        assert_eq!(exported(&split_words("stopIds"), NONE), "StopIDs");
        assert_eq!(exported(&split_words("id"), NONE), "ID");
        assert_eq!(exported(&split_words("nullSafeShortName"), NONE), "NullSafeShortName");
        assert_eq!(exported(&split_words("https"), NONE), "HTTPS");
        assert_eq!(exported(&split_words("json_schema"), NONE), "JsonSchema");
        assert_eq!(exported(&split_words("sms"), NONE), "SMS");
        assert_eq!(exported(&split_words("2fa"), NONE), "_2fa");
    }

    #[test]
    fn spells_local_names() {
        assert_eq!(local(&split_words("agencyID"), NONE), "agencyID");
        assert_eq!(local(&split_words("current_time"), NONE), "currentTime");
        assert_eq!(local(&split_words("type"), NONE), "type_");
        assert_eq!(unexported("AgencyGetResponse"), "agencyGetResponse");
    }

    #[test]
    fn applies_custom_casings() {
        let casings = [
            CustomCasing {
                word: "qa".into(),
                casing: Casing::Initialism(true),
            },
            CustomCasing {
                word: "lumaai".into(),
                casing: Casing::Pascal("LumaAI".into()),
            },
            CustomCasing {
                word: "id".into(),
                casing: Casing::Initialism(false),
            },
        ];
        let c = Casings(&casings);
        assert_eq!(exported(&["qas", "list"], c), "QAsList");
        assert_eq!(exported(&["lumaai"], c), "LumaAI");
        assert_eq!(exported(&["user", "id"], c), "UserId");
    }

    #[test]
    fn singularizes_the_last_word() {
        assert_eq!(singular(&split_words("trip-details")), ["trip", "detail"]);
        assert_eq!(
            singular(&split_words("agencies_with_coverage")),
            ["agencies", "with", "coverage"]
        );
        assert_eq!(singular(&["status"]), ["status"]);
        assert_eq!(singular(&["branches"]), ["branch"]);
        assert_eq!(singular(&["boxes"]), ["box"]);
        assert_eq!(singular(&["nearby", "stop", "Ids"]), ["nearby", "stop", "Id"]);
    }

    #[test]
    fn names_standard_methods() {
        let name = |w: &str| Name {
            written: w.into(),
            words: vec![w.into()],
        };
        assert_eq!(method_name(&name("retrieve"), NONE), "Get");
        assert_eq!(method_name(&name("create"), NONE), "New");
        assert_eq!(method_name(&name("list"), NONE), "List");
        assert_eq!(method_name(&name("get_settings"), NONE), "GetSettings");
    }
}
