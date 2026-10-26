//! The Python runtime, rendered from the template `build.rs` embeds (ADR 0020).
//!
//! A port of the default render of `ferrule_runtime` (`runtime/src/ferrule_runtime/_render.py`):
//! the options `ferrule generate` renders with, no pagination module and no `testing/` fake
//! server. Directives are resolved first, then tokens are replaced, in the text and in the path;
//! a token left over, or two templates rendering to one path, is an error. Pure:
//! `migrate/tests/test_generate.py` checks it gives the bytes `ferrule_runtime.render` gives.

include!(concat!(env!("OUT_DIR"), "/python_template.rs"));

/// The directive marker of a Python template line.
const MARKER: &str = "#ferrule:";

/// Template paths under this prefix are the `<pkg>.testing` fake server (ADR 0016), off by default.
const TESTING_DIR: &str = "testing/";

/// The line `base_model_hint_fits` measures, with `{pkg}` for the package name.
const BASE_MODEL_HINT: &str = "            raise TypeError(\"Pydantic models must subclass our base model type, e.g. `from {pkg} import BaseModel`\")";

/// Ruff's line length in a Stainless-generated SDK.
const LINE_LENGTH: usize = 120;

/// Default request timeout in seconds, and its comment (`timeout_comment(60)`).
const TIMEOUT: &str = "60";
const TIMEOUT_COMMENT: &str = "1 minute";

/// Default number of retries.
const MAX_RETRIES: &str = "2";

/// Python's reserved words, which `_check_identifier` refuses as names.
const KEYWORDS: [&str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
    "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

/// `default_client_name`: `acme_sdk` -> `AcmeSdk`, the `PascalCase` of the package name.
#[must_use]
pub fn default_client_name(package: &str) -> String {
    package
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}

/// `_screaming_snake`: `OnebusawaySDK` -> `ONEBUSAWAY_SDK`, an `_` before every capital that
/// follows a lower-case letter or a digit.
fn screaming_snake(name: &str) -> String {
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase() && prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit()) {
            out.push('_');
        }
        out.extend(c.to_uppercase());
        prev = Some(c);
    }
    out
}

/// `_check_identifier`: an ASCII Python identifier that is not a keyword.
fn check_identifier(kind: &str, value: &str) -> Result<(), String> {
    let mut chars = value.chars();
    let ident = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ident && !KEYWORDS.contains(&value) {
        Ok(())
    } else {
        Err(format!("{kind} must be a Python identifier, got '{value}'"))
    }
}

/// The runtime for `package`: paths relative to `src/<package>/`, with their text, in
/// template order.
///
/// # Errors
///
/// When `package` is not a Python identifier, or the template has a malformed directive, an
/// unknown flag or a token the render does not replace.
pub fn render(package: &str) -> Result<Vec<(String, String)>, String> {
    let client = default_client_name(package);
    let error = format!("{client}Error");
    for (kind, value) in [
        ("package_name", package),
        ("client_name", &client),
        ("error_name", &error),
    ] {
        check_identifier(kind, value)?;
    }
    let log_env = format!("{}_LOG", screaming_snake(&client));
    // Longest first so no token is a prefix of a replaced value, as `render_options` lists them.
    let replacements: [(&str, &str); 8] = [
        ("__FERRULE_FILE_UPLOADS_HINT__", ""),
        ("__FERRULE_TIMEOUT_COMMENT__", TIMEOUT_COMMENT),
        ("__FERRULE_MAX_RETRIES__", MAX_RETRIES),
        ("__FERRULE_TIMEOUT__", TIMEOUT),
        ("__FERRULE_LOG_ENV__", &log_env),
        ("__FerruleClient__", &client),
        ("__FerruleError__", &error),
        ("__ferrule_pkg__", package),
    ];
    let substitute = |text: &str| {
        replacements
            .iter()
            .fold(text.to_owned(), |text, (token, value)| text.replace(token, value))
    };
    let flags = [
        ("py39", true),
        ("webhooks", false),
        (
            "base_model_hint_fits",
            BASE_MODEL_HINT.replace("{pkg}", package).chars().count() <= LINE_LENGTH,
        ),
    ];
    let mut files: Vec<(String, String)> = Vec::new();
    let mut sources: Vec<(String, &str)> = Vec::new();
    for &(rel, template) in PYTHON_TEMPLATE {
        if rel.starts_with(TESTING_DIR) {
            continue;
        }
        let text = substitute(&resolve_directives(template, &flags, rel)?);
        let out = substitute(rel);
        for rendered in [&out, &text] {
            if let Some(token) = leftover_token(rendered) {
                return Err(format!("{rel}: unknown template token {token}"));
            }
        }
        if let Some((_, other)) = sources.iter().find(|(path, _)| *path == out) {
            return Err(format!(
                "{rel} and {other} both render to {out}; choose names that keep the runtime's files apart"
            ));
        }
        sources.push((out.clone(), rel));
        files.push((out, text));
    }
    Ok(files)
}

/// `resolve_directives` without slots or `if fits` (the Python template has neither): whole-line
/// `#ferrule:if <flag>` / `else` / `endif` keep or drop the lines between them.
fn resolve_directives(text: &str, flags: &[(&str, bool)], path: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    // (condition, in the else branch) per open `if`.
    let mut stack: Vec<(bool, bool)> = Vec::new();
    for line in text.split_inclusive('\n') {
        let stripped = line.trim();
        let active = stack.iter().all(|&(cond, in_else)| cond != in_else);
        let Some(directive) = stripped.strip_prefix(MARKER) else {
            if active {
                out.push_str(line);
            }
            continue;
        };
        let (op, name) = match directive.split_once(' ') {
            Some((op, name)) => (op, Some(name)),
            None => (directive, None),
        };
        let well_formed = matches!(op, "if" | "else" | "endif" | "slot")
            && name.is_none_or(|n| {
                !n.is_empty()
                    && n.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            });
        if !well_formed {
            return Err(format!("{path}: malformed directive '{stripped}'"));
        }
        match op {
            "if" => {
                let flag = name.and_then(|n| flags.iter().find(|(f, _)| *f == n));
                let Some(&(_, value)) = flag else {
                    return Err(format!("{path}: unknown template flag '{}'", name.unwrap_or("None")));
                };
                stack.push((value, false));
            }
            "slot" => return Err(format!("{path}: unknown template slot '{}'", name.unwrap_or("None"))),
            _ if stack.is_empty() => return Err(format!("{path}: {MARKER}{op} without {MARKER}if")),
            "else" => {
                if let Some(top) = stack.last_mut() {
                    top.1 = true;
                }
            }
            _ => {
                stack.pop();
            }
        }
    }
    if stack.is_empty() {
        Ok(out)
    } else {
        Err(format!("{path}: unterminated {MARKER}if"))
    }
}

/// The first `__ferrule_x__`, `__FerruleX__` or `__FERRULE_X__` token in `text`, as `_LEFTOVER`
/// finds it.
fn leftover_token(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let token = |start: usize| -> Option<usize> {
        let rest = &bytes[start + 2..];
        let (prefix, body): (&[u8], fn(u8) -> bool) = if rest.starts_with(b"ferrule_") {
            (b"ferrule_", |b| b.is_ascii_lowercase() || b == b'_')
        } else if rest.starts_with(b"FERRULE_") {
            (b"FERRULE_", |b| b.is_ascii_uppercase() || b == b'_')
        } else if rest.starts_with(b"Ferrule") {
            (b"Ferrule", |b| b.is_ascii_alphabetic())
        } else {
            return None;
        };
        let body_len = rest[prefix.len()..].iter().take_while(|&&b| body(b)).count();
        // The regex backtracks: a body ending in `_` can give its last `__` to the closing `__`.
        (1..=body_len).rev().find_map(|n| {
            let end = start + 2 + prefix.len() + n;
            bytes[end..].starts_with(b"__").then_some(end + 2)
        })
    };
    (0..bytes.len().saturating_sub(1))
        .filter(|&i| bytes[i..].starts_with(b"__"))
        .find_map(|i| token(i).map(|end| &text[i..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_python_rules() {
        assert_eq!(default_client_name("acme_sdk"), "AcmeSdk");
        assert_eq!(default_client_name("onebusaway"), "Onebusaway");
        assert_eq!(default_client_name("_a__b_"), "AB");
        assert_eq!(screaming_snake("OnebusawaySDK"), "ONEBUSAWAY_SDK");
        assert_eq!(screaming_snake("ImageKit"), "IMAGE_KIT");
        assert_eq!(screaming_snake("Lumaai2Go"), "LUMAAI2_GO");
        assert!(check_identifier("package_name", "acme_sdk").is_ok());
        assert!(check_identifier("package_name", "class").is_err());
        assert!(check_identifier("package_name", "9lives").is_err());
        assert!(check_identifier("package_name", "acme-sdk").is_err());
    }

    #[test]
    fn resolves_nested_directives() {
        let flags = [("a", true), ("b", false)];
        let text = "x\n#ferrule:if a\n  #ferrule:if b\nno\n  #ferrule:else\nyes\n  #ferrule:endif\n#ferrule:else\nno\n#ferrule:endif\ny";
        assert_eq!(resolve_directives(text, &flags, "t.py"), Ok("x\nyes\ny".to_owned()));
        for (bad, message) in [
            ("#ferrule:if c\n#ferrule:endif\n", "unknown template flag 'c'"),
            ("#ferrule:if\n", "unknown template flag 'None'"),
            ("#ferrule:endif\n", "#ferrule:endif without #ferrule:if"),
            ("#ferrule:if a\n", "unterminated #ferrule:if"),
            ("#ferrule:iff a\n", "malformed directive"),
            ("#ferrule:if A\n", "malformed directive"),
            ("#ferrule:slot x\n", "unknown template slot 'x'"),
        ] {
            let err = resolve_directives(bad, &flags, "t.py").expect_err(bad);
            assert!(err.contains(message), "{bad:?}: {err}");
        }
    }

    #[test]
    fn finds_leftover_tokens() {
        assert_eq!(
            leftover_token("x = __ferrule_pkg_path__ + 1"),
            Some("__ferrule_pkg_path__")
        );
        assert_eq!(leftover_token("class __FerruleThing__:"), Some("__FerruleThing__"));
        assert_eq!(leftover_token("__FERRULE_X__"), Some("__FERRULE_X__"));
        assert_eq!(leftover_token("__ferrule_a___"), Some("__ferrule_a___"));
        assert_eq!(leftover_token("__init__ __ferrule__ __Ferrule__ __FERRULE___"), None);
    }

    #[test]
    fn renders_the_runtime_without_tokens() {
        let files = render("acme_sdk").expect("renders");
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"_base_client.py"), "{paths:?}");
        assert!(paths.contains(&"_utils/_logs.py"), "{paths:?}");
        assert!(
            !paths
                .iter()
                .any(|p| p.starts_with("testing/") || p.ends_with("ruff.toml"))
        );
        let logs = &files.iter().find(|(p, _)| p == "_utils/_logs.py").expect("logs").1;
        assert!(logs.contains("ACME_SDK_LOG"), "{logs}");
        assert!(render("class").is_err());
    }
}
