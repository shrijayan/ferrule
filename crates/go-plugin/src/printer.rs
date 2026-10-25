//! The Go printer: turns emitter text into gofmt's layout, so the output is gofmt-clean without
//! running gofmt (emitters stay pure; ADR 0017).
//!
//! Emitters write one Go line per text line, indented with tabs, and separate the cells gofmt
//! aligns (a struct field's name, type and tag; a keyed element's key and value; a constant's
//! name, type and value) with [`ALIGN`]. [`align`] then pads them as gofmt's `text/tabwriter`
//! does: a column block is a run of adjacent lines that all have a cell in that column, padded
//! to its widest cell plus one space. Comment lines and blank lines have no cells, so they end a
//! block, as in gofmt.
//!
//! Doc comments are wrapped as Stainless wraps them: greedily at 80 characters of comment text,
//! a word longer than that on a line of its own.

use std::fmt::Write as _;

/// The cell separator gofmt's printer writes between aligned parts (`\v`, a "soft" tab).
pub const ALIGN: char = '\u{b}';

/// Comment text wraps at this many characters, not counting indentation or `// `.
pub const COMMENT_WIDTH: usize = 80;

/// The header every generated file starts with (branding, as for Python: BRANDING.md).
pub const HEADER: &str = "// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.";

/// One cell: its text and whether a hard tab (indentation) ended it.
struct Cell<'a> {
    text: &'a str,
    width: usize,
    hard: bool,
}

/// Splits a line into tabwriter cells: leading tabs are empty hard cells, [`ALIGN`] ends a soft
/// cell, and the rest is the trailing text (not part of any column).
fn cells(line: &str) -> Vec<Cell<'_>> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(r) = rest.strip_prefix('\t') {
        out.push(Cell {
            text: "",
            width: 0,
            hard: true,
        });
        rest = r;
    }
    for part in rest.split(ALIGN) {
        out.push(Cell {
            text: part,
            width: part.chars().count(),
            hard: false,
        });
    }
    out
}

/// Width of each column, computed as tabwriter's `format` does, written into `widths[line][col]`.
fn format(
    lines: &[Vec<Cell<'_>>],
    first: usize,
    end: usize,
    column: usize,
    stack: &mut Vec<usize>,
    out: &mut [Vec<usize>],
) {
    let mut this = first;
    while this < end {
        if column + 1 >= lines[this].len() {
            this += 1;
            continue;
        }
        let start = this;
        let mut width = 0;
        let mut discardable = true;
        while this < end && column + 1 < lines[this].len() {
            let c = &lines[this][column];
            width = width.max(c.width + 1);
            if c.width > 0 || c.hard {
                discardable = false;
            }
            this += 1;
        }
        if discardable {
            width = 0;
        }
        for row in &mut out[start..this] {
            row[column] = width;
        }
        stack.push(width);
        format(lines, start, this, column + 1, stack, out);
        stack.pop();
    }
}

/// Pads [`ALIGN`]-separated cells as gofmt does (tab width 8, padding 1, spaces for alignment,
/// tabs for indentation).
#[must_use]
pub fn align(text: &str) -> String {
    let raw: Vec<&str> = text.split('\n').collect();
    let lines: Vec<Vec<Cell<'_>>> = raw.iter().map(|l| cells(l)).collect();
    let mut widths: Vec<Vec<usize>> = lines.iter().map(|l| vec![0; l.len()]).collect();
    format(&lines, 0, lines.len(), 0, &mut Vec::new(), &mut widths);
    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let last = line.len() - 1;
        let mut pending = String::new();
        for (j, c) in line.iter().enumerate() {
            if j == last {
                if !c.text.is_empty() {
                    out.push_str(&pending);
                    out.push_str(c.text);
                }
                break;
            }
            if c.hard {
                // Indentation: tabwriter pads an empty hard cell with one tab.
                out.push('\t');
                continue;
            }
            pending.push_str(c.text);
            let pad = widths[i][j].saturating_sub(c.width);
            pending.extend(std::iter::repeat_n(' ', pad));
            out.push_str(&pending);
            pending.clear();
        }
    }
    out
}

/// Greedy word wrap of one paragraph at [`COMMENT_WIDTH`].
fn wrap_paragraph(text: &str, out: &mut Vec<String>) {
    let mut line = String::new();
    let mut len = 0;
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        let wlen = word.chars().count();
        if len > 0 && len + 1 + wlen > COMMENT_WIDTH {
            out.push(std::mem::take(&mut line));
            len = 0;
        }
        if len > 0 {
            line.push(' ');
            len += 1;
        }
        line.push_str(word);
        len += wlen;
    }
    if len > 0 {
        out.push(line);
    }
}

/// The lines of a doc comment for `text`: paragraphs (split at blank lines) wrapped at
/// [`COMMENT_WIDTH`], a blank comment line between them.
#[must_use]
pub fn comment_lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, paragraph) in text.trim().split("\n\n").enumerate() {
        let joined = paragraph.split('\n').map(str::trim).collect::<Vec<_>>().join(" ");
        if joined.is_empty() {
            continue;
        }
        if i > 0 && !out.is_empty() {
            out.push(String::new());
        }
        wrap_paragraph(&joined, &mut out);
    }
    out
}

/// Appends `text` as a doc comment at `indent` tabs; nothing for empty text.
pub fn comment(out: &mut String, indent: usize, text: &str) {
    for line in comment_lines(text) {
        out.extend(std::iter::repeat_n('\t', indent));
        if line.is_empty() {
            out.push_str("//\n");
        } else {
            out.push_str("// ");
            out.push_str(&line);
            out.push('\n');
        }
    }
}

/// A Go string literal for `s`, as `strconv.Quote` writes the printable ASCII cases.
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
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A Go file: header, package clause, imports (standard library first, then the rest, each group
/// sorted by path as gofmt sorts them) and the body, aligned.
#[must_use]
pub fn go_file(package: &str, imports: &[String], body: &str) -> String {
    let mut out = String::new();
    out.push_str(HEADER);
    out.push_str("\n\npackage ");
    out.push_str(package);
    out.push_str("\n\n");
    let mut std: Vec<&str> = imports.iter().map(String::as_str).filter(|p| !is_module(p)).collect();
    let mut module: Vec<&str> = imports.iter().map(String::as_str).filter(|p| is_module(p)).collect();
    std.sort_unstable();
    std.dedup();
    module.sort_unstable();
    module.dedup();
    if !std.is_empty() || !module.is_empty() {
        out.push_str("import (\n");
        for p in &std {
            let _ = writeln!(out, "\t{}", quote(p));
        }
        if !std.is_empty() && !module.is_empty() {
            out.push('\n');
        }
        for p in &module {
            let _ = writeln!(out, "\t{}", quote(p));
        }
        out.push_str(")\n\n");
    }
    out.push_str(&align(body.trim_end_matches('\n')));
    out.push('\n');
    out
}

/// Whether an import path is outside the standard library (its first element has a dot).
fn is_module(path: &str) -> bool {
    path.split('/').next().is_some_and(|first| first.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_runs_of_adjacent_cells() {
        let text = "type A struct {\n\tID\x0bstring\x0b`json:\"id\"`\n\tFareURL\x0bstring\x0b`json:\"fareUrl\"`\n\tJSON\x0baJSON\x0b`json:\"-\"`\n}";
        assert_eq!(
            align(text),
            "type A struct {\n\tID      string `json:\"id\"`\n\tFareURL string `json:\"fareUrl\"`\n\tJSON    aJSON  `json:\"-\"`\n}"
        );
    }

    #[test]
    fn comment_lines_end_a_block() {
        let text = "\t// doc\n\tA\x0bint\x0b`t`\n\tBB\x0bint\x0b`t`\n\t// doc\n\tCCC\x0bint\x0b`t`";
        assert_eq!(
            align(text),
            "\t// doc\n\tA  int `t`\n\tBB int `t`\n\t// doc\n\tCCC int `t`"
        );
    }

    #[test]
    fn a_line_without_the_column_splits_it() {
        // The middle line has no type cell, so the tag columns above and below are separate.
        let text = "\tA\x0bint\x0b`t`\n\tBBBB\x0bstring\n\tC\x0bfloat64\x0b`t`";
        assert_eq!(align(text), "\tA    int `t`\n\tBBBB string\n\tC    float64 `t`");
    }

    #[test]
    fn wraps_comments_at_80_characters() {
        let text =
            "Number of stops between the arriving transit vehicle and the current stop (excluding the current stop).";
        assert_eq!(
            comment_lines(text),
            [
                "Number of stops between the arriving transit vehicle and the current stop",
                "(excluding the current stop)."
            ]
        );
        assert_eq!(comment_lines("a\n\nb"), ["a", "", "b"]);
        assert_eq!(comment_lines("  "), Vec::<String>::new());
    }

    #[test]
    fn groups_and_sorts_imports() {
        let file = go_file(
            "p",
            &["github.com/x/y".into(), "net/http".into(), "context".into()],
            "var x = 1\n",
        );
        assert!(
            file.contains("import (\n\t\"context\"\n\t\"net/http\"\n\n\t\"github.com/x/y\"\n)\n\nvar x = 1\n"),
            "{file}"
        );
    }
}
