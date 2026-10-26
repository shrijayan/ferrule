//! The Rust target's settings (ADR 0019): `targets.rust` of a `ferrule.yml`, never of the
//! stainless.yml, whose schema has no Rust target.

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};
use serde_json::Value;

use crate::naming::{pascal, split_words};

/// `targets.rust` of a `ferrule.yml`; every key is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RustMeta {
    /// The crate's name; else `organization.name`.
    pub crate_name: Option<String>,
    /// The crate's version; else the spec's `info.version`.
    pub version: Option<String>,
    /// GitHub `owner/repo` of the crate, for its `repository` and README.
    pub repository: Option<String>,
    /// The crate's SPDX `license`; else none is declared.
    pub license: Option<String>,
}

impl RustMeta {
    /// The settings of a parsed `ferrule.yml`.
    ///
    /// # Errors
    ///
    /// FER101 when `targets.rust` or one of its keys has the wrong shape, or a key is unknown.
    pub fn from_config(config: &Value) -> Result<Self, FerruleError> {
        let Some(rust) = config.get("targets").and_then(|t| t.get("rust")) else {
            return Ok(Self::default());
        };
        let Some(fields) = rust.as_object() else {
            return Err(shape("/targets/rust", "is not a mapping"));
        };
        let mut meta = Self::default();
        for (key, value) in fields {
            let pointer = format!("/targets/rust/{key}");
            let text = value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| shape(&pointer, "is not a string"))?;
            match key.as_str() {
                "crate_name" => meta.crate_name = Some(text),
                "version" => meta.version = Some(text),
                "repository" => meta.repository = Some(text),
                "license" => meta.license = Some(text),
                _ => {
                    return Err(FerruleError::new(
                        FerCode::Fer101,
                        pointer,
                        format!("unknown key {key}"),
                        "use crate_name, version, repository or license (ADR 0019)",
                    ));
                }
            }
        }
        Ok(meta)
    }
}

fn shape(pointer: &str, what: &str) -> FerruleError {
    FerruleError::new(
        FerCode::Fer101,
        pointer,
        format!("{pointer} {what}"),
        "write targets.rust as a mapping of strings: crate_name, version, repository, license",
    )
}

/// The names one generation uses, resolved from the model and [`RustMeta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    /// `onebusaway-sdk`
    pub crate_name: String,
    /// `onebusaway_sdk`
    pub lib: String,
    /// `OnebusawaySdk`
    pub client: String,
    pub version: String,
    pub repository: Option<String>,
    pub license: Option<String>,
}

impl Names {
    /// The names of `client`'s crate.
    ///
    /// # Errors
    ///
    /// FER101 when the crate name has no letters or digits.
    pub fn of(client: &Client, meta: &RustMeta) -> Result<Self, FerruleError> {
        let organization = client
            .settings
            .as_ref()
            .map_or_else(|| client.package.name.clone(), |s| s.name.written.clone());
        let written = meta.crate_name.clone().unwrap_or_else(|| organization.clone());
        let crate_name = split_words(&written).join("-");
        if crate_name.is_empty() {
            return Err(FerruleError::new(
                FerCode::Fer101,
                "/targets/rust/crate_name",
                format!("crate name {written:?} has no letters or digits"),
                "set targets.rust.crate_name in ferrule.yml",
            ));
        }
        let lib = crate_name.replace('-', "_");
        let client_words = split_words(&organization);
        let client_name = pascal(&client_words, "Api", "Client");
        let version = semver(meta.version.as_deref().unwrap_or(&client.version));
        Ok(Self {
            crate_name,
            lib,
            client: client_name,
            version,
            repository: meta.repository.clone(),
            license: meta.license.clone(),
        })
    }
}

/// `version` as a semver `MAJOR.MINOR.PATCH`, missing parts zero (`1.2` -> `1.2.0`); `0.1.0` when
/// it does not start with a number.
#[must_use]
pub fn semver(version: &str) -> String {
    let core = version.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next().unwrap_or_default();
    let numbers: Vec<&str> = core
        .split('.')
        .take(3)
        .take_while(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        .collect();
    if numbers.is_empty() {
        return "0.1.0".to_owned();
    }
    let mut parts: Vec<String> = numbers.iter().map(|n| n.trim_start_matches('0').to_owned()).collect();
    for p in &mut parts {
        if p.is_empty() {
            "0".clone_into(p);
        }
    }
    parts.resize(3, "0".to_owned());
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_targets_rust() {
        let config = json!({"targets": {"rust": {"crate_name": "acme", "version": "1.2.3"}}});
        let meta = RustMeta::from_config(&config).expect("valid");
        assert_eq!(meta.crate_name.as_deref(), Some("acme"));
        assert_eq!(meta.version.as_deref(), Some("1.2.3"));
        assert_eq!(RustMeta::from_config(&json!({})), Ok(RustMeta::default()));
    }

    #[test]
    fn rejects_unknown_keys_and_shapes() {
        let err = RustMeta::from_config(&json!({"targets": {"rust": {"edition": "2021"}}})).expect_err("unknown");
        assert_eq!(
            (err.code, err.pointer.as_str()),
            (FerCode::Fer101, "/targets/rust/edition")
        );
        let err = RustMeta::from_config(&json!({"targets": {"rust": {"version": 1}}})).expect_err("shape");
        assert_eq!(err.pointer, "/targets/rust/version");
        assert!(RustMeta::from_config(&json!({"targets": {"rust": []}})).is_err());
    }

    #[test]
    fn normalizes_versions() {
        assert_eq!(semver("0.0.2"), "0.0.2");
        assert_eq!(semver("v1.2"), "1.2.0");
        assert_eq!(semver("2024-01-01"), "2024.0.0");
        assert_eq!(semver("1.02.3-beta"), "1.2.3");
        assert_eq!(semver("latest"), "0.1.0");
    }
}
