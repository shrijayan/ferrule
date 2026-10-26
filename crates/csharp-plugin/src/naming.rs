//! The C# namer: word lists from the neutral model spelled as C# identifiers, the way Stainless's
//! C# generator spells them.
//!
//! Types, members and namespaces are `PascalCase`; locals and parameters `camelCase`. Only two
//! words are initialisms, `AI` and `ID` (`requestId` -> `RequestID`, `aiTags` -> `AITags`); every
//! other word is capitalised as a word (`baseURL` -> `BaseUrl`, `GPSInfo` -> `GpsInfo`,
//! `fileIds` -> `FileIds`). `x-ferrule-casings` (`custom_casings`) overrides single words.
//! Services and the types of a method are named after the resource's key with its last word
//! singular (`accounts` -> `AccountService`, `origins.list` -> `OriginListParams`).

use ferrule_sdk_model::model::{Casing, CustomCasing, Name};

/// Words Stainless's C# generator upper-cases.
const INITIALISMS: [&str; 2] = ["ai", "id"];

/// The words of `x-ferrule-casings`, looked up by lower-case word.
#[derive(Debug, Clone, Copy)]
pub struct Casings<'a>(pub &'a [CustomCasing]);

impl Casings<'_> {
    fn get(&self, word: &str) -> Option<&Casing> {
        self.0.iter().find(|c| c.word == word).map(|c| &c.casing)
    }
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

/// One word of a `PascalCase` name.
fn word(w: &str, casings: Casings<'_>) -> String {
    let lower = w.to_ascii_lowercase();
    let upper = match casings.get(&lower) {
        Some(Casing::Pascal(p)) => return p.clone(),
        Some(Casing::Initialism(upper)) => *upper,
        None => INITIALISMS.contains(&lower.as_str()),
    };
    if upper {
        lower.to_ascii_uppercase()
    } else {
        capitalize(&lower)
    }
}

fn guard(name: String) -> String {
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    }
}

/// A type or member name: `["request", "id"]` -> `RequestID`.
#[must_use]
pub fn pascal<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    guard(words.iter().map(|w| word(w.as_ref(), casings)).collect())
}

/// A parameter or local: `["file", "id"]` -> `fileID`, `["id"]` -> `id`.
#[must_use]
pub fn camel<S: AsRef<str>>(words: &[S], casings: Casings<'_>) -> String {
    let Some((first, rest)) = words.split_first() else {
        return "_".to_owned();
    };
    let mut name = first.as_ref().to_ascii_lowercase();
    name.extend(rest.iter().map(|w| word(w.as_ref(), casings)));
    guard(name)
}

/// A `camelCase` spelling of a `PascalCase` name: `CustomMetadataField` -> `customMetadataField`,
/// `AITag` -> `aiTag`, `ID` -> `id`.
#[must_use]
pub fn camel_of(pascal: &str) -> String {
    camel(&split_words(pascal), Casings(&[]))
}

fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// The words with the last one singular, as Stainless names services and method types:
/// `custom metadata fields` -> `custom metadata field`, `usage analytics` stays.
#[must_use]
pub fn singular<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    let mut out: Vec<String> = words.iter().map(|w| w.as_ref().to_owned()).collect();
    let Some(last) = out.pop() else { return out };
    let s = if ends_with_ignore_case(&last, "ies") {
        format!("{}y", &last[..last.len() - 3])
    } else if ["ss", "us", "is", "ics"]
        .iter()
        .any(|x| ends_with_ignore_case(&last, x))
    {
        last
    } else if ends_with_ignore_case(&last, "s") {
        last[..last.len() - 1].to_owned()
    } else {
        last
    };
    out.push(s);
    out
}

/// The words with the last one plural: `origin response` -> `origin responses`.
#[must_use]
pub fn plural<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    let mut out: Vec<String> = words.iter().map(|w| w.as_ref().to_owned()).collect();
    let Some(last) = out.pop() else { return out };
    let s = if ends_with_ignore_case(&last, "s") {
        last
    } else if ends_with_ignore_case(&last, "y") && !last[..last.len() - 1].ends_with(['a', 'e', 'i', 'o', 'u']) {
        format!("{}ies", &last[..last.len() - 1])
    } else {
        format!("{last}s")
    };
    out.push(s);
    out
}

/// Split an identifier like `listUsers`, `list_users` or `HTTPServer` into words, as
/// Stainless's namers split it: `YCbCrPositioning` -> `Y Cb Cr Positioning`.
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

/// The words of a name as written (a wire name or a stainless.yml key), split as Stainless
/// splits it; the model's own words when the spelling has no letters or digits.
#[must_use]
pub fn words(name: &Name) -> Vec<String> {
    let split = split_words(&name.written);
    if split.is_empty() { name.words.clone() } else { split }
}

/// Title-cased words, as the project files name the organisation: `ImageKit` -> `Image Kit`,
/// `stagehand` -> `Stagehand`.
#[must_use]
pub fn title<S: AsRef<str>>(words: &[S]) -> String {
    words
        .iter()
        .map(|w| capitalize(&w.as_ref().to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// C# keywords, which a parameter or local cannot be named without `@`.
pub const KEYWORDS: [&str; 77] = [
    "abstract",
    "as",
    "base",
    "bool",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "checked",
    "class",
    "const",
    "continue",
    "decimal",
    "default",
    "delegate",
    "do",
    "double",
    "else",
    "enum",
    "event",
    "explicit",
    "extern",
    "false",
    "finally",
    "fixed",
    "float",
    "for",
    "foreach",
    "goto",
    "if",
    "implicit",
    "in",
    "int",
    "interface",
    "internal",
    "is",
    "lock",
    "long",
    "namespace",
    "new",
    "null",
    "object",
    "operator",
    "out",
    "override",
    "params",
    "private",
    "protected",
    "public",
    "readonly",
    "ref",
    "return",
    "sbyte",
    "sealed",
    "short",
    "sizeof",
    "stackalloc",
    "static",
    "string",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "uint",
    "ulong",
    "unchecked",
    "unsafe",
    "ushort",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
];

/// A local or parameter name that is not a keyword.
#[must_use]
pub fn local(name: String) -> String {
    if KEYWORDS.contains(&name.as_str()) {
        format!("@{name}")
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Casings<'static> = Casings(&[]);

    fn p(s: &str) -> String {
        pascal(&split_words(s), NONE)
    }

    #[test]
    fn spells_only_ai_and_id_as_initialisms() {
        assert_eq!(p("requestId"), "RequestID");
        assert_eq!(p("AITags"), "AITags");
        assert_eq!(p("removeAiTags"), "RemoveAITags");
        assert_eq!(p("getFromURL"), "GetFromUrl");
        assert_eq!(p("baseURL"), "BaseUrl");
        assert_eq!(p("GPSAltitude"), "GpsAltitude");
        assert_eq!(p("YCbCrPositioning"), "YCbCrPositioning");
        assert_eq!(p("x_request_id"), "XRequestID");
        assert_eq!(p("fileIds"), "FileIds");
        assert_eq!(p("ignoreHTTPSErrors"), "IgnoreHttpsErrors");
        assert_eq!(p("pHash"), "PHash");
    }

    #[test]
    fn spells_locals() {
        assert_eq!(camel(&split_words("fileId"), NONE), "fileID");
        assert_eq!(camel(&split_words("id"), NONE), "id");
        assert_eq!(camel_of("CustomMetadataField"), "customMetadataField");
        assert_eq!(camel_of("OriginResponse"), "originResponse");
        assert_eq!(local("params".to_owned()), "@params");
    }

    #[test]
    fn singularizes_and_pluralizes_the_last_word() {
        assert_eq!(
            singular(&["custom", "metadata", "fields"]),
            ["custom", "metadata", "field"]
        );
        assert_eq!(singular(&["usage", "analytics"]), ["usage", "analytics"]);
        assert_eq!(singular(&["accounts"]), ["account"]);
        assert_eq!(singular(&["cache"]), ["cache"]);
        assert_eq!(plural(&["origin", "response"]), ["origin", "responses"]);
        assert_eq!(plural(&["files"]), ["files"]);
        assert_eq!(title(&split_words("ImageKit")), "Image Kit");
    }
}
