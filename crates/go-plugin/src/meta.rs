//! The Go settings, read in one place ([`go_target`]): `targets.go` of stainless.yml
//! (`Client::target("go")`: `package_name`, `production_repo`), overridden by [`GoMeta`], which
//! also carries what no config key holds (the release version, the README example).

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};

/// What the Go plugin needs that the model does not carry yet. `None` fields fall back to the
/// model (see [`go_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GoMeta {
    /// The `go.mod` module path, such as `github.com/OneBusAway/go-sdk`
    /// (`github.com/<targets.go.production_repo>`).
    pub module_path: Option<String>,
    /// The root package, such as `onebusaway` (`targets.go.package_name`).
    pub package_name: Option<String>,
    /// The SDK version written to `internal/version.go` (release-please's manifest).
    pub version: Option<String>,
    /// `readme.example_requests.default`: the resource path (stainless.yml keys) and method name
    /// the README, `usage_test.go` and `client_test.go` call. Defaults to the first method without
    /// required parameters.
    pub example: Option<(Vec<String>, String)>,
}

/// The Go settings of one SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoTarget {
    pub module_path: String,
    pub package_name: String,
    pub version: String,
    pub example: Option<(Vec<String>, String)>,
}

fn is_identifier(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The Go settings: `meta` where given, else `targets.go`, else placeholders from
/// `x-ferrule-package` (its package name and a module path from its repository).
///
/// # Errors
///
/// FER101 when the package name is not a Go identifier.
pub fn go_target(client: &Client, meta: &GoMeta) -> Result<GoTarget, FerruleError> {
    let go = client.target("go");
    let package_name = meta
        .package_name
        .clone()
        .or_else(|| go.and_then(|t| t.package_name.clone()))
        .unwrap_or_else(|| client.package.name.replace('-', "_"));
    if !is_identifier(&package_name) {
        return Err(FerruleError::new(
            FerCode::Fer101,
            "#/x-ferrule-package/name",
            format!("Go package name {package_name:?} is not a Go identifier"),
            "set targets.go.package_name to a lower-case identifier",
        ));
    }
    let repo = go
        .and_then(|t| t.production_repo.as_deref())
        .map(|r| r.split_once('#').map_or(r, |(repo, _)| repo));
    let module_path = meta.module_path.clone().unwrap_or_else(|| {
        if let Some(repo) = repo {
            format!("github.com/{repo}")
        } else if client.package.repo.is_empty() {
            package_name.clone()
        } else {
            format!("github.com/{}", client.package.repo)
        }
    });
    Ok(GoTarget {
        module_path,
        package_name,
        version: meta.version.clone().unwrap_or_else(|| "0.0.1".to_owned()),
        example: meta.example.clone(),
    })
}
