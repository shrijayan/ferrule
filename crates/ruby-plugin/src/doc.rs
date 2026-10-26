//! A Wadler-style pretty printer (the algorithm of prettier and of `syntax_tree`'s `prettier_print`),
//! for the expressions of the `lib/**.rb` files: Stainless lays them out at 110 columns
//! (`.rubocop.yml`'s `Layout/LineLength`) with a two-space indent.

/// A document: text and the places where it may break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doc {
    Text(String),
    /// A space, or a newline when its group breaks.
    Line,
    /// Nothing, or a newline when its group breaks.
    SoftLine,
    /// Always a newline; the groups around it break.
    HardLine,
    /// Printed flat when it fits on the rest of the line, else broken.
    Group(Box<Doc>),
    /// Two more spaces of indent after each newline inside.
    Indent(Box<Doc>),
    Concat(Vec<Doc>),
    /// The first when its group breaks, the second when it is flat.
    IfBreak(Box<Doc>, Box<Doc>),
}

#[must_use]
pub fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}

#[must_use]
pub fn group(d: Doc) -> Doc {
    Doc::Group(Box::new(d))
}

#[must_use]
pub fn indent(d: Doc) -> Doc {
    Doc::Indent(Box::new(d))
}

#[must_use]
pub fn concat(parts: Vec<Doc>) -> Doc {
    Doc::Concat(parts)
}

#[must_use]
pub fn if_break(broken: Doc, flat: Doc) -> Doc {
    Doc::IfBreak(Box::new(broken), Box::new(flat))
}

/// `parts` separated by `sep`.
#[must_use]
pub fn join(sep: &Doc, parts: Vec<Doc>) -> Doc {
    let mut out = Vec::with_capacity(parts.len() * 2);
    for (i, p) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push(sep.clone());
        }
        out.push(p);
    }
    Doc::Concat(out)
}

/// `name(args)`: the arguments on one line, or one per line inside the parentheses.
#[must_use]
pub fn call(name: &str, args: Vec<Doc>) -> Doc {
    if args.is_empty() {
        return text(format!("{name}()"));
    }
    group(concat(vec![
        text(format!("{name}(")),
        indent(concat(vec![
            Doc::SoftLine,
            join(&concat(vec![text(","), Doc::Line]), args),
        ])),
        Doc::SoftLine,
        text(")"),
    ]))
}

/// `target = value`, breaking after `=` when the value does not fit.
#[must_use]
pub fn assign(target: &str, value: Doc) -> Doc {
    group(concat(vec![
        text(format!("{target} =")),
        indent(concat(vec![Doc::Line, value])),
    ]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

fn has_hardline(d: &Doc) -> bool {
    match d {
        Doc::HardLine => true,
        Doc::Group(c) | Doc::Indent(c) => has_hardline(c),
        Doc::Concat(cs) => cs.iter().any(has_hardline),
        Doc::IfBreak(b, f) => has_hardline(b) || has_hardline(f),
        Doc::Text(_) | Doc::Line | Doc::SoftLine => false,
    }
}

fn width(s: &str) -> isize {
    isize::try_from(s.chars().count()).unwrap_or(isize::MAX)
}

/// Whether `next` and then `rest` (innermost last) fit in `left` columns up to the next newline.
fn fits(next: (Mode, &Doc), rest: &[(usize, Mode, &Doc)], mut left: isize) -> bool {
    let mut stack: Vec<(Mode, &Doc)> = vec![next];
    let mut rest_idx = rest.len();
    loop {
        if left < 0 {
            return false;
        }
        let Some((mode, doc)) = stack.pop() else {
            if rest_idx == 0 {
                return true;
            }
            rest_idx -= 1;
            let (_, m, d) = rest[rest_idx];
            stack.push((m, d));
            continue;
        };
        match doc {
            Doc::Text(s) => left -= width(s),
            Doc::Concat(cs) => stack.extend(cs.iter().rev().map(|c| (mode, c))),
            Doc::Indent(c) => stack.push((mode, c)),
            Doc::Group(c) => {
                let m = if has_hardline(c) { Mode::Break } else { mode };
                stack.push((m, c));
            }
            Doc::IfBreak(b, f) => stack.push((mode, if mode == Mode::Break { b } else { f })),
            Doc::Line => {
                if mode == Mode::Break {
                    return true;
                }
                left -= 1;
            }
            Doc::SoftLine => {
                if mode == Mode::Break {
                    return true;
                }
            }
            Doc::HardLine => return true,
        }
    }
}

/// Prints a statement of `lib/**.rb` or `test/**.rb` at `indent`, as Stainless's generator
/// measures it: from column 4 at most, whatever the depth of the block it sits in (its printer
/// lays out each body on its own), at 110 columns.
#[must_use]
pub fn print_rb(doc: &Doc, indent: usize) -> String {
    let measured = indent.min(4);
    let shift = " ".repeat(indent - measured);
    print(doc, 110, measured).replace('\n', &format!("\n{shift}"))
}

/// Prints `doc` at `max` columns, starting at column `start` with `start` spaces of indent.
#[must_use]
pub fn print(doc: &Doc, max: usize, start: usize) -> String {
    let max = isize::try_from(max).unwrap_or(isize::MAX);
    let mut out = String::new();
    let mut pos = isize::try_from(start).unwrap_or(0);
    let mut cmds: Vec<(usize, Mode, &Doc)> = vec![(start, Mode::Break, doc)];
    while let Some((ind, mode, d)) = cmds.pop() {
        match d {
            Doc::Text(s) => {
                out.push_str(s);
                pos += width(s);
            }
            Doc::Concat(cs) => cmds.extend(cs.iter().rev().map(|c| (ind, mode, c))),
            Doc::Indent(c) => cmds.push((ind + 2, mode, c)),
            Doc::Group(c) => {
                let m = if mode == Mode::Flat || (!has_hardline(c) && fits((Mode::Flat, c), &cmds, max - pos)) {
                    Mode::Flat
                } else {
                    Mode::Break
                };
                cmds.push((ind, m, c));
            }
            Doc::IfBreak(b, f) => cmds.push((ind, mode, if mode == Mode::Break { b } else { f })),
            Doc::Line | Doc::SoftLine if mode == Mode::Flat => {
                if *d == Doc::Line {
                    out.push(' ');
                    pos += 1;
                }
            }
            Doc::Line | Doc::SoftLine | Doc::HardLine => {
                while out.ends_with(' ') {
                    out.pop();
                }
                out.push('\n');
                out.push_str(&" ".repeat(ind));
                pos = isize::try_from(ind).unwrap_or(0);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_only_the_groups_that_do_not_fit() {
        let d = call("f", vec![text("aaaa"), call("g", vec![text("bbbb"), text("cccc")])]);
        assert_eq!(print(&d, 80, 0), "f(aaaa, g(bbbb, cccc))");
        assert_eq!(print(&d, 16, 0), "f(\n  aaaa,\n  g(bbbb, cccc)\n)");
        assert_eq!(print(&d, 10, 0), "f(\n  aaaa,\n  g(\n    bbbb,\n    cccc\n  )\n)");
    }

    #[test]
    fn breaks_an_assignment_after_the_equals_sign() {
        let d = assign("x", text("y".repeat(20)));
        assert_eq!(print(&d, 30, 2), format!("x = {}", "y".repeat(20)));
        assert_eq!(print(&d, 20, 2), format!("x =\n    {}", "y".repeat(20)));
    }
}
