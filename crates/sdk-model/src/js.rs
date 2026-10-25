//! The JavaScript semantics the extension shapes depend on: the order `Object.entries` lists keys
//! in, UTF-16 string order, and the `\s` class. The same rules as the Python plugin's `js`, which
//! moves here when Python migrates onto this crate (see MIGRATION.md).

use std::cmp::Ordering;

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
    fn compares_strings_by_utf16_units() {
        assert_eq!(cmp("\u{1f600}", "\u{ff5e}"), Ordering::Less);
        assert!(is_space('\u{feff}'));
    }
}
