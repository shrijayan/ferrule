//! The PHP settings, read in one place ([`php_target`]): `targets.php` of stainless.yml
//! (`Client::target("php")`: `package_name`, `composer_package_name`, `production_repo`),
//! overridden by [`PhpMeta`], which also carries what no config key holds (the release version,
//! the client test's example request).

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};

use crate::naming::{Casings, class_name, split_words};

/// What the PHP plugin needs that the model does not carry yet. `None` fields fall back to the
/// model (see [`php_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhpMeta {
    /// The root namespace, such as `Imagekit` (from `targets.php.package_name`).
    pub namespace: Option<String>,
    /// The Composer package, such as `imagekit/imagekit` (`targets.php.composer_package_name`).
    pub composer_name: Option<String>,
    /// The SDK version (release-please's manifest).
    pub version: Option<String>,
    /// `readme.example_requests.default`: the resource path (stainless.yml keys) and method name
    /// `tests/ClientTest.php` calls. Defaults to the first method without required parameters.
    pub example: Option<(Vec<String>, String)>,
    /// `settings.disable_mock_tests`: the generated service tests skip themselves.
    pub disable_mock_tests: bool,
}

/// The PHP settings of one SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhpTarget {
    /// `Imagekit`: the PSR-4 prefix of `src/`.
    pub namespace: String,
    /// `imagekit/imagekit`.
    pub composer_name: String,
    /// `imagekit-developer/imagekit-php`, or empty.
    pub repo: String,
    pub version: String,
    pub example: Option<(Vec<String>, String)>,
    pub disable_mock_tests: bool,
}

fn is_namespace(s: &str) -> bool {
    !s.is_empty()
        && s.split('\\').all(|seg| {
            seg.starts_with(|c: char| c.is_ascii_uppercase()) && seg.chars().all(|c| c.is_ascii_alphanumeric())
        })
}

/// The PHP settings: `meta` where given, else `targets.php`, else a namespace named after
/// `x-ferrule-package`.
///
/// # Errors
///
/// FER101 when the namespace is not a PHP namespace.
pub fn php_target(client: &Client, meta: &PhpMeta) -> Result<PhpTarget, FerruleError> {
    let php = client.target("php");
    let package = php
        .and_then(|t| t.package_name.clone())
        .unwrap_or_else(|| client.package.name.clone());
    let namespace = meta.namespace.clone().unwrap_or_else(|| {
        package
            .split(['\\', '/'])
            .map(|seg| class_name(&split_words(seg), Casings(&client.casings)))
            .collect::<Vec<_>>()
            .join("\\")
    });
    if !is_namespace(&namespace) {
        return Err(FerruleError::new(
            FerCode::Fer101,
            "#/x-ferrule-targets/php/package_name",
            format!("PHP namespace {namespace:?} is not a namespace of PascalCase segments"),
            "set targets.php.package_name to letters and digits, such as `acme`",
        ));
    }
    let composer_name = meta
        .composer_name
        .clone()
        .or_else(|| {
            php.and_then(|t| t.options.get("composer_package_name"))
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| {
            let lower = package.to_ascii_lowercase().replace('_', "-");
            format!("{lower}/{lower}")
        });
    let repo = php
        .and_then(|t| t.production_repo.as_deref())
        .map_or("", |r| r.split_once('#').map_or(r, |(repo, _)| repo))
        .to_owned();
    Ok(PhpTarget {
        namespace,
        composer_name,
        repo,
        version: meta.version.clone().unwrap_or_else(|| "0.0.1".to_owned()),
        example: meta.example.clone(),
        disable_mock_tests: meta.disable_mock_tests,
    })
}
