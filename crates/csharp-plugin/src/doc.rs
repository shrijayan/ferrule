//! XML doc comments from spec descriptions: `/// <summary>` blocks whose first paragraph is
//! plain and whose later paragraphs are `<para>`s, with `&`, `<` and `>` escaped.
//!
//! Lines are filled greedily to a text width that depends on where the comment is ([`Place`]),
//! and a markdown link or a long code span never starts a line (it stays after the word before
//! it). This reproduces most of Stainless's C# docs but not all: its wrapping moves some words
//! between lines in a way no width explains (see `tests/oracle.rs`, "doc wrap").

use std::fmt::Write as _;

/// `&`, `<` and `>` as XML entities.
#[must_use]
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Splits on single spaces, keeping a link's `](url)` with the last word of its text (or, with
/// `whole_links`, the whole `[text](url)` as one token) and the empty tokens of runs of spaces
/// (they become a line's leading spaces after a break).
fn tokens(text: &str, whole_links: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '['
            && whole_links
            && let Some(close) = rest.find("](")
            && !rest[1..close].contains('[')
            && let Some(len) = link_len(&rest[close..])
        {
            cur.push_str(&rest[..close + len]);
            rest = &rest[close + len..];
            continue;
        }
        if c == ']'
            && let Some(len) = link_len(rest)
        {
            cur.push_str(&rest[..len]);
            rest = &rest[len..];
            continue;
        }
        if c == ' ' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
        rest = &rest[c.len_utf8()..];
    }
    out.push(cur);
    out
}

/// The length of the `](url)` of a link at the start of `s`.
fn link_len(s: &str) -> Option<usize> {
    let after = s.strip_prefix("](")?;
    let end = after.find(')')?;
    Some(2 + end + 1)
}

/// Whether a token stays on the line before it however long that makes the line: a link, or a
/// code span of more than 14 characters (the oracles' long code spans never start a line).
fn glued(t: &str) -> bool {
    t.contains("](") || (t.matches('`').count() >= 2 && t.chars().count() > 14) || t == "</para>"
}

/// Fills `text` greedily into lines of at most `width` characters.
fn fill(text: &str, width: usize, glue: bool, whole_links: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line: Option<String> = None;
    for t in tokens(text, whole_links) {
        let Some(cur) = line.as_mut() else {
            line = Some(t);
            continue;
        };
        let fits = cur.chars().count() + 1 + t.chars().count() <= width;
        if fits || (glue && glued(&t)) {
            cur.push(' ');
            cur.push_str(&t);
        } else {
            lines.push(std::mem::replace(cur, t));
        }
    }
    lines.extend(line);
    lines.into_iter().map(|l| l.trim_end().to_owned()).collect()
}

/// The paragraphs of a description: split at blank lines, each joined on one line (a newline
/// becomes a space, other spaces are kept).
fn paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
        if line.trim().is_empty() {
            if !cur.is_empty() {
                out.push(cur.join(" "));
                cur.clear();
            }
        } else {
            cur.push(line);
        }
    }
    if !cur.is_empty() {
        // A description that ends with a newline keeps a trailing space in its last paragraph.
        let mut last = cur.join(" ");
        if text.ends_with('\n') {
            last.push(' ');
        }
        out.push(last);
    }
    out
}

/// Where a doc comment is: its text width (without indentation and `/// `) differs, as fitted
/// to the oracles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// A model's or params class's property.
    Property,
    /// A service method.
    Method,
    /// A params class (from its method's description).
    Params,
    /// A model class.
    Model,
    /// An enum or union declared from a property.
    Variant,
    /// A client option.
    Opt,
}

impl Place {
    /// The text width of the doc lines.
    #[must_use]
    pub fn width(self) -> usize {
        match self {
            Self::Property => 77,
            Self::Method | Self::Params | Self::Model => 80,
            Self::Variant => 82,
            Self::Opt => 75,
        }
    }
}

/// The text lines of a summary (without `/// `): the first paragraph plain, the others as
/// `<para>`s, separated by empty lines. `extra` are paragraphs (already escaped and laid out)
/// appended after the description's, such as the NOTE about inheriting.
#[must_use]
pub fn summary_lines(description: &str, place: Place, extra: &[&str]) -> Vec<String> {
    let width = place.width();
    // Method docs fill plainly, as the other languages' docs do; elsewhere a link's URL or a
    // long code span stays on the line before it.
    let glue = place != Place::Method;
    // A model class keeps a link whole on one line.
    let whole_links = place == Place::Model;
    let mut out: Vec<String> = Vec::new();
    for (i, p) in paragraphs(description).iter().enumerate() {
        let p = escape(p);
        if !out.is_empty() {
            out.push(String::new());
        }
        let text = if i == 0 {
            p.trim().to_owned()
        } else {
            // A later paragraph is tagged, then filled. Params classes drop the trailing space a
            // description's last newline leaves; other docs keep it before `</para>`.
            let space = if p.ends_with(' ') && place != Place::Params {
                " "
            } else {
                ""
            };
            format!("<para>{}{space}</para>", p.trim())
        };
        out.extend(fill(&text, width, glue, whole_links));
    }
    for e in extra {
        if !out.is_empty() {
            out.push(String::new());
        }
        out.extend(e.lines().map(str::to_owned));
    }
    out
}

/// A `/// <summary>` block at `indent` spaces, or nothing for an empty description.
#[must_use]
pub fn summary(description: &str, indent: usize, place: Place, extra: &[&str]) -> String {
    let lines = summary_lines(description, place, extra);
    if lines.is_empty() {
        return String::new();
    }
    block(&lines, indent)
}

/// `lines` as a `/// <summary>` block.
#[must_use]
pub fn block(lines: &[String], indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut out = format!("{pad}/// <summary>\n");
    for l in lines {
        if l.is_empty() {
            let _ = writeln!(out, "{pad}///");
        } else {
            let _ = writeln!(out, "{pad}/// {l}");
        }
    }
    let _ = writeln!(out, "{pad}/// </summary>");
    out
}

/// The NOTE of a params class's summary, as a `<para>`.
pub const NOTE_PARA: &str = "<para>NOTE: Do not inherit from this type outside the SDK unless you're okay with
breaking changes in non-major versions. We may add new methods in the future that
cause existing derived classes to break.</para>";

/// The NOTE of a service interface's summary.
pub const NOTE: &str = "NOTE: Do not inherit from this type outside the SDK unless you're okay with breaking
changes in non-major versions. We may add new methods in the future that cause
existing derived classes to break.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_to_the_width_and_keeps_links_after_the_word_before() {
        let lines = summary_lines(
            "Your ImageKit webhook secret for verifying webhook signatures (starts with `whsec_`).\nYou can find this in the [ImageKit dashboard](https://imagekit.io/dashboard/developer/webhooks).\nOnly required if you're using webhooks.\n",
            Place::Opt,
            &[],
        );
        assert_eq!(
            lines,
            [
                "Your ImageKit webhook secret for verifying webhook signatures (starts with",
                "`whsec_`). You can find this in the [ImageKit dashboard](https://imagekit.io/dashboard/developer/webhooks).",
                "Only required if you're using webhooks.",
            ]
        );
    }

    #[test]
    fn writes_later_paragraphs_as_paras_and_escapes() {
        let lines = summary_lines(
            "Lists files.\n\nA query such as `a > b`.\n",
            Place::Params,
            &[NOTE_PARA],
        );
        assert_eq!(
            lines,
            [
                "Lists files.",
                "",
                "<para>A query such as `a &gt; b`.</para>",
                "",
                "<para>NOTE: Do not inherit from this type outside the SDK unless you're okay with",
                "breaking changes in non-major versions. We may add new methods in the future that",
                "cause existing derived classes to break.</para>",
            ]
        );
        assert_eq!(summary("", 0, Place::Property, &[]), "");
    }

    #[test]
    fn keeps_the_spaces_of_list_items() {
        let lines = summary_lines(
            "Filter results by file type.\n\n- `all` — include all file types  \n- `image` — include only image files  \n- `non-image` — include only non-image files (e.g., JS, CSS, video)\n",
            Place::Property,
            &[],
        );
        assert_eq!(
            lines[2..],
            [
                "<para>- `all` — include all file types   - `image` — include only image files",
                "  - `non-image` — include only non-image files (e.g., JS, CSS, video) </para>",
            ]
        );
    }
}
