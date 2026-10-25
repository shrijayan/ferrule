//! JavaScript semantics the TypeScript emitters relied on implicitly: string length and `<` count
//! UTF-16 code units, and numbers print the way `String(n)` prints them.
//!
//! TODO(merge): use `emit::common` versions of these once `common.rs` has them.

use std::cmp::Ordering;

/// `a.length` of a JavaScript string: UTF-16 code units.
pub(crate) fn len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// The order of JavaScript's `<` on strings: UTF-16 code units, not code points.
pub(crate) fn cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// JavaScript's `\s`: `WhiteSpace` and `LineTerminator` of ECMA-262 (which is not Rust's
/// `char::is_whitespace`: it has U+FEFF and lacks U+0085).
pub(crate) fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// A line terminator as JavaScript's `^`, `$` (with the `m` flag) and `.` see it.
pub(crate) fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `String(n)` for a JavaScript number (ECMA-262 `Number::toString`, radix 10).
pub(crate) fn number_to_string(n: f64) -> String {
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
    // `{:e}` gives the shortest digits that round-trip, as JavaScript does: `d.ddde±x`.
    let sci = format!("{:e}", n.abs());
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    // The value is 0.digits × 10^point.
    let point = exponent.parse::<i64>().unwrap_or(0) + 1;
    let zeros = |count: i64| "0".repeat(usize::try_from(count).unwrap_or(0));
    let body = if k <= point && point <= 21 {
        format!("{digits}{}", zeros(point - k))
    } else if 0 < point && point <= 21 {
        let at = usize::try_from(point).unwrap_or(0);
        format!("{}.{}", &digits[..at], &digits[at..])
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", zeros(-point))
    } else {
        let e = point - 1;
        let e = if e < 0 { format!("-{}", -e) } else { format!("+{e}") };
        if k == 1 {
            format!("{digits}e{e}")
        } else {
            format!("{}.{}e{e}", &digits[..1], &digits[1..])
        }
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::{cmp, len, number_to_string};
    use std::cmp::Ordering;

    #[test]
    fn numbers_print_like_javascript() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-122.305_641, "-122.305641"),
            (0.01, "0.01"),
            (1_710_918_000_000.0, "1710918000000"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (123_456_789_012_345_680_000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (-2.5e-7, "-2.5e-7"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
        ];
        for (n, want) in cases {
            assert_eq!(number_to_string(*n), *want, "{n}");
        }
    }

    #[test]
    fn strings_compare_by_utf16() {
        // U+1F600 sorts after U+FF5E by code point, before it by UTF-16 unit (0xD83D < 0xFF5E).
        assert_eq!("\u{1f600}".cmp("\u{ff5e}"), Ordering::Greater);
        assert_eq!(cmp("\u{1f600}", "\u{ff5e}"), Ordering::Less);
        assert_eq!(len("\u{1f600}"), 2);
    }
}
