//! Python source printer with indentation and deterministic imports, sorted the way the oracle
//! SDKs' ruff/isort settings sort them: sections (future, stdlib, third party, local), plain
//! imports before `from` imports, modules and names by length then text, names grouped as
//! constants, classes, then functions and variables. Port of `python-plugin/src/printer.ts`.

use std::cmp::Ordering;

use crate::js;

/// Ruff's line length in Stainless-generated SDKs.
pub const LINE_LENGTH: usize = 120;

/// Modules the oracle SDKs sort into the standard-library section.
const STDLIB: [&str; 8] = [
    "typing",
    "typing_extensions",
    "datetime",
    "os",
    "json",
    "inspect",
    "warnings",
    "functools",
];

fn section_of(module: &str) -> usize {
    if module == "__future__" {
        return 0;
    }
    if module.starts_with('.') {
        return 3;
    }
    let first = module.split('.').next().unwrap_or(module);
    if STDLIB.contains(&first) { 1 } else { 2 }
}

fn by_length(a: &str, b: &str) -> Ordering {
    js::len(a).cmp(&js::len(b)).then_with(|| js::cmp(a, b))
}

fn name_rank(name: &str) -> u8 {
    let base = name.split(' ').next().unwrap_or(name);
    if js::len(base) > 1 && base == base.to_uppercase() && base.chars().any(|c| c.is_ascii_uppercase()) {
        return 0;
    }
    if base.starts_with(|c: char| c.is_ascii_uppercase()) {
        return 1;
    }
    2
}

fn by_name_type(a: &str, b: &str) -> Ordering {
    name_rank(a).cmp(&name_rank(b)).then_with(|| by_length(a, b))
}

/// Pushes `item` unless it is there already, keeping first-insertion order (a JavaScript `Set`).
fn insert(set: &mut Vec<String>, item: &str) {
    if !set.iter().any(|s| s == item) {
        set.push(item.to_owned());
    }
}

/// Marks where an [`PythonPrinter::open`] block began.
#[must_use]
#[derive(Debug, Clone, Copy)]
pub struct Opened(usize);

#[derive(Debug, Clone)]
pub struct PythonPrinter {
    lines: Vec<String>,
    depth: usize,
    plain_imports: Vec<String>,
    from_imports: Vec<(String, Vec<String>)>,
    exploded: Vec<String>,
    header: Vec<String>,
    indent_unit: String,
}

impl Default for PythonPrinter {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl PythonPrinter {
    #[must_use]
    pub fn new(header: &[&str]) -> Self {
        Self::with_indent(header, "    ")
    }

    #[must_use]
    pub fn with_indent(header: &[&str], indent_unit: &str) -> Self {
        Self {
            lines: Vec::new(),
            depth: 0,
            plain_imports: Vec::new(),
            from_imports: Vec::new(),
            exploded: Vec::new(),
            header: header.iter().map(|&h| h.to_owned()).collect(),
            indent_unit: indent_unit.to_owned(),
        }
    }

    /// Prints `from module import (...)` one name per line (a magic trailing comma).
    pub fn explode(&mut self, module: &str) -> &mut Self {
        insert(&mut self.exploded, module);
        self
    }

    /// `import module`
    pub fn import_module(&mut self, module: &str) -> &mut Self {
        insert(&mut self.plain_imports, module);
        self
    }

    /// `from module import name`; `name` may be `Field as FieldInfo`.
    pub fn import_from(&mut self, module: &str, name: &str) -> &mut Self {
        if let Some((_, names)) = self.from_imports.iter_mut().find(|(m, _)| m == module) {
            insert(names, name);
        } else {
            self.from_imports.push((module.to_owned(), vec![name.to_owned()]));
        }
        self
    }

    /// One line at the current indentation; an empty string prints a blank line.
    pub fn line(&mut self, text: &str) -> &mut Self {
        let line = if text.is_empty() {
            String::new()
        } else {
            format!("{}{text}", self.indentation())
        };
        self.lines.push(line);
        self
    }

    /// A blank line.
    pub fn blank(&mut self) -> &mut Self {
        self.line("")
    }

    /// The current indentation.
    #[must_use]
    pub fn indentation(&self) -> String {
        self.indent_unit.repeat(self.depth)
    }

    /// A line printed as is, already indented.
    pub fn raw(&mut self, text: &str) -> &mut Self {
        self.lines.push(text.to_owned());
        self
    }

    /// Print `header` then run `body` one level deeper.
    pub fn block(&mut self, header: &str, body: impl FnOnce(&mut Self)) -> &mut Self {
        self.line(header);
        self.indented(body)
    }

    /// Run `body` one level deeper; prints `pass` if it printed nothing.
    pub fn indented(&mut self, body: impl FnOnce(&mut Self)) -> &mut Self {
        let opened = self.open();
        body(self);
        self.close(opened);
        self
    }

    /// Starts an [`indented`](Self::indented) body that [`close`](Self::close) ends, for bodies
    /// that need more than the printer.
    pub fn open(&mut self) -> Opened {
        self.depth += 1;
        Opened(self.lines.len())
    }

    /// Ends a body [`open`](Self::open) started; prints `pass` if it printed nothing.
    pub fn close(&mut self, opened: Opened) {
        if self.lines.len() == opened.0 {
            self.line("pass");
        }
        self.depth -= 1;
    }

    fn import_lines(&self, indent: &str) -> Vec<String> {
        let mut sections: [Vec<String>; 4] = Default::default();
        let mut plain: Vec<&String> = self.plain_imports.iter().collect();
        plain.sort_by(|a, b| by_length(a, b));
        for m in plain {
            sections[section_of(m)].push(format!("{indent}import {m}"));
        }
        let mut modules: Vec<&(String, Vec<String>)> = self.from_imports.iter().collect();
        modules.sort_by(|a, b| by_length(&a.0, &b.0));
        for (m, names) in modules {
            let mut names: Vec<&String> = names.iter().collect();
            names.sort_by(|a, b| by_name_type(a, b));
            let joined = names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(", ");
            let one = format!("{indent}from {m} import {joined}");
            let text = if js::len(&one) <= LINE_LENGTH && !self.exploded.contains(m) {
                one
            } else {
                let mut parts = vec![format!("{indent}from {m} import (")];
                parts.extend(names.iter().map(|n| format!("{indent}    {n},")));
                parts.push(format!("{indent})"));
                parts.join("\n")
            };
            sections[section_of(m)].push(text);
        }
        sections
            .into_iter()
            .filter(|s| !s.is_empty())
            .flat_map(|mut s| {
                s.push(String::new());
                s
            })
            .collect()
    }

    /// The registered imports alone, at `indent`, for an `if TYPE_CHECKING:` block.
    #[must_use]
    pub fn import_block(&self, indent: &str) -> Vec<String> {
        let joined = self.import_lines(indent).join("\n");
        let mut lines: Vec<String> = joined.split('\n').map(str::to_owned).collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines
    }

    /// Header, imports by section, then the body, ending with exactly one newline.
    #[must_use]
    pub fn render(&self) -> String {
        let mut all: Vec<String> = Vec::new();
        if !self.header.is_empty() {
            all.extend(self.header.iter().cloned());
            all.push(String::new());
        }
        all.extend(self.import_lines(""));
        all.extend(self.lines.iter().cloned());
        format!("{}\n", js::trim(&all.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::PythonPrinter;

    #[test]
    fn sorts_imports_into_sections_by_length_classes_before_functions() {
        let mut p = PythonPrinter::new(&["# header"]);
        p.import_from(".._types", "not_given")
            .import_from(".._types", "NotGiven")
            .import_from(".._types", "Body")
            .import_from(".._types", "MAX")
            .import_from("typing", "Optional")
            .import_module("os")
            .import_from("typing", "List")
            .import_from("__future__", "annotations")
            .import_module("httpx")
            .import_from("typing_extensions", "Literal")
            .import_from(".._compat", "cached_property")
            .import_from("typing", "List");
        p.block("class Foo:", |b| {
            b.block("def bar(self) -> None:", |_| {});
            b.blank();
            b.line("x = 1");
        });
        assert_eq!(
            p.render(),
            [
                "# header",
                "",
                "from __future__ import annotations",
                "",
                "import os",
                "from typing import List, Optional",
                "from typing_extensions import Literal",
                "",
                "import httpx",
                "",
                "from .._types import MAX, Body, NotGiven, not_given",
                "from .._compat import cached_property",
                "",
                "class Foo:",
                "    def bar(self) -> None:",
                "        pass",
                "",
                "    x = 1",
                "",
            ]
            .join("\n")
        );
    }

    #[test]
    fn wraps_an_import_longer_than_the_line_length() {
        let mut p = PythonPrinter::default();
        for n in [
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "cccccccccccccccccccccccccccccccccccccccccccc",
        ] {
            p.import_from(".mod", n);
        }
        assert_eq!(
            p.render(),
            [
                "from .mod import (",
                "    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,",
                "    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,",
                "    cccccccccccccccccccccccccccccccccccccccccccc,",
                ")",
                "",
            ]
            .join("\n")
        );
    }

    #[test]
    fn prints_without_imports_with_a_custom_indent() {
        let mut p = PythonPrinter::with_indent(&[], "  ");
        p.block("if x:", |b| {
            b.line("y()");
        });
        assert_eq!(p.render(), "if x:\n  y()\n");
    }
}
