//! Rust spellings of the model's names (ADR 0019): `snake_case` for modules, functions and
//! fields, `UpperCamelCase` for types (RFC 430: acronyms are words, `custom_casings` are not
//! applied), and a way around keywords.

use std::collections::HashSet;

/// Every word Rust reserves (2024 edition), strict and reserved.
const KEYWORDS: [&str; 52] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract",
    "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Keywords a raw identifier cannot spell.
const NOT_RAW: [&str; 5] = ["crate", "self", "Self", "super", "_"];

/// Splits `text` into lower-case words at non-alphanumerics, case changes and letter-digit
/// boundaries that follow a lower-case letter: `stopIDs` -> `stop ids`, `trip-details` ->
/// `trip details`, `XMLDocument` -> `xml document`.
#[must_use]
pub fn split_words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1).copied();
        let boundary = prev.is_some_and(|p| {
            (p.is_lowercase() && c.is_uppercase())
                || (p.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase) && !plural(&chars, i))
                || (p.is_alphabetic() && c.is_ascii_digit() && p.is_lowercase())
        });
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Whether the upper-case letter at `i` starts a lone plural `s` (`IDs`), which stays with its
/// acronym.
fn plural(chars: &[char], i: usize) -> bool {
    chars.get(i + 1) == Some(&'s') && !chars.get(i + 2).is_some_and(|c| c.is_lowercase())
}

/// The words, lower-cased and joined by `_`: `stops_for_location`.
#[must_use]
pub fn snake(words: &[String]) -> String {
    words.iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("_")
}

/// The words, each capitalized and joined: `StopsForLocation`; `prefix` leads a name that would
/// start with a digit, and `fallback` stands for no words.
#[must_use]
pub fn pascal(words: &[String], prefix: &str, fallback: &str) -> String {
    let joined: String = words
        .iter()
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or_else(String::new, |c| {
                c.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect()
            })
        })
        .collect();
    if joined.is_empty() {
        fallback.to_owned()
    } else if joined.starts_with(|c: char| c.is_ascii_digit()) {
        format!("{prefix}{joined}")
    } else {
        joined
    }
}

/// `SCREAMING_SNAKE_CASE`.
#[must_use]
pub fn screaming(words: &[String]) -> String {
    snake(words).to_uppercase()
}

/// A field or parameter name: snake case, a raw identifier for a keyword (`r#type`), else a
/// trailing `_` (`self_`), and `field_` ahead of a leading digit.
#[must_use]
pub fn field(words: &[String]) -> String {
    let name = snake(words);
    if name.is_empty() {
        "field".to_owned()
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("field_{name}")
    } else {
        escape_raw(name)
    }
}

/// A module or method name: snake case with a trailing `_` for a keyword (`type_`), which reads
/// better than `r#type()` at a call site.
#[must_use]
pub fn item(words: &[String]) -> String {
    let name = snake(words);
    if name.is_empty() {
        "item".to_owned()
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("n{name}")
    } else if KEYWORDS.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

fn escape_raw(name: String) -> String {
    if NOT_RAW.contains(&name.as_str()) {
        format!("{name}_")
    } else if KEYWORDS.contains(&name.as_str()) {
        format!("r#{name}")
    } else {
        name
    }
}

/// Names taken in one scope; [`Taken::claim`] makes each unique by a number suffix.
#[derive(Debug, Default, Clone)]
pub struct Taken(HashSet<String>);

impl Taken {
    /// A scope where `reserved` are already taken.
    #[must_use]
    pub fn with(reserved: &[&str]) -> Self {
        Self(reserved.iter().map(|&s| s.to_owned()).collect())
    }

    /// `name`, else `name2`, `name3`, … (`name_2` for snake case), whichever is free first.
    pub fn claim(&mut self, name: &str) -> String {
        if self.0.insert(name.to_owned()) {
            return name.to_owned();
        }
        let raw = name.strip_prefix("r#").unwrap_or(name);
        let sep = if raw.contains('_') || raw.chars().next().is_some_and(char::is_lowercase) {
            "_"
        } else {
            ""
        };
        (2..usize::MAX)
            .map(|n| format!("{raw}{sep}{n}"))
            .find(|candidate| self.0.insert(candidate.clone()))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        split_words(text)
    }

    #[test]
    fn splits_words() {
        assert_eq!(words("trip-details"), ["trip", "details"]);
        assert_eq!(words("stopIDs"), ["stop", "ids"]);
        assert_eq!(words("XMLDocument"), ["xml", "document"]);
        assert_eq!(words("v1"), ["v", "1"]);
        assert_eq!(words("onebusaway-sdk"), ["onebusaway", "sdk"]);
    }

    #[test]
    fn spells_each_casing() {
        assert_eq!(snake(&words("agenciesWithCoverage")), "agencies_with_coverage");
        assert_eq!(pascal(&words("onebusaway-sdk"), "T", "Type"), "OnebusawaySdk");
        assert_eq!(pascal(&words("2fa"), "V", "Value"), "V2fa");
        assert_eq!(pascal(&[], "V", "Empty"), "Empty");
        assert_eq!(screaming(&words("cursor_page")), "CURSOR_PAGE");
    }

    #[test]
    fn escapes_keywords() {
        assert_eq!(field(&words("type")), "r#type");
        assert_eq!(field(&words("self")), "self_");
        assert_eq!(field(&words("3d")), "field_3d");
        assert_eq!(item(&words("type")), "type_");
        assert_eq!(item(&words("match")), "match_");
    }

    #[test]
    fn claims_unique_names() {
        let mut taken = Taken::with(&["send"]);
        assert_eq!(taken.claim("send"), "send_2");
        assert_eq!(taken.claim("Data"), "Data");
        assert_eq!(taken.claim("Data"), "Data2");
        assert_eq!(taken.claim("r#type"), "r#type");
        assert_eq!(taken.claim("r#type"), "type_2");
    }
}
