//! The JVM settings, read in one place ([`jvm_target`]): `targets.java` or `targets.kotlin` of
//! stainless.yml (`Client::target`: `reverse_domain`, `production_repo`), overridden by
//! [`JvmMeta`], which also carries what no config key holds (the release version).

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};

use crate::naming::{Casings, pascal, split_words, title};

/// Which Stainless target the plugin writes: both are Kotlin source with the same layout; the
/// Java target adds `Optional` overloads, `@Jvm*` annotations and `CompletableFuture`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Java,
    Kotlin,
}

impl Flavor {
    /// The stainless.yml `targets` key.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Kotlin => "kotlin",
        }
    }

    /// `Java` or `Kotlin`, as the User-Agent names the language.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            Self::Java => "Java",
            Self::Kotlin => "Kotlin",
        }
    }

    #[must_use]
    pub fn is_java(self) -> bool {
        self == Self::Java
    }
}

/// What the JVM plugin needs that the model does not carry yet. `None` fields fall back to the
/// model (see [`jvm_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JvmMeta {
    /// The root package, such as `org.onebusaway` (`targets.<java|kotlin>.reverse_domain`).
    pub package: Option<String>,
    /// The Gradle project prefix, such as `onebusaway-sdk-java` (`<organization>-<target>`).
    pub artifact: Option<String>,
    /// The SDK version (release-please's manifest).
    pub version: Option<String>,
    /// `readme.example_requests.default`: the resource path (stainless.yml keys) and method name
    /// the error-handling and params tests call. Defaults to the first method without required
    /// parameters.
    pub example: Option<(Vec<String>, String)>,
    /// The year of the LICENSE copyright line (the plugin has no clock). Defaults to 2026.
    pub year: Option<u32>,
    /// `organization.docs`, the published POM's URL.
    pub docs_url: Option<String>,
    /// `organization.contact`, the published POM's developer email.
    pub contact: Option<String>,
    /// `settings.disable_mock_tests` inverted: whether the service tests run against the mock
    /// server (`TestServerExtension`) or are disabled. Defaults to true.
    pub mock_tests: Option<bool>,
}

/// The JVM settings of one SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmTarget {
    pub flavor: Flavor,
    /// `org.onebusaway`.
    pub package: String,
    /// `onebusaway-sdk-java`.
    pub artifact: String,
    /// The class-name prefix of the client and its errors: `OnebusawaySdk`.
    pub client: String,
    /// The API's name in docs: `Onebusaway SDK`.
    pub title: String,
    /// GitHub `owner/name`, without a `#branch`.
    pub repo: String,
    /// The default branch: `production_repo`'s `#branch`, else `main`.
    pub branch: String,
    pub version: String,
    pub example: Option<(Vec<String>, String)>,
    pub year: u32,
    pub docs_url: String,
    pub contact: String,
    pub mock_tests: bool,
}

fn is_package(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|seg| {
            seg.starts_with(|c: char| c.is_ascii_lowercase())
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// The JVM settings for `flavor`: `meta` where given, else `targets.<flavor>`, else placeholders
/// from `x-ferrule-package` and `x-ferrule-client`.
///
/// # Errors
///
/// FER101 when the package is not a JVM package name.
pub fn jvm_target(client: &Client, flavor: Flavor, meta: &JvmMeta) -> Result<JvmTarget, FerruleError> {
    let settings = client.target(flavor.key());
    let option = |key: &str| {
        settings
            .and_then(|t| t.options.get(key))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    };
    let org = client
        .settings
        .as_ref()
        .map_or_else(|| client.package.name.clone(), |s| s.name.written.clone());
    let org_words = split_words(&org);
    let package = meta
        .package
        .clone()
        .or_else(|| option("reverse_domain"))
        .unwrap_or_else(|| format!("com.{}", crate::naming::package(&org_words)));
    if !is_package(&package) {
        return Err(FerruleError::new(
            FerCode::Fer101,
            format!("#/x-ferrule-targets/{}/options/reverse_domain", flavor.key()),
            format!("JVM package {package:?} is not a package name"),
            "set reverse_domain to lower-case dot-separated identifiers, such as com.acme.api",
        ));
    }
    let artifact = meta.artifact.clone().unwrap_or_else(|| {
        let mut words: Vec<String> = org_words.iter().map(|w| w.to_ascii_lowercase()).collect();
        words.push(flavor.key().to_owned());
        words.join("-")
    });
    let branch = settings
        .and_then(|t| t.production_repo.as_deref())
        .and_then(|r| r.split_once('#'))
        .map_or_else(|| "main".to_owned(), |(_, b)| b.to_owned());
    let repo = settings.and_then(|t| t.production_repo.as_deref()).map_or_else(
        || client.package.repo.clone(),
        |r| r.split_once('#').map_or(r, |(repo, _)| repo).to_owned(),
    );
    Ok(JvmTarget {
        flavor,
        package,
        artifact,
        client: pascal(&org_words, Casings(&client.casings)),
        title: title(&org_words),
        repo,
        branch,
        version: meta.version.clone().unwrap_or_else(|| "0.0.1".to_owned()),
        example: meta.example.clone(),
        year: meta.year.unwrap_or(2026),
        docs_url: meta.docs_url.clone().unwrap_or_default(),
        contact: meta.contact.clone().unwrap_or_default(),
        mock_tests: meta.mock_tests.unwrap_or(true),
    })
}
