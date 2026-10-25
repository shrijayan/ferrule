//! JavaScript semantics the TypeScript plugin's output depends on, so the port prints the same
//! bytes: string lengths and order in UTF-16 code units, `\s` and `trim`, `String(number)`, and
//! the order `Object.entries` lists keys in.

use std::cmp::Ordering;

/// `s.length`: the length in UTF-16 code units.
#[must_use]
pub fn len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `a < b ? -1 : a > b ? 1 : 0` on strings: UTF-16 code unit order.
#[must_use]
pub fn cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// A character of the regular expression class `\s`.
#[must_use]
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// `s.trim()`.
#[must_use]
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
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
    // JavaScript's `n`: the value is 0.digits * 10^n.
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

/// The array index a key stands for, as JavaScript objects order such keys first.
fn array_index(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    let canonical = key == "0" || (matches!(bytes.first(), Some(b'1'..=b'9')) && bytes.iter().all(u8::is_ascii_digit));
    if !canonical {
        return None;
    }
    key.parse::<u32>().ok().filter(|&n| n < u32::MAX)
}

/// `Object.entries` of an object whose members are in written order: integer-like keys first,
/// ascending, then the rest in written order.
#[must_use]
pub fn entries<V>(members: &[(String, V)]) -> Vec<(&str, &V)> {
    let mut indexed: Vec<(u32, &str, &V)> = Vec::new();
    let mut rest: Vec<(&str, &V)> = Vec::new();
    for (k, v) in members {
        match array_index(k) {
            Some(i) => indexed.push((i, k, v)),
            None => rest.push((k, v)),
        }
    }
    indexed.sort_by_key(|&(i, _, _)| i);
    indexed.into_iter().map(|(_, k, v)| (k, v)).chain(rest).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_numbers_like_javascript() {
        let cases: [(f64, &str); 12] = [
            (1.0, "1"),
            (-3.0, "-3"),
            (0.5, "0.5"),
            (1.5e-7, "1.5e-7"),
            (1e21, "1e+21"),
            (1.25e21, "1.25e+21"),
            (123_456_789_012.0, "123456789012"),
            (0.000_001, "0.000001"),
            (100.0, "100"),
            (-0.0, "0"),
            (12.34, "12.34"),
            (f64::INFINITY, "Infinity"),
        ];
        for (n, s) in cases {
            assert_eq!(number_to_string(n), s, "{n}");
        }
    }

    #[test]
    fn orders_integer_keys_first() {
        let members = vec![
            ("b".to_owned(), 1),
            ("10".to_owned(), 2),
            ("2".to_owned(), 3),
            ("01".to_owned(), 4),
        ];
        let keys: Vec<&str> = entries(&members).into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys, ["2", "10", "b", "01"]);
    }

    #[test]
    fn measures_and_orders_in_utf16() {
        assert_eq!(len("a\u{1f600}"), 3);
        assert_eq!(cmp("\u{1f600}", "\u{ff61}"), Ordering::Less);
        assert_eq!(trim("\u{feff} x\u{a0}"), "x");
    }
}
