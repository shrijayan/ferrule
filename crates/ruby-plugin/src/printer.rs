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

/// Greedy word wrap of `words` at `width`, the first line after `first`, the others after
/// `rest`.
fn fill(words: &[&str], width: usize, first: &str, rest: &str, out: &mut Vec<String>) {
    let mut cur = first.to_owned();
    let mut empty = true;
    for word in words {
        if empty {
            cur.push_str(word);
            empty = false;
        } else if len(&cur) + 1 + len(word) <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(std::mem::replace(&mut cur, format!("{rest}{word}")));
        }
    }
    out.push(cur);
}

/// The words of a line of markdown: whitespace-separated, but a link (`[text](url)`) or a code
/// span (`` `a b` ``) is one word, as prettier never breaks them.
fn tokens(line: &str) -> Vec<&str> {
    let is_space = ferrule_sdk_model::js::is_space;
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let (mut bracket, mut paren, mut code) = (0_i32, 0_i32, false);
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        if c == '`' {
            code = !code;
        } else if !code {
            match c {
                '[' => bracket += 1,
                ']' => bracket -= 1,
                '(' if prev == ']' || paren > 0 => paren += 1,
                ')' if paren > 0 => paren -= 1,
                _ => {}
            }
        }
        let inside = code || bracket > 0 || paren > 0;
        if is_space(c) && !inside {
            if let Some(s) = start.take() {
                out.push(&line[s..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
        prev = c;
    }
    if let Some(s) = start {
        out.push(&line[s..]);
    }
    out
}

/// The marker of a markdown list item (`- `, `* `, `1. `), if the line starts one.
fn list_marker(line: &str) -> Option<&str> {
    for m in ["- ", "* ", "+ "] {
        if line.starts_with(m) {
            return Some(&line[..2]);
        }
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    (digits > 0 && line[digits..].starts_with(". ")).then(|| &line[..digits + 2])
}

/// Description text as Stainless's comments lay it out: markdown reflowed at `width` (lines of a
/// paragraph joined, each list item on its own lines with a hanging indent, fenced code as
/// written), paragraphs separated by an empty line; leading and trailing blank lines dropped.
#[must_use]
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let is_space = ferrule_sdk_model::js::is_space;
    let mut out: Vec<String> = Vec::new();
    // The words of the paragraph or list item being collected, and its marker.
    let mut words: Vec<&str> = Vec::new();
    let mut marker: Option<&str> = None;
    let mut fenced = false;
    let flush = |words: &mut Vec<&str>, marker: &mut Option<&str>, out: &mut Vec<String>| {
        if !words.is_empty() {
            match marker.take() {
                Some(m) => fill(words, width, m, &" ".repeat(len(m)), out),
                None => fill(words, width, "", "", out),
            }
            words.clear();
        }
    };
    for raw in text.split('\n') {
        let line = raw.trim_end_matches(is_space);
        if fenced {
            out.push(line.to_owned());
            fenced = !line.trim_start().starts_with("```");
            continue;
        }
        let trimmed = line.trim_start_matches(is_space);
        if trimmed.is_empty() {
            flush(&mut words, &mut marker, &mut out);
            if out.last().is_some_and(|l| !l.is_empty()) {
                out.push(String::new());
            }
            continue;
        }
        if trimmed.starts_with("```") {
            flush(&mut words, &mut marker, &mut out);
            out.push(trimmed.to_owned());
            fenced = true;
            continue;
        }
        if let Some(m) = list_marker(trimmed) {
            // A list right after a paragraph is its own block.
            if marker.is_none() && !words.is_empty() {
                flush(&mut words, &mut marker, &mut out);
                out.push(String::new());
            }
            flush(&mut words, &mut marker, &mut out);
            marker = Some(m);
            words.extend(tokens(&trimmed[m.len()..]));
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with('|') {
            flush(&mut words, &mut marker, &mut out);
            out.push(trimmed.to_owned());
            continue;
        }
        words.extend(tokens(trimmed));
    }
    flush(&mut words, &mut marker, &mut out);
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
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
        assert_eq!(wrap("one\ntwo", 80), ["one two"]);
        assert_eq!(wrap("see [a b](c) d", 6), ["see", "[a b](c)", "d"]);
        assert_eq!(wrap("intro:\n- a", 80), ["intro:", "", "- a"]);
        assert_eq!(
            wrap("Intro.\n\n- `a` x y  \n- `b` z", 8),
            ["Intro.", "", "- `a` x", "  y", "- `b` z"]
        );
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
