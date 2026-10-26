//! The frame of every generated PHP file: the `<?php` header, the namespace, the sorted `use`
//! lines and docblocks.

use std::fmt::Write as _;

use crate::layout::indent;
use crate::types::Imports;

/// `<?php`, strict types, the namespace, the imports and `body` (which starts with the class's
/// docblock or declaration).
#[must_use]
pub fn php_file(root: &str, imp: &Imports, body: &str) -> String {
    let mut out = String::from("<?php\n\ndeclare(strict_types=1);\n\n");
    if imp.namespace.is_empty() {
        let _ = write!(out, "namespace {root};\n\n");
    } else {
        let _ = write!(out, "namespace {root}\\{};\n\n", imp.namespace.join("\\"));
    }
    let uses = imp.use_lines();
    if !uses.is_empty() {
        for u in uses {
            out.push_str(&u);
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(body);
    out
}

/// Marks a docblock line that is already laid out with its ` * ` prefix (an array shape).
pub const RAW: char = '\u{1}';

/// The lines of [`crate::layout::doc_shape`] as docblock lines.
#[must_use]
pub fn raw_lines(lines: Vec<String>) -> Vec<String> {
    lines.into_iter().map(|l| format!("{RAW}{l}")).collect()
}

/// The lines of a description in a docblock (` * ` prefixes added by [`docblock`]).
#[must_use]
pub fn text_lines(text: &str) -> Vec<String> {
    let t = text.trim_end();
    if t.trim().is_empty() {
        return Vec::new();
    }
    t.lines().map(|l| l.trim_end().to_owned()).collect()
}

/// A docblock at `depth` from sections (each a list of lines without the ` * ` prefix), with a
/// blank line between non-empty sections; empty when every section is.
#[must_use]
pub fn docblock(sections: &[Vec<String>], depth: usize) -> String {
    let ind = indent(depth);
    let sections: Vec<&Vec<String>> = sections.iter().filter(|s| !s.is_empty()).collect();
    if sections.is_empty() {
        return String::new();
    }
    let mut out = format!("{ind}/**\n");
    for (i, s) in sections.iter().enumerate() {
        if i > 0 {
            let _ = writeln!(out, "{ind} *");
        }
        for line in *s {
            if line.is_empty() {
                let _ = writeln!(out, "{ind} *");
            } else if let Some(raw) = line.strip_prefix(RAW) {
                out.push_str(raw);
                out.push('\n');
            } else {
                let _ = writeln!(out, "{ind} * {line}");
            }
        }
    }
    let _ = writeln!(out, "{ind} */");
    out
}
