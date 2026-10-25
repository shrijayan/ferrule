//! The Go settings, read in one place ([`go_target`]): `targets.go` of stainless.yml
//! (`Client::target("go")`: `package_name`, `production_repo`, `options.enable_v2`), overridden
//! by [`GoMeta`], which also carries what the model does not hold (the release version, the
//! README examples, `settings.disable_mock_tests`).

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};

/// Stainless's two Go layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// `internal/param`'s `param.Field[T]` params and `JSON` metadata structs (OneBusAway).
    #[default]
    V1,
    /// `targets.go.options.enable_v2`: `packages/param`'s `param.Opt[T]`, `omitzero` tags and
    /// `packages/respjson` metadata (imagekit-go, knock-mgmt-go).
    V2,
}

impl Layout {
    /// The runtime renderer's name for it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V2 => "v2",
        }
    }
}

/// What the Go plugin needs that the model does not carry yet. `None` fields fall back to the
/// model (see [`go_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GoMeta {
    /// The `go.mod` module path, such as `github.com/OneBusAway/go-sdk`
    /// (`github.com/<targets.go.production_repo>`).
    pub module_path: Option<String>,
    /// The root package, such as `onebusaway` (`targets.go.package_name`).
    pub package_name: Option<String>,
    /// The SDK version written to `internal/version.go` (release-please's manifest). A major
    /// version of 2 or more suffixes the module path (`imagekit-go/v2`), as Go requires.
    pub version: Option<String>,
    /// `readme.example_requests.default`: the resource path (stainless.yml keys) and method name
    /// the README, `usage_test.go` and `client_test.go` call. Defaults to the first method without
    /// required parameters.
    pub example: Option<(Vec<String>, String)>,
    /// `readme.example_requests.pagination`, as for `example`: the method the pagination tests
    /// call. Defaults to the first paginated method without path params or required params.
    pub pagination_example: Option<(Vec<String>, String)>,
    /// The layout, overriding `targets.go.options.enable_v2`.
    pub layout: Option<Layout>,
    /// `settings.disable_mock_tests`: every test against the mock server starts with
    /// `t.Skip("Mock server tests are disabled")` (not in the model; Phase 1 gap list).
    pub disable_mock_tests: bool,
}

/// The Go settings of one SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoTarget {
    pub module_path: String,
    pub package_name: String,
    pub version: String,
    pub example: Option<(Vec<String>, String)>,
    pub pagination_example: Option<(Vec<String>, String)>,
    pub layout: Layout,
    pub disable_mock_tests: bool,
}

impl GoTarget {
    #[must_use]
    pub fn v2(&self) -> bool {
        self.layout == Layout::V2
    }
}

fn is_identifier(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The major version of a release (`2` for `2.5.0`), if it parses.
fn major(version: &str) -> Option<u64> {
    version.trim_start_matches('v').split('.').next()?.parse().ok()
}

/// The Go settings: `meta` where given, else `targets.go`, else placeholders from
/// `x-ferrule-package` (its package name and a module path from its repository).
///
/// `targets.go.package_name` loses its dashes, as Stainless's Go packages do (`knock-mapi` is
/// `knockmapi`, `ogx-client` is `ogxclient`).
///
/// # Errors
///
/// FER101 when the package name is not a Go identifier.
pub fn go_target(client: &Client, meta: &GoMeta) -> Result<GoTarget, FerruleError> {
    let go = client.target("go");
    let package_name = meta
        .package_name
        .clone()
        .or_else(|| go.and_then(|t| t.package_name.as_ref()).map(|p| p.replace('-', "")))
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
    let version = meta.version.clone().unwrap_or_else(|| "0.0.1".to_owned());
    let module_path = meta.module_path.clone().unwrap_or_else(|| {
        let base = if let Some(repo) = repo {
            format!("github.com/{repo}")
        } else if client.package.repo.is_empty() {
            package_name.clone()
        } else {
            format!("github.com/{}", client.package.repo)
        };
        match major(&version) {
            Some(m) if m >= 2 => format!("{base}/v{m}"),
            _ => base,
        }
    });
    let enable_v2 = go
        .and_then(|t| t.options.get("options"))
        .and_then(|o| o.get("enable_v2"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let layout = meta.layout.unwrap_or(if enable_v2 { Layout::V2 } else { Layout::V1 });
    Ok(GoTarget {
        module_path,
        package_name,
        version,
        example: meta.example.clone(),
        pagination_example: meta.pagination_example.clone(),
        layout,
        disable_mock_tests: meta.disable_mock_tests,
    })
}
