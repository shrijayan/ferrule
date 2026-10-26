//! The generator's line breaking, which php-cs-fixer keeps.
//!
//! Stainless's PHP generator lays out lists in brackets (parameters, arguments, arrays, array
//! shapes in phpdoc) at 80 columns with an indent of 2 per level, and php-cs-fixer then re-indents
//! to 4 without joining or splitting them (its `method_argument_space` only puts the items of a
//! hugged list on lines of their own). A list is printed in the first of three forms that fits:
//!
//! - flat: `f(a, b)` on the line;
//! - hugged: the items on one line of their own, `f(\n  a, b\n)`, with no trailing comma;
//! - expanded: one item per line, each with a trailing comma, items laid out the same way.
//!
//! This module measures in the generator's columns (2 per level) and writes php-cs-fixer's
//! indentation (4 per level), so the output reads like the formatted file.

/// The generator's line width.
pub const WIDTH: usize = 80;

/// What follows a method's parameter list on its line in the generator's output: ` {`.
pub const BRACE: usize = 2;

/// A piece of code: text, or a list in brackets that may break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doc {
    Text(String),
    /// `open` items `close`, separated by `, `.
    List {
        open: String,
        items: Vec<Doc>,
        close: String,
    },
    Cat(Vec<Doc>),
}

#[must_use]
pub fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

#[must_use]
pub fn list(open: impl Into<String>, items: Vec<Doc>, close: impl Into<String>) -> Doc {
    Doc::List {
        open: open.into(),
        items,
        close: close.into(),
    }
}

#[must_use]
pub fn cat(parts: Vec<Doc>) -> Doc {
    Doc::Cat(parts)
}

/// Display width of a string.
#[must_use]
pub fn width(s: &str) -> usize {
    s.chars().count()
}

impl Doc {
    /// The one-line form.
    #[must_use]
    pub fn flat(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::List { open, items, close } => {
                format!(
                    "{open}{}{close}",
                    items.iter().map(Self::flat).collect::<Vec<_>>().join(", ")
                )
            }
            Self::Cat(parts) => parts.iter().map(Self::flat).collect(),
        }
    }

    /// The width of the text before the first place this may break.
    fn head(&self) -> (usize, bool) {
        match self {
            Self::Text(s) => (width(s), false),
            Self::List { open, .. } => (width(open), true),
            Self::Cat(parts) => {
                let mut w = 0;
                for p in parts {
                    let (pw, stop) = p.head();
                    w += pw;
                    if stop {
                        return (w, true);
                    }
                }
                (w, false)
            }
        }
    }
}

/// The indentation php-cs-fixer writes at `depth`.
#[must_use]
pub fn indent(depth: usize) -> String {
    " ".repeat(depth * 4)
}

/// Lays out `doc` starting at generator column `col` (the code before it on the line is already
/// written) at `depth`; `rest` is the width of what follows it on its last line.
#[must_use]
pub fn render(doc: &Doc, depth: usize, col: usize, rest: usize) -> String {
    let mut out = String::new();
    go(doc, depth, col, rest, &mut out);
    out
}

/// Lays out a whole statement at `depth` (its indentation included).
#[must_use]
pub fn statement(doc: &Doc, depth: usize) -> String {
    format!("{}{}", indent(depth), render(doc, depth, depth * 2, 0))
}

fn go(doc: &Doc, depth: usize, col: usize, rest: usize, out: &mut String) -> usize {
    match doc {
        Doc::Text(s) => {
            out.push_str(s);
            col + width(s)
        }
        Doc::Cat(parts) => {
            let mut col = col;
            for (i, p) in parts.iter().enumerate() {
                let mut after = 0;
                let mut stopped = false;
                for q in &parts[i + 1..] {
                    let (w, stop) = q.head();
                    after += w;
                    if stop {
                        stopped = true;
                        break;
                    }
                }
                let r = if stopped { after } else { after + rest };
                col = go(p, depth, col, r, out);
            }
            col
        }
        Doc::List { open, items, close } => {
            let flat = doc.flat();
            if col + width(&flat) + rest <= WIDTH || items.is_empty() {
                out.push_str(&flat);
                return col + width(&flat);
            }
            let inner: String = items.iter().map(Doc::flat).collect::<Vec<_>>().join(", ");
            out.push_str(open);
            if (depth + 1) * 2 + width(&inner) <= WIDTH {
                out.push('\n');
                out.push_str(&indent(depth + 1));
                out.push_str(&inner);
            } else {
                for item in items {
                    out.push('\n');
                    out.push_str(&indent(depth + 1));
                    go(item, depth + 1, (depth + 1) * 2, 1, out);
                    out.push(',');
                }
            }
            out.push('\n');
            out.push_str(&indent(depth));
            out.push_str(close);
            depth * 2 + width(close)
        }
    }
}

/// A list in a docblock: `head` `open` items `close` `tail`, as lines at `depth` (with their
/// ` * ` prefixes). `base` is the generator column the head starts at (the generator does not
/// measure every docblock alike: 0 for a class's shapes, 3 for a method's). The items of a hugged
/// or expanded list are indented by two after the `*`.
#[must_use]
pub fn doc_list(
    head: &str,
    (open, close): (&str, &str),
    items: &[String],
    tail: &str,
    depth: usize,
    base: usize,
) -> Vec<String> {
    let prefix = format!("{} * ", indent(depth));
    let flat = format!("{head}{open}{}{close}{tail}", items.join(", "));
    if items.is_empty() || base + width(&flat) <= WIDTH {
        return vec![format!("{prefix}{flat}")];
    }
    let mut out = vec![format!("{prefix}{head}{open}")];
    let inner = items.join(", ");
    if base + 2 + width(&inner) <= WIDTH {
        out.push(format!("{prefix}  {inner}"));
    } else {
        out.extend(items.iter().map(|i| format!("{prefix}  {i},")));
    }
    out.push(format!("{prefix}{close}{tail}"));
    out
}

/// An array shape in a docblock (see [`doc_list`]).
#[must_use]
pub fn doc_shape(head: &str, items: &[String], tail: &str, depth: usize, base: usize) -> Vec<String> {
    doc_list(head, ("array{", "}"), items, tail, depth, base)
}

/// A method chain in a docblock: `start->a()->b()` on one line, else one call per line indented
/// by two.
#[must_use]
pub fn doc_chain(start: &str, calls: &[String], depth: usize, base: usize) -> Vec<String> {
    let prefix = format!("{} * ", indent(depth));
    let flat = format!("{start}{}", calls.concat());
    if base + width(&flat) <= WIDTH {
        return vec![format!("{prefix}{flat}")];
    }
    let mut out = vec![format!("{prefix}{start}")];
    out.extend(calls.iter().map(|c| format!("{prefix}  {c}")));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, args: &[&str]) -> Doc {
        list(format!("{name}("), args.iter().map(|a| text(*a)).collect(), ")")
    }

    #[test]
    fn prints_flat_hugged_and_expanded() {
        let short = cat(vec![text("$x = "), call("f", &["$a", "$b"]), text(";")]);
        assert_eq!(statement(&short, 2), "        $x = f($a, $b);");
        let long = format!("${}", "a".repeat(50));
        let long = long.as_str();
        let hug = cat(vec![
            text("$x = "),
            call("fffffffffffffffffffff", &[long, "$b"]),
            text(";"),
        ]);
        assert_eq!(
            statement(&hug, 2),
            format!("        $x = fffffffffffffffffffff(\n            {long}, $b\n        );")
        );
        let exp = cat(vec![text("$x = "), call("f", &[long, long]), text(";")]);
        assert_eq!(
            statement(&exp, 2),
            format!("        $x = f(\n            {long},\n            {long},\n        );")
        );
    }

    #[test]
    fn shapes_in_docblocks() {
        assert_eq!(
            doc_shape("@phpstan-type AShape = ", &["a: string".into()], "", 0, 0),
            [" * @phpstan-type AShape = array{a: string}"]
        );
        let three: Vec<String> = (0..3).map(|i| format!("field{i}?: string|null")).collect();
        assert_eq!(
            doc_shape("@param ", &three, "|AParams $params", 1, 3)
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "     * @param array{",
                "     *   field0?: string|null, field1?: string|null, field2?: string|null",
                "     * }|AParams $params",
            ]
        );
        let four: Vec<String> = (0..4).map(|i| format!("field{i}?: string|null")).collect();
        assert_eq!(
            doc_shape("@phpstan-type AShape = ", &four, "", 0, 0)
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                " * @phpstan-type AShape = array{",
                " *   field0?: string|null,",
                " *   field1?: string|null,",
                " *   field2?: string|null,",
                " *   field3?: string|null,",
                " * }",
            ]
        );
    }
}
