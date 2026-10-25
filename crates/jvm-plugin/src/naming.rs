//! The JVM namer: word lists from the neutral model spelled as Kotlin identifiers, the way
//! Stainless's Java and Kotlin generators spell them.
//!
//! Classes are `PascalCase` and members `camelCase`, with every word capitalised as a word, not
//! as an initialism (`agencyID` -> `agencyId`, `onebusaway-sdk` -> `OnebusawaySdk`);
//! `x-ferrule-casings` (`custom_casings`) overrides single words. Enum members are
//! `SCREAMING_SNAKE`. Packages are the lower-case words run together (`agencieswithcoverage`).

use ferrule_sdk_model::model::{Casing, CustomCasing, Name};

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
#[must_use]
pub fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_ascii_uppercase().to_string() + chars.as_str())
}

/// `w[0].toLowerCase() + w.slice(1)` on an ASCII word.
#[must_use]
pub fn decapitalize(w: &str) -> String {
    let mut chars = w.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_ascii_lowercase().to_string() + chars.as_str())
}

/// One capitalised word: `ID` -> `Id`, `lumaai` -> `LumaAI` with that custom casing.
fn word(w: &str, casings: Casings<'_>) -> String {
    let lower = w.to_ascii_lowercase();
    match casings.get(&lower) {
        Some(Casing::Pascal(p)) => p.clone(),
        Some(Casing::Initialism(true)) => lower.to_ascii_uppercase(),
        Some(Casing::Initialism(false)) | None => capitalize(&lower),
    }
}

/// A class name: `["agencies", "with", "coverage"]` -> `AgenciesWithCoverage`.
#[must_use]
pub fn pascal<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let name: String = clean(words).iter().map(|w| word(w, casings)).collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// A member name: `["agency", "id"]` -> `agencyId`.
#[must_use]
pub fn camel<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let words = clean(words);
    let Some((first, rest)) = words.split_first() else {
        return "_".to_owned();
    };
    let mut name = first.to_ascii_lowercase();
    name.extend(rest.iter().map(|w| word(w, casings)));
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// An enum member or constant: `stop_name_wrong` -> `STOP_NAME_WRONG`.
#[must_use]
pub fn constant<S: AsRef<str>>(words: &[S]) -> String {
    let name = clean(words)
        .iter()
        .map(|w| w.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// A package segment: `["trip", "details"]` -> `tripdetails`.
#[must_use]
pub fn package<S: AsRef<str>>(words: &[S]) -> String {
    clean(words).iter().map(|w| w.to_ascii_lowercase()).collect()
}

/// Title-cased words with `sdk`/`api` upper-cased, as the client docs name the API:
/// `onebusaway-sdk` -> `Onebusaway SDK`, `ImageKit` -> `Image Kit`.
#[must_use]
pub fn title<S: AsRef<str>>(words: &[S]) -> String {
    clean(words)
        .iter()
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            if matches!(lower.as_str(), "sdk" | "api") {
                lower.to_ascii_uppercase()
            } else {
                capitalize(&lower)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    let split = split_words(&name.written);
    if split.is_empty() { name.words.clone() } else { split }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Casings<'static> = Casings(&[]);

    #[test]
    fn spells_classes_and_members() {
        assert_eq!(pascal(&split_words("onebusaway-sdk"), NONE), "OnebusawaySdk");
        assert_eq!(pascal(&split_words("ImageKit"), NONE), "ImageKit");
        assert_eq!(camel(&split_words("agencyID"), NONE), "agencyId");
        assert_eq!(camel(&split_words("fareUrl"), NONE), "fareUrl");
        assert_eq!(pascal(&split_words("getFromURL"), NONE), "GetFromUrl");
        assert_eq!(camel(&split_words("current_time"), NONE), "currentTime");
        assert_eq!(constant(&split_words("stop_name_wrong")), "STOP_NAME_WRONG");
        assert_eq!(package(&split_words("trip-details")), "tripdetails");
    }

    #[test]
    fn titles_the_organization() {
        assert_eq!(title(&split_words("onebusaway-sdk")), "Onebusaway SDK");
        assert_eq!(title(&split_words("ImageKit")), "Image Kit");
    }

    #[test]
    fn applies_custom_casings() {
        let casings = [CustomCasing {
            word: "lumaai".into(),
            casing: Casing::Pascal("LumaAI".into()),
        }];
        assert_eq!(pascal(&["lumaai", "client"], Casings(&casings)), "LumaAIClient");
    }

    #[test]
    fn singularizes_the_last_word() {
        assert_eq!(singular(&split_words("trip-details")), ["trip", "detail"]);
        assert_eq!(singular(&split_words("stopTimes")), ["stop", "Time"]);
        assert_eq!(singular(&["status"]), ["status"]);
    }
}
