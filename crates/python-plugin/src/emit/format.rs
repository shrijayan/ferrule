//! Line splitting the way ruff format (black style) splits the bracketed expressions the emitters
//! print: a statement stays on one line when it fits in 120 columns and nothing inside it has a
//! magic trailing comma; otherwise its last bracket is split, the contents go on one indented line
//! when they fit there, and otherwise one item per line with a trailing comma, each split the same
//! way. The plugin does not run ruff, so this replaces it (Forge gaps G8, G15).
//! Port of `python-plugin/src/emit/format.ts`.

use crate::js;
use crate::printer::{LINE_LENGTH, PythonPrinter};

/// An atom (printed as is) or a bracketed list: `open` + items joined by `, ` + `close`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Atom(String),
    Bracket(Bracket),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bracket {
    /// Everything up to and including the opening bracket, such as `self._get(`.
    pub open: String,
    pub items: Vec<Expr>,
    /// The closing bracket, such as `)`.
    pub close: String,
    /// A trailing comma after the last item, which keeps the bracket split.
    pub magic: bool,
}

impl From<String> for Expr {
    fn from(s: String) -> Self {
        Self::Atom(s)
    }
}

impl From<&str> for Expr {
    fn from(s: &str) -> Self {
        Self::Atom(s.to_owned())
    }
}

impl From<Bracket> for Expr {
    fn from(b: Bracket) -> Self {
        Self::Bracket(b)
    }
}

/// `open` + `items` + `)`.
#[must_use]
pub fn call(open: &str, items: Vec<Expr>, magic: bool) -> Bracket {
    bracket(open, items, magic, ")")
}

#[must_use]
pub fn bracket(open: &str, items: Vec<Expr>, magic: bool, close: &str) -> Bracket {
    Bracket {
        open: open.to_owned(),
        items,
        close: close.to_owned(),
        magic,
    }
}

fn has_magic(e: &Expr) -> bool {
    match e {
        Expr::Atom(_) => false,
        Expr::Bracket(b) => b.magic || b.items.iter().any(has_magic),
    }
}

/// The expression on one line.
#[must_use]
pub fn flat(e: &Expr) -> String {
    match e {
        Expr::Atom(s) => s.clone(),
        Expr::Bracket(b) => format!(
            "{}{}{}",
            b.open,
            b.items.iter().map(flat).collect::<Vec<_>>().join(", "),
            b.close
        ),
    }
}

fn fits(line: &str) -> bool {
    js::len(line) <= LINE_LENGTH
}

/// The lines of `prefix + e + suffix` at `indent`, split as ruff would.
#[must_use]
pub fn format_expr(e: &Expr, indent: &str, prefix: &str, suffix: &str) -> Vec<String> {
    let b = match e {
        Expr::Atom(s) => return vec![format!("{indent}{prefix}{s}{suffix}")],
        Expr::Bracket(b) => b,
    };
    let one = format!("{indent}{prefix}{}{suffix}", flat(e));
    if !has_magic(e) && fits(&one) {
        return vec![one];
    }
    let inner = format!("{indent}    ");
    let joined = format!("{inner}{}", b.items.iter().map(flat).collect::<Vec<_>>().join(", "));
    let body = if !has_magic(e) && !b.items.is_empty() && fits(&joined) {
        vec![joined]
    } else if let [Expr::Atom(only)] = b.items.as_slice()
        && !b.magic
    {
        vec![format!("{inner}{only}")]
    } else {
        b.items
            .iter()
            .flat_map(|item| format_expr(item, &inner, "", ","))
            .collect()
    };
    let mut out = vec![format!("{indent}{prefix}{}", b.open)];
    out.extend(body);
    out.push(format!("{indent}{}{suffix}", b.close));
    out
}

/// Prints `prefix + e + suffix` at the printer's indentation.
pub fn print_expr(p: &mut PythonPrinter, e: &Expr, prefix: &str, suffix: &str) {
    for l in format_expr(e, &p.indentation(), prefix, suffix) {
        p.raw(&l);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_calls_the_way_ruff_splits_them() {
        let long = "x".repeat(130);
        assert_eq!(
            format_expr(&call("f(", vec![long.clone().into()], false).into(), "", "", ""),
            ["f(".to_owned(), format!("    {long}"), ")".to_owned()]
        );
        let nested = call("f(", vec!["a".into(), call("g(", vec!["b".into()], true).into()], false);
        assert_eq!(
            format_expr(&nested.into(), "", "", ""),
            ["f(", "    a,", "    g(", "        b,", "    ),", ")"]
        );
    }
}
