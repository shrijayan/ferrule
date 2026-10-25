//! Unicode general categories as JavaScript's `\p{...}` regex escapes test them.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CLASSES: RefCell<HashMap<&'static str, regress::Regex>> = RefCell::new(HashMap::new());
}

fn test(class: &'static str, c: char) -> bool {
    CLASSES.with(|cache| {
        let mut cache = cache.borrow_mut();
        let re = cache.entry(class).or_insert_with(|| {
            regress::Regex::with_flags(&format!("^{class}$"), "u").expect("the category classes are valid regexes")
        });
        let mut buf = [0u8; 4];
        re.find(c.encode_utf8(&mut buf)).is_some()
    })
}

/// `\p{L}` or `\p{N}`.
#[must_use]
pub fn is_letter_or_number(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphanumeric();
    }
    test(r"[\p{L}\p{N}]", c)
}

/// `\p{Lu}`.
#[must_use]
pub fn is_upper(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_uppercase();
    }
    test(r"\p{Lu}", c)
}

/// `\p{Ll}`.
#[must_use]
pub fn is_lower(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_lowercase();
    }
    test(r"\p{Ll}", c)
}

/// `\p{Ll}` or `\p{N}`.
#[must_use]
pub fn is_lower_or_number(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_lowercase() || c.is_ascii_digit();
    }
    test(r"[\p{Ll}\p{N}]", c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_like_javascript() {
        assert!(is_letter_or_number('é'));
        assert!(is_letter_or_number('٣'));
        assert!(!is_letter_or_number('\u{301}'));
        assert!(!is_letter_or_number('-'));
        assert!(is_upper('É'));
        assert!(!is_upper('ǅ'));
        assert!(is_lower('ß'));
        assert!(is_lower_or_number('Ⅻ'));
    }
}
