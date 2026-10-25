//! Diagnostics helpers and path segments.

use crate::{Diagnostic, Severity};
use std::fmt;

/// One segment of a path into a JSON value: an object key or an array index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Seg {
    Key(String),
    Index(usize),
}

impl fmt::Display for Seg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Seg::Key(k) => f.write_str(k),
            Seg::Index(i) => write!(f, "{i}"),
        }
    }
}

impl From<&str> for Seg {
    fn from(s: &str) -> Self {
        Seg::Key(s.to_owned())
    }
}

impl From<String> for Seg {
    fn from(s: String) -> Self {
        Seg::Key(s)
    }
}

impl From<&String> for Seg {
    fn from(s: &String) -> Self {
        Seg::Key(s.clone())
    }
}

impl From<usize> for Seg {
    fn from(i: usize) -> Self {
        Seg::Index(i)
    }
}

/// Escape one RFC 6901 reference token.
#[must_use]
pub fn escape_token(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}

/// Encode path segments as an RFC 6901 JSON pointer (`toJsonPointer`).
#[must_use]
pub fn to_json_pointer(segments: &[Seg]) -> String {
    let mut out = String::new();
    for s in segments {
        out.push('/');
        out.push_str(&escape_token(&s.to_string()));
    }
    out
}

/// Build a path from string literals and indices: `path!["resources", name, 0]`.
#[macro_export]
macro_rules! path {
    ($($seg:expr),* $(,)?) => {
        vec![$($crate::diagnostic::Seg::from($seg)),*]
    };
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        })
    }
}

impl Diagnostic {
    /// A diagnostic from its parts.
    #[must_use]
    pub fn new(
        code: &str,
        severity: Severity,
        message: impl Into<String>,
        pointer: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_owned(),
            severity,
            message: message.into(),
            pointer: pointer.into(),
            suggestion: suggestion.into(),
        }
    }

    /// An error diagnostic.
    #[must_use]
    pub fn error(
        code: &str,
        pointer: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::new(code, Severity::Error, message, pointer, suggestion)
    }

    /// A warning diagnostic.
    #[must_use]
    pub fn warning(
        code: &str,
        pointer: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::new(code, Severity::Warning, message, pointer, suggestion)
    }

    /// An info diagnostic.
    #[must_use]
    pub fn info(
        code: &str,
        pointer: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::new(code, Severity::Info, message, pointer, suggestion)
    }

    /// Whether this is an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_json_pointer_escapes_tilde_and_slash() {
        assert_eq!(
            to_json_pointer(&[
                Seg::from("paths"),
                Seg::from("/users/{id}"),
                Seg::Index(0),
                Seg::from("a~b")
            ]),
            "/paths/~1users~1{id}/0/a~0b"
        );
        assert_eq!(to_json_pointer(&[]), "");
    }
}
