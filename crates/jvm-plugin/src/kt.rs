//! Kotlin source text: the file header, imports and `KDoc` comments.
//!
//! The plugin writes Kotlin with Stainless's tokens, comments and blank lines but not its line
//! breaks: `ktfmt --kotlinlang-style` (0.61, the version the Stainless JVM SDKs pin) lays it out
//! in `ferrule generate` and in the tests, sorts the imports and drops the unused ones. Text that
//! ktfmt keeps as written (a `KDoc` table, a code fence) is written in its final layout.

use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The first line of every generated Kotlin file (Stainless's, branded).
pub const HEADER: &str = "// File generated from our OpenAPI spec by Ferrule.";

/// The imports of one file, as full paths.
#[derive(Debug, Default, Clone)]
pub struct Imports {
    root: String,
    paths: BTreeSet<String>,
}

impl Imports {
    /// Imports for a file of the SDK rooted at package `root` (`org.onebusaway`).
    #[must_use]
    pub fn new(root: &str) -> Self {
        Self {
            root: root.to_owned(),
            paths: BTreeSet::new(),
        }
    }

    /// Imports a full path: `java.util.Objects`.
    pub fn add(&mut self, path: impl Into<String>) {
        self.paths.insert(path.into());
    }

    /// Imports a path under the SDK's root package: `core.JsonField`.
    pub fn sdk(&mut self, rel: &str) {
        self.paths.insert(format!("{}.{rel}", self.root));
    }

    /// Imports a name from the SDK's `core` package.
    pub fn core(&mut self, name: &str) {
        self.sdk(&format!("core.{name}"));
    }

    /// The SDK's root package.
    #[must_use]
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Merges `other` into these imports.
    pub fn extend(&mut self, other: &Self) {
        self.paths.extend(other.paths.iter().cloned());
    }
}

/// A whole Kotlin file: header, `package`, imports and `body`.
#[must_use]
pub fn file(package: &str, imports: &Imports, body: &str) -> String {
    let mut out = format!("{HEADER}\n\npackage {package}\n\n");
    let own = |p: &str| {
        p.rsplit_once('.')
            .is_some_and(|(pkg, name)| pkg == package && !name.is_empty())
    };
    let mut any = false;
    for p in imports.paths.iter().filter(|p| !own(p)) {
        let _ = writeln!(out, "import {p}");
        any = true;
    }
    if any {
        out.push('\n');
    }
    out.push_str(body.trim_start_matches('\n'));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// A `KDoc` comment of `text` (one paragraph per blank line, as written), with a newline; empty
/// for empty text.
#[must_use]
pub fn doc(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::from("/**\n");
    for line in text.replace("*/", "*&#47;").lines() {
        let line = line.trim_end();
        if line.is_empty() {
            out.push_str(" *\n");
        } else {
            let _ = writeln!(out, " * {line}");
        }
    }
    out.push_str(" */\n");
    out
}

/// A Kotlin string literal of `s`.
#[must_use]
pub fn string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Indents every non-empty line of `text` by `levels` steps of four spaces.
#[must_use]
pub fn indent(text: &str, levels: usize) -> String {
    let pad = "    ".repeat(levels);
    text.lines()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if text.ends_with('\n') { "\n" } else { "" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_file_with_sorted_imports() {
        let mut i = Imports::new("org.acme");
        i.core("JsonField");
        i.add("java.util.Objects");
        i.sdk("models.Widget");
        let text = file("org.acme.models", &i, "class X\n");
        assert_eq!(
            text,
            "// File generated from our OpenAPI spec by Ferrule.\n\npackage org.acme.models\n\n\
             import java.util.Objects\nimport org.acme.core.JsonField\n\nclass X\n"
        );
    }

    #[test]
    fn writes_kdoc_and_strings() {
        assert_eq!(doc("One\n\nTwo"), "/**\n * One\n *\n * Two\n */\n");
        assert_eq!(doc(" "), "");
        assert_eq!(string("a\"$b"), "\"a\\\"\\$b\"");
        assert_eq!(indent("a\n\nb\n", 1), "    a\n\n    b\n");
    }
}
