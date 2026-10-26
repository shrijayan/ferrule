//! `JSDoc` comments as Stainless writes them: the description formatted as markdown the way
//! prettier formats it with `proseWrap: "always"` at 80 columns (paragraphs re-filled, list
//! items continued two spaces in, fenced code kept), then each line behind ` * `.

use crate::doc::width;
use crate::js;

/// The markdown width of a description.
pub const PROSE_WIDTH: usize = 80;

/// Where an inline construct prettier never breaks ends: a link `[text](url)` or code
/// `` `code` `` starting at `chars[i]`, or `None`.
fn atom_end(chars: &[char], i: usize) -> Option<usize> {
    match chars[i] {
        '`' => chars[i + 1..].iter().position(|&c| c == '`').map(|p| i + 1 + p),
        '[' => {
            let mut depth = 0;
            let mut j = i;
            while j < chars.len() {
                match chars[j] {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if chars.get(j + 1) != Some(&'(') {
                return None;
            }
            chars[j + 2..].iter().position(|&c| c == ')').map(|p| j + 2 + p)
        }
        _ => None,
    }
}

/// The words of a paragraph: split at whitespace, links and inline code kept whole.
fn tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if js::is_space(c) {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            i += 1;
        } else if let Some(end) = atom_end(&chars, i) {
            for &ch in &chars[i..=end] {
                cur.push(if js::is_space(ch) { ' ' } else { ch });
            }
            i = end + 1;
        } else {
            cur.push(c);
            i += 1;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    // Prettier never breaks before a word that would start a list item, heading or quote
    // (`^>|^(?:[*+-]|#{1,6}|\d+[).])$`): it stays with the word before it.
    let mut glued: Vec<String> = Vec::with_capacity(out.len());
    for w in out {
        match glued.last_mut() {
            Some(prev) if marker_like(&w) => {
                prev.push(' ');
                prev.push_str(&w);
            }
            _ => glued.push(w),
        }
    }
    glued
}

fn marker_like(w: &str) -> bool {
    if w.starts_with('>') || matches!(w, "*" | "+" | "-") {
        return true;
    }
    if !w.is_empty() && w.len() <= 6 && w.bytes().all(|b| b == b'#') {
        return true;
    }
    let digits = w.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && digits + 1 == w.len() && matches!(w.as_bytes()[digits], b'.' | b')')
}

/// Greedy fill of `words` at `limit` columns, the first line `first` columns narrower.
fn fill(words: &[String], limit: usize, first: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for w in words {
        let room = if lines.is_empty() {
            limit.saturating_sub(first)
        } else {
            limit
        };
        if cur.is_empty() {
            cur.clone_from(w);
        } else if width(&cur) + 1 + width(w) <= room {
            cur.push(' ');
            cur.push_str(w);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.clone_from(w);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// A list item marker at the start of a trimmed line: `- `, `* `, `+ ` or `1. `.
fn list_marker(line: &str) -> Option<(String, &str)> {
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(m) {
            return Some(("- ".to_owned(), rest));
        }
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0
        && let Some(rest) = line[digits..].strip_prefix(". ")
    {
        return Some((format!("{}. ", &line[..digits]), rest));
    }
    None
}

/// Markdown text re-filled at `limit` columns.
#[must_use]
pub fn format_markdown(text: &str, limit: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let blank = |out: &mut Vec<String>| {
        if out.last().is_some_and(|l| !l.is_empty()) {
            out.push(String::new());
        }
    };
    while i < lines.len() {
        let raw = lines[i];
        let trimmed = js_trim(raw);
        if trimmed.is_empty() {
            blank(&mut out);
            i += 1;
            continue;
        }
        if trimmed.starts_with("```") {
            let mut j = i + 1;
            out.push(trimmed.to_owned());
            while j < lines.len() && !js_trim(lines[j]).starts_with("```") {
                out.push(lines[j].trim_end().to_owned());
                j += 1;
            }
            if j < lines.len() {
                out.push(js_trim(lines[j]).to_owned());
            }
            i = j + 1;
            continue;
        }
        if let Some((marker, rest)) = list_marker(trimmed) {
            // One list: items until a blank line; a line that is not an item continues one.
            let mut items: Vec<(String, String)> = vec![(marker, rest.to_owned())];
            let mut j = i + 1;
            while j < lines.len() && !js_trim(lines[j]).is_empty() {
                let t = js_trim(lines[j]);
                if let Some((m, r)) = list_marker(t) {
                    items.push((m, r.to_owned()));
                } else if let Some(last) = items.last_mut() {
                    last.1.push(' ');
                    last.1.push_str(t);
                }
                j += 1;
            }
            for (marker, item) in items {
                let pad = " ".repeat(marker.len());
                for (k, l) in fill(&tokens(&item), limit.saturating_sub(marker.len()), 0)
                    .into_iter()
                    .enumerate()
                {
                    out.push(if k == 0 {
                        format!("{marker}{l}")
                    } else {
                        format!("{pad}{l}")
                    });
                }
            }
            i = j;
            continue;
        }
        let mut para = String::new();
        let mut j = i;
        while j < lines.len() {
            let t = js_trim(lines[j]);
            if t.is_empty() || t.starts_with("```") || (j > i && list_marker(t).is_some()) {
                break;
            }
            para.push(' ');
            para.push_str(t);
            j += 1;
        }
        out.extend(fill(&tokens(&para), limit, 0));
        i = j;
    }
    while out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js::is_space)
}

/// Makes text safe inside `/** */`.
fn escape(text: &str) -> String {
    text.replace("*/", "*\\/")
}

/// The lines of a `JSDoc` block at `indent` spaces: the description, then each tag section
/// (`@example ...`, `@deprecated`), separated by blank comment lines.
#[must_use]
pub fn jsdoc(description: &str, sections: &[Vec<String>], indent: usize) -> Vec<String> {
    let pad = " ".repeat(indent);
    let mut body: Vec<String> = format_markdown(&escape(description), PROSE_WIDTH);
    for s in sections {
        if !body.is_empty() {
            body.push(String::new());
        }
        body.extend(s.iter().cloned());
    }
    if body.is_empty() {
        return Vec::new();
    }
    let mut out = vec![format!("{pad}/**")];
    for l in body {
        if l.is_empty() {
            out.push(format!("{pad} *"));
        } else {
            out.push(format!("{pad} * {l}"));
        }
    }
    out.push(format!("{pad} */"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refills_paragraphs_at_80_columns() {
        let text =
            "The text or array of texts to embed.\n\nEach text must contain at least one non-whitespace character.";
        assert_eq!(
            format_markdown(text, 40),
            [
                "The text or array of texts to embed.",
                "",
                "Each text must contain at least one",
                "non-whitespace character."
            ]
        );
    }

    #[test]
    fn keeps_links_and_code_whole() {
        assert_eq!(
            format_markdown(
                "Whether the query is an [Isaacus Query Language (IQL)](https://x.y/iql) query.",
                30
            ),
            [
                "Whether the query is an",
                "[Isaacus Query Language (IQL)](https://x.y/iql)",
                "query."
            ]
        );
        assert_eq!(format_markdown("[42] The `a b` c", 8), ["[42] The", "`a b` c"]);
        assert_eq!(
            format_markdown("such as Section 2. Definitions", 15),
            ["such as", "Section 2.", "Definitions"]
        );
    }

    #[test]
    fn continues_list_items_two_columns_in() {
        let text = "Type:\n\n- `a`: one two three four\n- `b`: five";
        assert_eq!(
            format_markdown(text, 16),
            ["Type:", "", "- `a`: one two", "  three four", "- `b`: five"]
        );
    }

    #[test]
    fn writes_blocks_with_sections() {
        assert_eq!(
            jsdoc("Hi */ there.", &[vec!["@deprecated".to_owned()]], 2),
            ["  /**", "   * Hi *\\/ there.", "   *", "   * @deprecated", "   */"]
        );
        assert_eq!(jsdoc("", &[], 0), Vec::<String>::new());
    }
}
