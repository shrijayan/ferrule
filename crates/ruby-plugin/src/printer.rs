//! Text helpers shared by the views: Stainless's comment wrap, Ruby literals, and the layout of
//! a `lib/**.rb` statement (see [`crate::doc`]).
//!
//! The output is what Stainless's generator writes before the gem's `rake format`: `.rb` files
//! laid out as the generator lays them out (110 columns, see [`crate::doc::print_rb`]), which
//! `rubocop -a` then aligns and breaks further with the generated `.rubocop.yml`; `.rbi` and
//! `.rbs` files one declaration per line, which `stree write` lays out (80 columns). Both
//! formatters run in `ferrule generate` and in the oracle tests, at the versions the oracles'
//! `Gemfile.lock` pins (`crates/ruby-plugin/tools/Gemfile`).

use std::fmt::Write as _;

use ferrule_sdk_model::model::Literal;

/// The width Stainless wraps description text to, not counting the comment's indent and `# `.
pub const COMMENT_WIDTH: usize = 80;

/// The line length of `lib/**.rb` (`.rubocop.yml`'s `Layout/LineLength`).
pub const RB_WIDTH: usize = 110;

/// A template copied from an oracle, without its first line (the `Modified by` mark NOTICE
/// asks for, which is not emitted).
#[must_use]
pub fn template(text: &str) -> &str {
    text.split_once('\n').map_or("", |(_, rest)| rest)
}

/// UTF-16 length, as Stainless's generator measures text.
#[must_use]
pub fn len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Greedy word wrap of each paragraph of `text` at `width`; trailing blank lines dropped.
#[must_use]
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in para.split(ferrule_sdk_model::js::is_space).filter(|w| !w.is_empty()) {
            if cur.is_empty() {
                word.clone_into(&mut cur);
            } else if len(&cur) + 1 + len(word) <= width {
                cur.push(' ');
                cur.push_str(word);
            } else {
                lines.push(std::mem::replace(&mut cur, word.to_owned()));
            }
        }
        lines.push(cur);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    lines
}

/// Comment lines `# <line>` at `indent` (an empty line is `#`), each ending in a newline.
pub fn comment(out: &mut String, indent: usize, lines: &[String]) {
    for l in lines {
        let pad = " ".repeat(indent);
        if l.is_empty() {
            let _ = writeln!(out, "{pad}#");
        } else {
            let _ = writeln!(out, "{pad}# {l}");
        }
    }
}

/// A Ruby double-quoted string literal.
#[must_use]
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '#' => out.push('#'),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    // `#{` and `#@` would interpolate.
    out.replace("#{", "\\#{").replace("#@", "\\#@").replace("#$", "\\#$")
}

/// A symbol literal: `:other`, or `:"240"` when the value is not a plain identifier.
#[must_use]
pub fn symbol(s: &str) -> String {
    let plain = s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if plain {
        format!(":{s}")
    } else {
        format!(":{}", quote(s))
    }
}

/// A number as Ruby writes it: `60.0`, `2`.
#[must_use]
pub fn float(n: f64) -> String {
    let s = format!("{n}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

/// An enum member's value: a symbol for a string, else the number or boolean.
#[must_use]
pub fn literal(v: &Literal) -> String {
    match v {
        Literal::String(s) => symbol(s),
        Literal::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{n:.0}")
            } else {
                float(*n)
            }
        }
        Literal::Boolean(b) => b.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_the_width() {
        assert_eq!(wrap("a b c", 3), ["a b", "c"]);
        assert_eq!(wrap("a\n\nb\n", 80), ["a", "", "b"]);
    }

    #[test]
    fn writes_literals() {
        assert_eq!(quote("a\"b#{c}"), "\"a\\\"b\\#{c}\"");
        assert_eq!(symbol("other"), ":other");
        assert_eq!(symbol("240"), ":\"240\"");
        assert_eq!(float(60.0), "60.0");
        assert_eq!(float(0.5), "0.5");
    }
}
