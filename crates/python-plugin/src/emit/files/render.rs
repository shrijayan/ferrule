//! The template language of the project files: `%%NAME%%` is replaced by `vars[NAME]`, and a line
//! starting with `%%isort%%` is a one-line `from x import a, b` whose names are re-sorted the way
//! the oracle's ruff isort config (`length-sort = true`) orders them once the tokens are filled.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

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
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownToken(token) => write!(f, "template token {token} has no value"),
            Self::NotAnImport(line) => {
                write!(f, "%%isort%% on a line that is not a one-line import: {line}")
            }
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

/// Fills `template` with `vars`.
///
/// # Errors
///
/// A token without a value, or a `%%isort%%` line that is not a one-line import.
pub fn render(template: &str, vars: &BTreeMap<&str, String>) -> Result<String, RenderError> {
    let mut filled = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find("%%") {
        filled.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some((name, n)) = token_at(rest) {
            let token = &rest[..n];
            if name == "isort" {
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
        match body.strip_prefix("%%isort%%") {
            Some(line) => out.push_str(&sort_import_line(line)?),
            None => out.push_str(body),
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
    fn ranks_constants_classes_variables() {
        let mut names = vec!["a_B", "A_b", "AB"];
        names.sort_by(|a, b| isort_compare(a, b));
        assert_eq!(names, ["AB", "A_b", "a_B"]);
    }
}
