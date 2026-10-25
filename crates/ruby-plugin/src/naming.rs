//! The Ruby namer: word lists from the neutral model spelled as Ruby identifiers, the way
//! Stainless's Ruby generator spells them.
//!
//! Classes and modules are `PascalCase` with Stainless's initialisms upper-cased (`StopIDsForAgency`,
//! `URL`); `x-ferrule-casings` (`custom_casings`) overrides single words. Methods, attributes and
//! files are `snake_case`; enum members are `SCREAMING_SNAKE_CASE`. Synthesized types are named by
//! position: a method's types start with its resource's key, the last word singular
//! (`TripDetailRetrieveResponse` for `trip-details`), and inline objects are nested classes named
//! after their field (array items singular: `stopTimes` -> `StopTime`).

use ferrule_sdk_model::model::{Casing, CustomCasing, Name};

/// Words Stainless spells upper-case in class names; a plural `s` stays lower-case (`IDs`).
const INITIALISMS: [&str; 11] = [
    "id", "ids", "url", "uri", "api", "http", "https", "ip", "json", "uuid", "sdk",
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

/// One word of a class name.
fn class_word(w: &str, casings: Casings<'_>) -> String {
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
    if !INITIALISMS.contains(&lower.as_str()) {
        capitalize(w)
    } else if lower.ends_with('s') && lower != "https" {
        format!("{}s", lower[..lower.len() - 1].to_ascii_uppercase())
    } else {
        lower.to_ascii_uppercase()
    }
}

/// A class or module name: `["stop", "ids", "for", "agency"]` -> `StopIDsForAgency`.
#[must_use]
pub fn class_name<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let name: String = clean(words).iter().map(|w| class_word(w, casings)).collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// A method, attribute or file name: `["readable", "Time"]` -> `readable_time`.
#[must_use]
pub fn snake<S: AsRef<str>>(words: &[S]) -> String {
    let name = clean(words)
        .iter()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("_");
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// An enum member's constant: `stop_name_wrong` -> `STOP_NAME_WRONG`; a value that starts with a
/// digit is prefixed with the enum's own name (`240` in `StreamingResolution` ->
/// `STREAMING_RESOLUTION_240`).
#[must_use]
pub fn constant<S: AsRef<str>>(words: &[S], owner: &str) -> String {
    let name = clean(words)
        .iter()
        .map(|w| w.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        let prefix = snake(&split_words(owner)).to_ascii_uppercase();
        if name.is_empty() {
            prefix
        } else {
            format!("{prefix}_{name}")
        }
    } else {
        name
    }
}

/// The words of a class name, an initialism's plural kept whole: `RouteIDsForAgency` ->
/// `route ids for agency` (not `route i ds ...`).
#[must_use]
pub fn class_words(name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in split_words(name) {
        let plural = w.len() == 2 && w.starts_with(|c: char| c.is_ascii_uppercase()) && w.ends_with('s');
        match out.last_mut() {
            Some(prev) if plural && prev.chars().all(|c| c.is_ascii_uppercase()) => prev.push_str(&w),
            _ => out.push(w),
        }
    }
    out
}

/// Words RBS reserves or that name methods every module has, which a type alias cannot be.
const RBS_RESERVED: [&str; 34] = [
    "alias",
    "attr_accessor",
    "attr_reader",
    "attr_writer",
    "bool",
    "bot",
    "class",
    "def",
    "end",
    "extend",
    "false",
    "include",
    "instance",
    "interface",
    "module",
    "nil",
    "out",
    "prepend",
    "private",
    "public",
    "self",
    "singleton",
    "super",
    "top",
    "true",
    "type",
    "unchecked",
    "untyped",
    "void",
    "use",
    "format",
    "method",
    "name",
    "y",
];

/// The RBS type alias of a class or module: `ReportProblemWithStopRetrieveParams` ->
/// `report_problem_with_stop_retrieve_params`, `Type` -> `type_`.
#[must_use]
pub fn rbs_alias(class: &str) -> String {
    let name = snake(&class_words(class));
    if RBS_RESERVED.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// The words with the last one singular, as Stainless names array items and method types:
/// `agencies` -> `agency`, `trip details` -> `trip detail`.
#[must_use]
pub fn singular<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    let mut out: Vec<String> = words.iter().map(|w| w.as_ref().to_owned()).collect();
    let Some(last) = out.pop() else { return out };
    let s = if ends_with_ignore_case(&last, "ies") {
        format!("{}y", &last[..last.len() - 3])
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
/// splitting as the other plugins' namers and Stainless's).
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
    let split = class_words(&name.written);
    if split.is_empty() { name.words.clone() } else { split }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Casings<'static> = Casings(&[]);

    #[test]
    fn spells_class_names_with_initialisms() {
        assert_eq!(
            class_name(&split_words("stop_ids_for_agency"), NONE),
            "StopIDsForAgency"
        );
        assert_eq!(class_name(&split_words("url"), NONE), "URL");
        assert_eq!(class_name(&split_words("onebusaway-sdk"), NONE), "OnebusawaySDK");
        assert_eq!(class_name(&split_words("trip-details"), NONE), "TripDetails");
        assert_eq!(class_name(&split_words("imagekitio"), NONE), "Imagekitio");
    }

    #[test]
    fn spells_snake_names_and_constants() {
        assert_eq!(snake(&split_words("readableTime")), "readable_time");
        assert_eq!(snake(&class_words("stopIDs")), "stop_ids");
        assert_eq!(snake(&split_words("trip-details")), "trip_details");
        assert_eq!(constant(&split_words("stop_name_wrong"), "Code"), "STOP_NAME_WRONG");
        assert_eq!(
            constant(&split_words("240"), "StreamingResolution"),
            "STREAMING_RESOLUTION_240"
        );
    }

    #[test]
    fn spells_rbs_aliases() {
        assert_eq!(
            rbs_alias("RouteIDsForAgencyListParams"),
            "route_ids_for_agency_list_params"
        );
        assert_eq!(rbs_alias("AITag"), "ai_tag");
        assert_eq!(rbs_alias("Name"), "name_");
        assert_eq!(rbs_alias("URL"), "url");
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
        ];
        let c = Casings(&casings);
        assert_eq!(class_name(&["qas", "list"], c), "QAsList");
        assert_eq!(class_name(&["lumaai"], c), "LumaAI");
    }

    #[test]
    fn singularizes_the_last_word() {
        assert_eq!(singular(&split_words("trip-details")), ["trip", "detail"]);
        assert_eq!(singular(&["stop", "times"]), ["stop", "time"]);
        assert_eq!(singular(&["status"]), ["status"]);
        assert_eq!(singular(&["agencies"]), ["agency"]);
    }
}
