//! The Ruby settings, read in one place ([`ruby_target`]): `targets.ruby` of stainless.yml
//! (`Client::target("ruby")`: `gem_name`, `production_repo`), overridden by [`RubyMeta`], which
//! also carries what no config key holds (the release version, the README example).

use ferrule_sdk_model::model::Client;
use ferrule_sdk_model::{FerCode, FerruleError};

use crate::naming::{Casings, class_name, snake, split_words};

/// What the Ruby plugin needs that the model does not carry yet. `None` fields fall back to the
/// model (see [`ruby_target`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RubyMeta {
    /// The gem's name, such as `onebusaway-sdk` (`targets.ruby.gem_name`).
    pub gem_name: Option<String>,
    /// The SDK version written to `lib/<gem>/version.rb` (release-please's manifest).
    pub version: Option<String>,
    /// `readme.example_requests.default`: the resource path (stainless.yml keys) and method name
    /// `client_test.rb` calls. Defaults to the first method without required parameters.
    pub example: Option<(Vec<String>, String)>,
    /// `settings.disable_mock_tests`: the generated resource tests skip themselves.
    pub disable_mock_tests: bool,
}

/// The Ruby settings of one SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyTarget {
    /// `onebusaway-sdk`: the gemspec's name.
    pub gem_name: String,
    /// `onebusaway_sdk`: `lib/<lib>.rb`, `lib/<lib>/**`, `rbi/<lib>/**`, `sig/<lib>/**`.
    pub lib: String,
    /// `OnebusawaySDK`: the top-level module.
    pub module: String,
    /// `OneBusAway/ruby-sdk`, or empty.
    pub repo: String,
    pub version: String,
    pub example: Option<(Vec<String>, String)>,
    pub disable_mock_tests: bool,
}

fn is_gem_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// The Ruby settings: `meta` where given, else `targets.ruby`, else a gem named after
/// `x-ferrule-package`.
///
/// # Errors
///
/// FER101 when the gem name is not a lower-case gem name.
pub fn ruby_target(client: &Client, meta: &RubyMeta) -> Result<RubyTarget, FerruleError> {
    let ruby = client.target("ruby");
    let gem_name = meta
        .gem_name
        .clone()
        .or_else(|| {
            ruby.and_then(|t| t.options.get("gem_name"))
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
        .or_else(|| ruby.and_then(|t| t.package_name.clone()))
        .unwrap_or_else(|| client.package.name.replace('_', "-"));
    if !is_gem_name(&gem_name) {
        return Err(FerruleError::new(
            FerCode::Fer101,
            "#/x-ferrule-targets/ruby/gem_name",
            format!("Ruby gem name {gem_name:?} is not a lower-case gem name"),
            "set targets.ruby.gem_name to lower-case letters, digits, `-` and `_`",
        ));
    }
    let words = split_words(&gem_name);
    let repo = ruby
        .and_then(|t| t.production_repo.as_deref())
        .map_or("", |r| r.split_once('#').map_or(r, |(repo, _)| repo))
        .to_owned();
    Ok(RubyTarget {
        lib: snake(&words),
        module: class_name(&words, Casings(&client.casings)),
        gem_name,
        repo,
        version: meta.version.clone().unwrap_or_else(|| "0.0.1".to_owned()),
        example: meta.example.clone(),
        disable_mock_tests: meta.disable_mock_tests,
    })
}
