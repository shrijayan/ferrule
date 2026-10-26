//! The PHP namer: word lists from the neutral model spelled as PHP identifiers, the way
//! Stainless's PHP generator spells them.
//!
//! Classes, namespaces and services are `PascalCase` with Stainless's initialisms upper-cased
//! (`URLEndpoints`, `AITag`); `x-ferrule-casings` (`custom_casings`) overrides single words.
//! Properties, parameters and methods are `camelCase` with the same initialisms after the first
//! word (`fileID`, `aiTags`, `getFromURL`); enum cases are `SCREAMING_SNAKE_CASE`, prefixed with
//! `_` when they would start with a digit (`_240`). Synthesized types are named by position: a
//! method's types start with its resource's key, the last word singular (`FolderCopyParams` for
//! `folders`), and inline objects are nested classes named after their field (array items
//! singular: `aiTags` -> `AITag`).

use ferrule_sdk_model::model::{Casing, CustomCasing, Name};

/// Words Stainless spells upper-case in names; a plural `s` stays lower-case (`IDs`).
const INITIALISMS: [&str; 12] = [
    "id", "ids", "url", "uri", "api", "http", "https", "ip", "json", "uuid", "sdk", "ai",
];

/// Names a PHP class cannot take (reserved words and type names); Stainless appends `_`.
const RESERVED: [&str; 37] = [
    "abstract",
    "and",
    "array",
    "as",
    "bool",
    "break",
    "callable",
    "case",
    "catch",
    "class",
    "clone",
    "const",
    "continue",
    "default",
    "do",
    "else",
    "enum",
    "false",
    "float",
    "for",
    "function",
    "int",
    "interface",
    "iterable",
    "list",
    "match",
    "mixed",
    "never",
    "new",
    "null",
    "object",
    "parent",
    "self",
    "static",
    "string",
    "true",
    "void",
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
        capitalize(&lower)
    } else if lower.ends_with('s') && lower != "https" {
        format!("{}s", lower[..lower.len() - 1].to_ascii_uppercase())
    } else {
        lower.to_ascii_uppercase()
    }
}

/// A class, namespace segment or service name: `["url", "endpoints"]` -> `URLEndpoints`.
#[must_use]
pub fn class_name<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let name: String = clean(words).iter().map(|w| class_word(w, casings)).collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// The name of a declared class: [`class_name`], with `_` after a reserved word (`Mixed_`).
#[must_use]
pub fn type_name<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let name = class_name(words, casings);
    if RESERVED.contains(&name.to_ascii_lowercase().as_str()) {
        format!("{name}_")
    } else {
        name
    }
}

/// A property, parameter or method name: `["base", "url", "for", "canonical", "header"]` ->
/// `baseURLForCanonicalHeader`, `["ai", "tags"]` -> `aiTags`.
#[must_use]
pub fn camel<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let words = clean(words);
    let Some((first, rest)) = words.split_first() else {
        return String::new();
    };
    let head = match casings.get(&first.to_ascii_lowercase()) {
        Some(Casing::Pascal(p)) => {
            let mut chars = p.chars();
            chars
                .next()
                .map_or_else(String::new, |c| c.to_ascii_lowercase().to_string() + chars.as_str())
        }
        _ => first.to_ascii_lowercase(),
    };
    let name: String = std::iter::once(head)
        .chain(rest.iter().map(|w| class_word(w, casings)))
        .collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// An enum case: `stop_name_wrong` -> `STOP_NAME_WRONG`, `240` -> `_240`.
#[must_use]
pub fn case_name<S: AsRef<str>>(words: &[S]) -> String {
    let name = clean(words)
        .iter()
        .map(|w| w.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
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
        assert_eq!(class_name(&split_words("urlEndpoints"), NONE), "URLEndpoints");
        assert_eq!(class_name(&split_words("AITag"), NONE), "AITag");
        assert_eq!(class_name(&split_words("removeAiTags"), NONE), "RemoveAITags");
        assert_eq!(class_name(&split_words("imagekit"), NONE), "Imagekit");
        assert_eq!(type_name(&split_words("mixed"), NONE), "Mixed_");
        assert_eq!(class_name(&split_words("list"), NONE), "List");
    }

    #[test]
    fn spells_camel_names() {
        assert_eq!(camel(&class_words("AITags"), NONE), "aiTags");
        assert_eq!(camel(&class_words("fileId"), NONE), "fileID");
        assert_eq!(
            camel(&class_words("successfullyDeletedFileIds"), NONE),
            "successfullyDeletedFileIDs"
        );
        assert_eq!(
            camel(&class_words("baseUrlForCanonicalHeader"), NONE),
            "baseURLForCanonicalHeader"
        );
        assert_eq!(camel(&class_words("getFromURL"), NONE), "getFromURL");
        assert_eq!(camel(&class_words("bg_image_url"), NONE), "bgImageURL");
        assert_eq!(camel(&class_words("url"), NONE), "url");
    }

    #[test]
    fn spells_enum_cases() {
        assert_eq!(case_name(&split_words("ASC_CREATED")), "ASC_CREATED");
        assert_eq!(case_name(&split_words("Pending")), "PENDING");
        assert_eq!(case_name(&split_words("240")), "_240");
        assert_eq!(case_name(&split_words("non-image")), "NON_IMAGE");
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
        assert_eq!(camel(&["lumaai", "id"], c), "lumaAIID");
    }

    #[test]
    fn singularizes_the_last_word() {
        assert_eq!(singular(&split_words("folders")), ["folders".trim_end_matches('s')]);
        assert_eq!(singular(&["ai", "tags"]), ["ai", "tag"]);
        assert_eq!(singular(&["status"]), ["status"]);
    }
}
