//! Doc comments from the spec's descriptions, made safe for rustdoc and clippy.
//!
//! Spec descriptions are Markdown written for other tools. In a Rust doc comment an indented
//! block or an unlabelled code fence is a doctest, and a list item's unindented continuation is
//! a `doc_lazy_continuation` warning, so lines lose their leading whitespace, fences (of backticks
//! or tildes) become `text` blocks and list continuations are indented under their item. Control
//! characters become spaces, so no text can end the comment's line.

use std::fmt::Write as _;

/// `text` as `///` lines at `indent`; nothing for blank text.
#[must_use]
pub fn doc(text: &str, indent: &str) -> String {
    let mut out = String::new();
    for line in lines(text) {
        if line.is_empty() {
            let _ = writeln!(out, "{indent}///");
        } else {
            let _ = writeln!(out, "{indent}/// {line}");
        }
    }
    out
}

/// The doc lines of `text`, trimmed of leading and trailing blank lines.
fn lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_fence = false;
    // The indent a list item's continuation lines take, while in one.
    let mut item_indent: Option<usize> = None;
    for raw in text.lines() {
        let raw = inline(raw);
        let line = raw.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            item_indent = None;
            out.push(if in_fence {
                "```".to_owned()
            } else {
                "```text".to_owned()
            });
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push(raw.trim_end().to_owned());
            continue;
        }
        if line.is_empty() {
            item_indent = None;
            out.push(String::new());
        } else if let Some(width) = list_marker(line) {
            item_indent = Some(width);
            out.push(line.to_owned());
        } else if let Some(width) = item_indent {
            out.push(format!("{}{line}", " ".repeat(width)));
        } else if line.starts_with('#') {
            // A heading inside an item's docs reads as a heading of the whole page.
            out.push(line.trim_start_matches('#').trim_start().to_owned());
        } else {
            out.push(line.to_owned());
        }
    }
    if in_fence {
        out.push("```".to_owned());
    }
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    let first = out.iter().position(|l| !l.is_empty()).unwrap_or(out.len());
    out.drain(..first);
    out
}

/// `text` on one line: every control character (a newline among them) a space. For names and
/// URLs written into a one-line doc comment.
#[must_use]
pub fn inline(text: &str) -> String {
    text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect()
}

/// The width of a list item's marker with its space (`- `, `1. `), when `line` starts one.
fn list_marker(line: &str) -> Option<usize> {
    if ["- ", "* ", "+ "].iter().any(|m| line.starts_with(m)) {
        return Some(2);
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let rest = &line[digits..];
    (digits > 0 && (rest.starts_with(". ") || rest.starts_with(") "))).then_some(digits + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_doc_lines() {
        assert_eq!(
            doc("One.\n\n  Two.  \n", "    "),
            "    /// One.\n    ///\n    /// Two.\n"
        );
        assert_eq!(doc("  \n", ""), "");
    }

    #[test]
    fn makes_code_blocks_text() {
        assert_eq!(lines("a\n```json\n  {}\n```"), ["a", "```text", "  {}", "```"]);
        assert_eq!(lines("```\nx"), ["```text", "x", "```"]);
        assert_eq!(lines("    indented"), ["indented"]);
    }

    #[test]
    fn keeps_text_inside_the_comment() {
        assert_eq!(doc("a\rpub fn x() {}", ""), "/// a pub fn x() {}\n");
        assert_eq!(inline("x\n}\nfn y"), "x } fn y");
        assert_eq!(lines("~~~\nfn main() {}\n~~~"), ["```text", "fn main() {}", "```"]);
    }

    #[test]
    fn indents_list_continuations() {
        assert_eq!(
            lines("- one\ncontinued\n\nafter"),
            ["- one", "  continued", "", "after"]
        );
        assert_eq!(lines("10. ten\nmore"), ["10. ten", "    more"]);
        assert_eq!(lines("## Title"), ["Title"]);
    }
}
