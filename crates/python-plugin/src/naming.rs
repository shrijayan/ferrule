//! Names are carried as word lists; these render them in Python conventions.

const PY_KEYWORDS: [&str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
    "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

/// Words Stainless spells upper-case inside class names, such as `SituationURL`.
const ACRONYMS: [&str; 11] = [
    "id", "ids", "url", "uri", "api", "http", "https", "ip", "json", "uuid", "sdk",
];

/// Words Stainless spells upper-case in class names it makes from a schema name, as listed in the
/// `initialism` description of the public stainless.yml schema (the converter's
/// `DEFAULT_INITIALISMS`, which it applies to model names).
const DEFAULT_INITIALISMS: [&str; 39] = [
    "3ds", "ach", "acl", "ai", "api", "ats", "cpu", "crm", "db", "dns", "e2e", "eeoc", "gb", "gpu", "hd", "hris",
    "html", "http", "https", "id", "ip", "iso", "jsonl", "kb", "kyb", "kyc", "lfs", "mb", "mp3", "mp4", "nft", "saml",
    "sdk", "sku", "sms", "ss", "ssh", "sso", "url",
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

fn guard(name: String) -> String {
    let safe = if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{name}")
    } else {
        name
    };
    if PY_KEYWORDS.contains(&safe.as_str()) {
        format!("{safe}_")
    } else {
        safe
    }
}

/// `w.charAt(0).toUpperCase() + w.slice(1)` on an ASCII word.
fn capitalize(w: &str) -> String {
    let mut chars = w.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_ascii_uppercase().to_string() + chars.as_str())
}

/// `["list", "Users"]` -> `list_users`
#[must_use]
pub fn snake_case<S: AsRef<str>>(words: &[S]) -> String {
    guard(
        clean(words)
            .iter()
            .map(|w| w.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join("_"),
    )
}

/// `["user", "ID"]` -> `UserID`; existing inner capitals are preserved.
#[must_use]
pub fn pascal_case<S: AsRef<str>>(words: &[S]) -> String {
    guard(clean(words).iter().map(|w| capitalize(w)).collect())
}

/// A class-name segment: `PascalCase` with acronyms upper-cased; `ids` becomes `IDs`.
#[must_use]
pub fn class_segment<S: AsRef<str>>(words: &[S]) -> String {
    class_segment_with(words, &[])
}

/// How `custom_casings` spells one word in class names (`x-ferrule-casings`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Casing {
    /// `initialism: true` upper-cases it (`qa` -> `QA`); `false` stops treating it as one.
    Initialism(bool),
    /// The spelling as written, first letter included: `LumaAI`, `v1`.
    Pascal(String),
}

/// [`class_segment`] with custom casings by lowercase word: `["qa"]` -> `QA`, `["lumaai"]` ->
/// `LumaAI`. A custom initialism takes a plural `s` as the built-in ones do (`QAs`).
#[must_use]
pub fn class_segment_with<S: AsRef<str>>(words: &[S], casings: &[(String, Casing)]) -> String {
    let casing = |w: &str| casings.iter().find(|(k, _)| k == w).map(|(_, c)| c);
    clean(words)
        .iter()
        .map(|w| {
            let lower = w.to_ascii_lowercase();
            match casing(&lower) {
                Some(Casing::Pascal(p)) => return p.clone(),
                Some(Casing::Initialism(true)) => return lower.to_ascii_uppercase(),
                Some(Casing::Initialism(false)) => return capitalize(w),
                None => {}
            }
            if let Some(stem) = lower.strip_suffix('s')
                && matches!(casing(stem), Some(Casing::Initialism(true)))
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
        })
        .collect()
}

/// A class-name segment from a schema name, as Stainless names a union variant after the schema
/// it comes from: each word capitalised and the rest lower-case, a default initialism upper-case
/// (knock-mgmt `EmailHtmlBlock` is `EmailHTMLBlock`, ogx `OpenAIResponseInputToolChoiceMCPTool`
/// is `OpenAIResponseInputToolChoiceMcpTool`). A plural initialism keeps its `s` (`APIs`).
#[must_use]
pub fn schema_segment(name: &str) -> String {
    guard(
        clean(&split_words(name))
            .iter()
            .map(|w| {
                let lower = w.to_ascii_lowercase();
                if DEFAULT_INITIALISMS.contains(&lower.as_str()) {
                    return lower.to_ascii_uppercase();
                }
                if let Some(stem) = lower.strip_suffix('s')
                    && DEFAULT_INITIALISMS.contains(&stem)
                {
                    return format!("{}s", stem.to_ascii_uppercase());
                }
                capitalize(&lower)
            })
            .collect(),
    )
}

fn ends_with_ignore_case(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len()
        && s.is_char_boundary(s.len() - suffix.len())
        && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// The singular of the last word, as Stainless names array item classes: `agencies` -> `agency`,
/// `branches` -> `branch` (knock-mgmt `Branch`), `submatches` -> `submatch` (perplexity).
#[must_use]
pub fn singular<S: AsRef<str>>(words: &[S]) -> Vec<String> {
    let mut out: Vec<String> = words.iter().map(|w| w.as_ref().to_owned()).collect();
    let Some(last) = out.pop() else { return out };
    let s = if ends_with_ignore_case(&last, "ies") {
        format!("{}y", &last[..last.len() - 3])
    } else if ["ches", "shes", "xes", "sses", "zzes"]
        .iter()
        .any(|x| ends_with_ignore_case(&last, x))
    {
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

/// Split an identifier like `listUsers`, `list_users` or `HTTPServer` into words.
#[must_use]
pub fn split_words(identifier: &str) -> Vec<String> {
    // `.replace(/([a-z0-9])([A-Z])/g, '$1 $2')`
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
    // `.replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')`
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
    // `.split(/[^A-Za-z0-9]+/).filter((w) => w.length > 0)`
    second
        .split(|c| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.iter().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_cases_word_lists() {
        assert_eq!(snake_case(&["list", "Users"]), "list_users");
        assert_eq!(snake_case(&["class"]), "class_");
        assert_eq!(snake_case(&["2fa", "code"]), "_2fa_code");
        assert_eq!(snake_case(&["a-b", "", "c"]), "ab_c");
    }

    #[test]
    fn pascal_cases_word_lists() {
        assert_eq!(pascal_case(&["user", "ID"]), "UserID");
        assert_eq!(pascal_case(&["none"]), "None_");
    }

    #[test]
    fn spells_schema_names_as_variant_segments() {
        assert_eq!(schema_segment("EmailHtmlBlock"), "EmailHTMLBlock");
        assert_eq!(
            schema_segment("OpenAIResponseInputToolChoiceMCPTool"),
            "OpenAIResponseInputToolChoiceMcpTool"
        );
        assert_eq!(schema_segment("ResponseFormatJSONSchema"), "ResponseFormatJsonSchema");
        assert_eq!(schema_segment("list_apis"), "ListAPIs");
    }

    #[test]
    fn splits_identifiers() {
        assert_eq!(split_words("listUsers"), ["list", "Users"]);
        assert_eq!(split_words("HTTPServer_v2"), ["HTTP", "Server", "v2"]);
        assert_eq!(split_words("Acme API"), ["Acme", "API"]);
        assert_eq!(split_words("aBcD"), ["a", "Bc", "D"]);
        // Expected values printed by the TypeScript `splitWords`.
        assert_eq!(split_words("ABCdEFGh"), ["AB", "Cd", "EF", "Gh"]);
        assert_eq!(split_words("XMLHttpRequest"), ["XML", "Http", "Request"]);
        assert_eq!(split_words("ABc"), ["A", "Bc"]);
        assert_eq!(split_words("aBCDe"), ["a", "BC", "De"]);
        assert_eq!(split_words("A1BC2dE"), ["A1", "BC2d", "E"]);
    }

    #[test]
    fn spells_words_with_custom_casings() {
        let casings = [
            ("qa".to_owned(), Casing::Initialism(true)),
            ("id".to_owned(), Casing::Initialism(false)),
            ("v1".to_owned(), Casing::Pascal("v1".into())),
            ("lumaai".to_owned(), Casing::Pascal("LumaAI".into())),
        ];
        assert_eq!(class_segment_with(&["qa"], &casings), "QA");
        assert_eq!(class_segment_with(&["qas", "list"], &casings), "QAsList");
        assert_eq!(class_segment_with(&["v1"], &casings), "v1");
        assert_eq!(class_segment_with(&split_words("lumaai"), &casings), "LumaAI");
        assert_eq!(class_segment_with(&["user", "id"], &casings), "UserId");
        assert_eq!(class_segment_with(&["route", "ids"], &casings), "RouteIDs");
        assert_eq!(class_segment_with(&["situation", "url"], &casings), "SituationURL");
    }

    #[test]
    fn upper_cases_acronyms_and_singularizes() {
        assert_eq!(class_segment(&["situation", "url"]), "SituationURL");
        assert_eq!(class_segment(&["route", "ids"]), "RouteIDs");
        assert_eq!(class_segment(&["https"]), "HTTPS");
        assert_eq!(singular(&["agencies"]), ["agency"]);
        assert_eq!(singular(&["status"]), ["status"]);
        assert_eq!(singular(&["stop", "times"]), ["stop", "time"]);
        assert_eq!(singular(&["list"]), ["list"]);
        assert_eq!(singular::<&str>(&[]), Vec::<String>::new());
    }

    /// A small deterministic generator in place of fast-check: words of `[A-Za-z][A-Za-z0-9]*`.
    #[test]
    fn snake_case_output_is_always_a_valid_lowercase_identifier() {
        const ALPHABET: &[u8] = b"AZaz09Kq7_";
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..2000 {
            let n = usize::try_from(next() % 4).unwrap_or(0) + 1;
            let words: Vec<String> = (0..n)
                .map(|_| {
                    let len = usize::try_from(next() % 6).unwrap_or(0) + 1;
                    let mut w = String::new();
                    w.push(char::from(ALPHABET[usize::try_from(next() % 4).unwrap_or(0)]));
                    for _ in 1..len {
                        let c = char::from(ALPHABET[usize::try_from(next() % 9).unwrap_or(0)]);
                        if c != '_' {
                            w.push(c);
                        }
                    }
                    w
                })
                .collect();
            let s = snake_case(&words);
            assert!(s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_'), "{s}");
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{s}"
            );
        }
    }
}
