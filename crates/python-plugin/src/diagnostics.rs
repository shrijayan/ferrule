//! Diagnostics with addresses: a FER code, a pointer into the spec, and a suggested fix.
//! Port of `python-plugin/src/diagnostics.ts`.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FerCode {
    /// An `x-ferrule-*` extension has the wrong shape.
    Fer101,
    /// An `x-ferrule-*` extension the plugin does not know.
    Fer102,
    /// A required `x-ferrule-*` extension is missing.
    Fer103,
    /// The spec uses something the plugin cannot generate yet.
    Fer201,
}

impl FerCode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fer101 => "FER101",
            Self::Fer102 => "FER102",
            Self::Fer103 => "FER103",
            Self::Fer201 => "FER201",
        }
    }
}

impl fmt::Display for FerCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An error the plugin reports; displays as `<code> at <pointer>: <detail>. Fix: <fix>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FerruleError {
    pub code: FerCode,
    pub pointer: String,
    pub detail: String,
    pub fix: String,
}

impl FerruleError {
    #[must_use]
    pub fn new(code: FerCode, pointer: impl Into<String>, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            code,
            pointer: pointer.into(),
            detail: detail.into(),
            fix: fix.into(),
        }
    }
}

impl fmt::Display for FerruleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}. Fix: {}",
            self.code, self.pointer, self.detail, self.fix
        )
    }
}

impl std::error::Error for FerruleError {}

/// JSON pointer of an operation: `#/paths/~1items/get`.
#[must_use]
pub fn operation_pointer(path: &str, verb: &str) -> String {
    format!(
        "#/paths/{}/{}",
        path.replace('~', "~0").replace('/', "~1"),
        verb.to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_path() {
        assert_eq!(operation_pointer("/a/{b}~c", "GET"), "#/paths/~1a~1{b}~0c/get");
    }

    #[test]
    fn displays_code_pointer_and_fix() {
        let e = FerruleError::new(FerCode::Fer103, "#/p", "missing x", "add x");
        assert_eq!(e.to_string(), "FER103 at #/p: missing x. Fix: add x");
    }
}
