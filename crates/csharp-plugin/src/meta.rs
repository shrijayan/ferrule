//! The C# settings, read in one place ([`csharp_target`]): `targets.csharp` of stainless.yml
//! (`Client::target("csharp")`: `package_name`, `production_repo`) and the client settings,
//! overridden by [`CSharpMeta`], which also carries what no model field holds yet (the release
//! version, `settings.license`, `organization.docs`, the copyright year).

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};

use crate::naming::{Casings, pascal, split_words, title, words};

/// What the C# plugin needs that the model does not carry yet. `None` fields fall back to the
/// model (see [`csharp_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CSharpMeta {
    /// The root namespace and project name, such as `Imagekit` (`targets.csharp.package_name`).
    pub namespace: Option<String>,
    /// The SDK version written to the project's `VersionPrefix` (release-please's manifest).
    pub version: Option<String>,
    /// `settings.license`: the package's licence expression. Defaults to `Apache-2.0`.
    pub license: Option<String>,
    /// `organization.docs`: the package's project URL.
    pub docs: Option<String>,
    /// The copyright year of `Directory.Build.props` (the plugin has no clock). Defaults to 2026.
    pub year: Option<u32>,
    /// `settings.disable_mock_tests`: the generated service tests skip themselves.
    pub disable_mock_tests: bool,
}

/// The C# settings of one SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpTarget {
    /// `Imagekit`: the root namespace, `src/<ns>/` and `src/<ns>.Tests/`.
    pub ns: String,
    /// `ImageKit`: the prefix of `ImageKitClient` and the exception classes.
    pub client: String,
    /// `Image Kit`: the organisation as the project files and docs name it.
    pub title: String,
    /// `IMAGE_KIT`: `<PREFIX>_BASE_URL` and `<PREFIX>_CUSTOM_HEADERS`.
    pub env_prefix: String,
    /// `imagekit-developer/imagekit-dotnet`, or empty.
    pub repo: String,
    pub version: String,
    pub license: String,
    pub docs: String,
    pub year: u32,
    pub disable_mock_tests: bool,
}

fn is_identifier(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_uppercase()) && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// The C# settings: `meta` where given, else `targets.csharp`, else names derived from the
/// client's name.
///
/// # Errors
///
/// FER101 when the namespace is not a C# identifier.
pub fn csharp_target(client: &Client, meta: &CSharpMeta) -> Result<CSharpTarget, FerruleError> {
    let target = client.target("csharp");
    let casings = Casings(&client.casings);
    let settings_name = client.settings.as_ref().map(|s| words(&s.name));
    let fallback = settings_name
        .clone()
        .unwrap_or_else(|| split_words(&client.package.name));
    let ns = meta.namespace.clone().unwrap_or_else(|| {
        target
            .and_then(|t| t.package_name.as_deref())
            .map_or_else(|| pascal(&fallback, casings), |n| pascal(&split_words(n), casings))
    });
    if !is_identifier(&ns) {
        return Err(FerruleError::new(
            FerCode::Fer101,
            "#/x-ferrule-targets/csharp/package_name",
            format!("C# namespace {ns:?} is not a PascalCase identifier"),
            "set targets.csharp.package_name to letters and digits, such as `Acme`",
        ));
    }
    let client_words = settings_name.unwrap_or_else(|| split_words(&ns));
    let repo = target
        .and_then(|t| t.production_repo.as_deref())
        .map_or("", |r| r.split_once('#').map_or(r, |(repo, _)| repo))
        .to_owned();
    Ok(CSharpTarget {
        client: pascal(&client_words, casings),
        title: title(&client_words),
        env_prefix: client
            .settings
            .as_ref()
            .map_or_else(|| ns.to_ascii_uppercase(), |s| s.env_prefix.clone()),
        ns,
        repo,
        version: meta.version.clone().unwrap_or_else(|| "0.0.1".to_owned()),
        license: meta.license.clone().unwrap_or_else(|| "Apache-2.0".to_owned()),
        docs: meta.docs.clone().unwrap_or_default(),
        year: meta.year.unwrap_or(2026),
        disable_mock_tests: meta.disable_mock_tests,
    })
}
