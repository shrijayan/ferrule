//! TypeScript spellings of model names, the way Stainless spells them: `PascalCase` classes and
//! types (acronyms upper-cased, `custom_casings` applied), `camelCase` properties and methods,
//! `kebab-case` files.

use ferrule_sdk_model::model::{Casing, CustomCasing, Name};

/// Words Stainless spells upper-case inside type names, such as `URLEndpoints`, `APIKeys`.
const ACRONYMS: [&str; 12] = [
    "id", "ids", "url", "uri", "api", "http", "https", "ip", "json", "uuid", "sdk", "ai",
];

/// Reserved words a TypeScript identifier cannot be.
const RESERVED: [&str; 37] = [
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

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

fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_ascii_uppercase().to_string() + chars.as_str())
}

fn casing<'a>(casings: &'a [CustomCasing], word: &str) -> Option<&'a Casing> {
    casings.iter().find(|c| c.word == word).map(|c| &c.casing)
}

/// One word of a type name: custom casing, else an upper-cased acronym, else capitalized.
fn type_word(w: &str, casings: &[CustomCasing]) -> String {
    let lower = w.to_ascii_lowercase();
    match casing(casings, &lower) {
        Some(Casing::Pascal(p)) => return p.clone(),
        Some(Casing::Initialism(true)) => return lower.to_ascii_uppercase(),
        Some(Casing::Initialism(false)) => return capitalize(w),
        None => {}
    }
    if let Some(stem) = lower.strip_suffix('s')
        && matches!(casing(casings, stem), Some(Casing::Initialism(true)))
    {
        return format!("{}s", stem.to_ascii_uppercase());
    }
    if !ACRONYMS.contains(&lower.as_str()) {
        capitalize(w)
    } else if lower.ends_with('s') && lower != "https" {
        format!("{}s", lower[..lower.len() - 1].to_ascii_uppercase())
    } else {
        lower.to_ascii_uppercase()
    }
}

/// `["url", "endpoints"]` -> `URLEndpoints`; `["qa"]` with the `qa` initialism -> `QA`.
#[must_use]
pub fn type_name<S: AsRef<str>>(words: &[S], casings: &[CustomCasing]) -> String {
    clean(words).iter().map(|w| type_word(w, casings)).collect()
}

/// `["url", "endpoints"]` -> `urlEndpoints`: the first word lower case, the rest as in type names.
#[must_use]
pub fn camel_case<S: AsRef<str>>(words: &[S], casings: &[CustomCasing]) -> String {
    let words = clean(words);
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        if i == 0 {
            match casing(casings, &w.to_ascii_lowercase()) {
                Some(Casing::Pascal(p)) if p.chars().next().is_some_and(|c| c.is_ascii_lowercase()) => out.push_str(p),
                _ => out.push_str(&w.to_ascii_lowercase()),
            }
        } else {
            out.push_str(&type_word(w, casings));
        }
    }
    out
}

/// `["url", "endpoints"]` -> `url-endpoints`.
#[must_use]
pub fn kebab_case<S: AsRef<str>>(words: &[S]) -> String {
    clean(words)
        .iter()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// The words of a name: its words, or the words of its spelling when it has none.
#[must_use]
pub fn words_of(name: &Name) -> Vec<String> {
    if name.words.is_empty() {
        split_words(&name.written)
    } else {
        name.words.clone()
    }
}

fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// The singular of the last word, as Stainless names params types and array items:
/// `embeddings` -> `embedding`, `agencies` -> `agency`.
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

/// Splits an identifier like `listUsers`, `list_users` or `HTTPServer` into lower-case words.
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
        .split(|c: &char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.iter().collect::<String>().to_ascii_lowercase())
        .collect()
}

/// `name` when it is a valid identifier, else `name_`.
#[must_use]
pub fn guard(name: String) -> String {
    let safe = if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    };
    if RESERVED.contains(&safe.as_str()) {
        format!("_{safe}")
    } else {
        safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn casings() -> Vec<CustomCasing> {
        vec![
            CustomCasing {
                word: "qa".to_owned(),
                casing: Casing::Initialism(true),
            },
            CustomCasing {
                word: "v1".to_owned(),
                casing: Casing::Pascal("v1".to_owned()),
            },
        ]
    }

    #[test]
    fn spells_types_with_acronyms_and_custom_casings() {
        assert_eq!(type_name(&["url", "endpoints"], &[]), "URLEndpoints");
        assert_eq!(type_name(&["api", "keys"], &[]), "APIKeys");
        assert_eq!(type_name(&["qa"], &casings()), "QA");
        assert_eq!(type_name(&["v1"], &casings()), "v1");
        assert_eq!(type_name(&["v2"], &[]), "V2");
    }

    #[test]
    fn spells_properties_files_and_singulars() {
        assert_eq!(camel_case(&["url", "endpoints"], &[]), "urlEndpoints");
        assert_eq!(camel_case(&["list", "logs"], &[]), "listLogs");
        assert_eq!(camel_case(&["qa"], &casings()), "qa");
        assert_eq!(kebab_case(&["channel", "groups"]), "channel-groups");
        assert_eq!(singular(&["embeddings"]), ["embedding"]);
        assert_eq!(singular(&["agencies"]), ["agency"]);
        assert_eq!(split_words("ILGSv1Document"), ["ilg", "sv1", "document"]);
    }
}
