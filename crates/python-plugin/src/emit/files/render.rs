//! The template language of the project files: `%%NAME%%` is replaced by `vars[NAME]`, and a line
//! starting with `%%isort%%` is a one-line `from x import a, b` whose names are re-sorted the way
//! the oracle's ruff isort config (`length-sort = true`) orders them once the tokens are filled.
//!
//! Lines `%%if NAME%%` (or `%%if !NAME%%`), `%%else%%` and `%%end%%` keep or drop the lines between
//! them by whether `vars[NAME]` is non-empty; they nest. A line starting with `%%wrap%%` (or
//! `%%wrapmd%%`) is one Python statement laid out by [`super::layout`] within 120 columns (the
//! SDK's ruff `line-length`; 88 for README examples, ruff's default) once the tokens are filled.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use super::layout::layout;
use crate::js;

/// Token name -> value.
pub type TemplateVars = BTreeMap<&'static str, String>;

/// A template that cannot be rendered: a bug in a bundled template, never in user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// A `%%TOKEN%%` with no value.
    UnknownToken(String),
    /// A `%%isort%%` line that is not `from <module> import <names>`.
    NotAnImport(String),
    /// An `%%else%%` or `%%end%%` without its `%%if%%`, or an `%%if%%` without its `%%end%%`.
    Unbalanced(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownToken(token) => write!(f, "template token {token} has no value"),
            Self::NotAnImport(line) => {
                write!(f, "%%isort%% on a line that is not a one-line import: {line}")
            }
            Self::Unbalanced(line) => write!(f, "unbalanced conditional at: {line}"),
        }
    }
}

impl std::error::Error for RenderError {}

fn rank(name: &str) -> u8 {
    let all_caps = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    if all_caps {
        0
    } else if name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
        1
    } else {
        2
    }
}

/// Sort key of ruff isort with `length-sort`: length, then CONSTANTS, Classes, variables.
#[must_use]
pub fn isort_compare(a: &str, b: &str) -> Ordering {
    js::len(a)
        .cmp(&js::len(b))
        .then_with(|| rank(a).cmp(&rank(b)))
        .then_with(|| js::cmp(a, b))
}

/// `/^from (\S+) import (.+)$/` on one line, names re-sorted.
fn sort_import_line(line: &str) -> Result<String, RenderError> {
    let parsed = line.strip_prefix("from ").and_then(|rest| {
        let end = rest.find(js::is_space).unwrap_or(rest.len());
        let (module, tail) = rest.split_at(end);
        let names = tail.strip_prefix(" import ")?;
        (!module.is_empty() && !names.is_empty()).then_some((module, names))
    });
    let Some((module, names)) = parsed else {
        return Err(RenderError::NotAnImport(line.to_owned()));
    };
    let mut names: Vec<&str> = names.split(", ").collect();
    names.sort_by(|a, b| isort_compare(a, b));
    Ok(format!("from {module} import {}", names.join(", ")))
}

/// `/%%([A-Za-z_]+)%%/` at the start of `s`: the name and the token's length.
fn token_at(s: &str) -> Option<(&str, usize)> {
    let rest = s.strip_prefix("%%")?;
    let n = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphabetic() || *b == b'_')
        .count();
    (n > 0 && rest[n..].starts_with("%%")).then(|| (&rest[..n], n + 4))
}

/// Line markers kept through token substitution and acted on afterwards.
const MARKERS: [&str; 3] = ["isort", "wrap", "wrapmd"];

fn flag(vars: &BTreeMap<&str, String>, name: &str, line: &str) -> Result<bool, RenderError> {
    let (negated, name) = name.strip_prefix('!').map_or((false, name), |n| (true, n));
    let value = vars
        .get(name)
        .ok_or_else(|| RenderError::UnknownToken(line.to_owned()))?;
    Ok(value.is_empty() == negated)
}

/// `template` without the lines its conditionals drop, nor the conditional lines themselves.
fn select(template: &str, vars: &BTreeMap<&str, String>) -> Result<String, RenderError> {
    let mut out = String::with_capacity(template.len());
    // Per open `%%if%%`: (whether its condition holds, whether the enclosing lines are kept).
    let mut open: Vec<(bool, bool)> = Vec::new();
    let mut keep = true;
    for line in template.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        if let Some(name) = body.strip_prefix("%%if ").and_then(|r| r.strip_suffix("%%")) {
            let holds = flag(vars, name, body)?;
            open.push((holds, keep));
            keep = keep && holds;
        } else if body == "%%else%%" {
            let &(holds, outer) = open.last().ok_or_else(|| RenderError::Unbalanced(body.to_owned()))?;
            keep = outer && !holds;
        } else if body == "%%end%%" {
            let (_, outer) = open.pop().ok_or_else(|| RenderError::Unbalanced(body.to_owned()))?;
            keep = outer;
        } else if keep {
            out.push_str(line);
        }
    }
    if open.is_empty() {
        Ok(out)
    } else {
        Err(RenderError::Unbalanced("end of template".to_owned()))
    }
}

/// Fills `template` with `vars`.
///
/// # Errors
///
/// A token without a value, a `%%isort%%` line that is not a one-line import, or an unbalanced
/// conditional.
pub fn render(template: &str, vars: &BTreeMap<&str, String>) -> Result<String, RenderError> {
    let selected = select(template, vars)?;
    let mut filled = String::with_capacity(selected.len());
    let mut rest = selected.as_str();
    while let Some(at) = rest.find("%%") {
        filled.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some((name, n)) = token_at(rest) {
            let token = &rest[..n];
            if MARKERS.contains(&name) {
                filled.push_str(token);
            } else {
                let value = vars
                    .get(name)
                    .ok_or_else(|| RenderError::UnknownToken(token.to_owned()))?;
                filled.push_str(value);
            }
            rest = &rest[n..];
        } else {
            // Like a global regex scan: no match here, try again one character on.
            filled.push('%');
            rest = &rest[1..];
        }
    }
    filled.push_str(rest);

    // `/^%%isort%%(.*)$/gm`: a line is what lies between JavaScript line terminators.
    let mut out = String::with_capacity(filled.len());
    for segment in filled.split_inclusive(js::is_line_terminator) {
        let body = segment.trim_end_matches(js::is_line_terminator);
        // `split_inclusive` keeps at most one terminator per segment.
        let terminator = &segment[body.len()..];
        if let Some(line) = body.strip_prefix("%%isort%%") {
            out.push_str(&sort_import_line(line)?);
        } else if let Some(line) = body.strip_prefix("%%wrapmd%%") {
            out.push_str(&layout(line, 88));
        } else if let Some(line) = body.strip_prefix("%%wrap%%") {
            out.push_str(&layout(line, 120));
        } else {
            out.push_str(body);
        }
        out.push_str(terminator);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{RenderError, TemplateVars, isort_compare, render};

    #[test]
    fn scans_tokens_like_a_global_regex() {
        let vars = TemplateVars::from([("A", "1".to_owned())]);
        assert_eq!(render("%%%A%%%", &vars), Ok("%1%".to_owned()));
        assert_eq!(render("%% %%A%%", &vars), Ok("%% 1".to_owned()));
        assert_eq!(render("%%isort%%", &vars), Err(RenderError::NotAnImport(String::new())));
    }

    #[test]
    fn isort_lines_end_at_any_line_terminator() {
        let vars = TemplateVars::new();
        assert_eq!(
            render("%%isort%%from m import bb, a\r\nx", &vars),
            Ok("from m import a, bb\r\nx".to_owned())
        );
        assert_eq!(
            render("%%isort%%from m import", &vars),
            Err(RenderError::NotAnImport("from m import".to_owned()))
        );
    }

    #[test]
    fn keeps_the_lines_of_true_conditions() {
        let vars = TemplateVars::from([("A", "1".to_owned()), ("B", String::new())]);
        let t = "x\n%%if A%%\na\n%%if B%%\nab\n%%else%%\nanb\n%%end%%\n%%end%%\n%%if !A%%\nna\n%%end%%\ny\n";
        assert_eq!(render(t, &vars), Ok("x\na\nanb\ny\n".to_owned()));
        assert!(matches!(render("%%end%%\n", &vars), Err(RenderError::Unbalanced(_))));
        assert!(matches!(render("%%if A%%\n", &vars), Err(RenderError::Unbalanced(_))));
        assert!(matches!(
            render("%%if C%%\n%%end%%\n", &vars),
            Err(RenderError::UnknownToken(_))
        ));
    }

    #[test]
    fn lays_out_wrap_lines() {
        let vars = TemplateVars::from([("C", "Client".to_owned())]);
        assert_eq!(
            render("%%wrap%%x = %%C%%(a)\n", &vars),
            Ok("x = Client(a)\n".to_owned())
        );
        assert_eq!(
            render("%%wrapmd%%x = f(a,)\n", &vars),
            Ok("x = f(\n    a,\n)\n".to_owned())
        );
    }

    #[test]
    fn ranks_constants_classes_variables() {
        let mut names = vec!["a_B", "A_b", "AB"];
        names.sort_by(|a, b| isort_compare(a, b));
        assert_eq!(names, ["AB", "A_b", "a_B"]);
    }
}
