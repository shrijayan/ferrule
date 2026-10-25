//! JavaScript semantics the output depends on: `String(number)`, `JSON.stringify` of a string,
//! identifier syntax, and words as `\s+` splits them.

use std::fmt::Write as _;

/// A character of the regular expression class `\s`.
#[must_use]
pub fn is_space(c: char) -> bool {
    ferrule_sdk_model::js::is_space(c)
}

/// `s.split(/\s+/).filter((w) => w.length > 0)`.
pub fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split(is_space).filter(|w| !w.is_empty())
}

/// `String(n)` for a number.
#[must_use]
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_owned();
    }
    if n == 0.0 {
        return "0".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let sign = if n < 0.0 { "-" } else { "" };
    // Rust's `{:e}` prints the shortest digits that read back as `n`, as JavaScript does.
    let exp_form = format!("{:e}", n.abs());
    let (mantissa, exp) = exp_form.split_once('e').unwrap_or((&exp_form, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let point = exp.parse::<i64>().unwrap_or(0) + 1;
    let body = if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat(usize::try_from(point - k).unwrap_or(0)))
    } else if 0 < point && point <= 21 {
        let at = usize::try_from(point).unwrap_or(0);
        format!("{}.{}", &digits[..at], &digits[at..])
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat(usize::try_from(-point).unwrap_or(0)))
    } else {
        let e = point - 1;
        let e_sign = if e < 0 { '-' } else { '+' };
        let (first, rest) = digits.split_at(1);
        if rest.is_empty() {
            format!("{first}e{e_sign}{}", e.abs())
        } else {
            format!("{first}.{rest}e{e_sign}{}", e.abs())
        }
    };
    format!("{sign}{body}")
}

/// A JSON number as JavaScript prints it.
#[must_use]
pub fn json_number(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    number_to_string(n.as_f64().unwrap_or(f64::NAN))
}

/// The body of `JSON.stringify(s)`, without the quotes.
#[must_use]
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

/// A string literal as prettier prints `JSON.stringify(s)` with `singleQuote: true`: single
/// quotes unless the text has more single than double quotes; only the enclosing quote escaped.
#[must_use]
pub fn string_literal(s: &str) -> String {
    let raw = json_escape(s);
    let singles = s.matches('\'').count();
    let doubles = s.matches('"').count();
    let quote = if singles > doubles { '"' } else { '\'' };
    make_string(&raw, quote)
}

/// Prettier's `makeString(rawText, enclosingQuote)`.
fn make_string(raw: &str, quote: char) -> String {
    let other = if quote == '"' { '\'' } else { '"' };
    let mut out = String::with_capacity(raw.len() + 2);
    out.push(quote);
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let Some(escaped) = chars.next() else {
                out.push('\\');
                break;
            };
            if escaped == other {
                out.push(escaped);
            } else if escaped == quote || "\n\r\"'01234567\\bfnrtuvx\u{2028}\u{2029}".contains(escaped) {
                out.push('\\');
                out.push(escaped);
            } else {
                out.push(escaped);
            }
        } else if c == quote {
            out.push('\\');
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out.push(quote);
    out
}

/// An ES5 identifier name of ASCII letters, digits, `_` and `$`.
#[must_use]
pub fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// An object key as written: bare when it is an identifier, else a string literal.
#[must_use]
pub fn property_key(key: &str) -> String {
    if is_identifier(key) {
        key.to_owned()
    } else {
        string_literal(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_numbers_like_javascript() {
        assert_eq!(number_to_string(0.1), "0.1");
        assert_eq!(number_to_string(1792.0), "1792");
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(1.5e-7), "1.5e-7");
    }

    #[test]
    fn quotes_strings_as_prettier_does() {
        assert_eq!(string_literal("a"), "'a'");
        assert_eq!(string_literal("say \"hi\""), "'say \"hi\"'");
        assert_eq!(string_literal("it's"), "\"it's\"");
        assert_eq!(string_literal("it's \"x\" 'y'"), "\"it's \\\"x\\\" 'y'\"");
        assert_eq!(string_literal("a\nb\\c"), "'a\\nb\\\\c'");
    }

    #[test]
    fn quotes_keys_only_when_needed() {
        assert_eq!(property_key("overlap_ratio"), "overlap_ratio");
        assert_eq!(property_key("x-api-key"), "'x-api-key'");
        assert_eq!(property_key("1a"), "'1a'");
    }
}
