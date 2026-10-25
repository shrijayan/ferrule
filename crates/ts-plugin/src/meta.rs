//! Settings of the `typescript` target that the annotated spec does not carry yet.
//!
//! The package name, repository and options come from the model (`Client::target("typescript")`,
//! `x-ferrule-targets`); the version, organization, stats and README examples are not in the
//! annotated spec yet and come in [`TsMeta`]. Every emitter reads them through [`ts_settings`]
//! only.

use ferrule_sdk_model::Client;
use serde_json::Value;

use crate::naming::{split_words, type_name};

/// The stainless.yml `targets` key of the current TypeScript layout.
pub const TARGET: &str = "typescript";

/// A request from `readme.example_requests`: the endpoint (`post /embeddings`) and its params.
#[derive(Debug, Clone, PartialEq)]
pub struct ExampleRequest {
    pub endpoint: String,
    pub params: Value,
    /// `response_property`: the response field the README prints.
    pub response_property: Option<String>,
}

/// `.stats.yml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub openapi_spec_url: String,
    pub openapi_spec_hash: String,
    pub config_hash: String,
}

/// `targets.typescript.options.mcp_server`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    pub package_name: String,
}

/// What the TypeScript project needs that the annotated spec does not carry yet.
#[derive(Debug, Clone, PartialEq)]
pub struct TsMeta {
    /// The SDK version (`package.json`, `src/version.ts`).
    pub version: String,
    /// `organization.name`, `docs`, `contact`.
    pub organization_name: Option<String>,
    pub docs_url: Option<String>,
    pub contact: Option<String>,
    pub stats: Option<Stats>,
    /// `readme.example_requests` by key (`default`, `headline`, `nested_params`).
    pub readme_examples: Vec<(String, ExampleRequest)>,
    /// `settings.license`.
    pub license: String,
    /// `settings.disable_mock_tests`: generated tests are `test.skip`.
    pub disable_mock_tests: bool,
}

impl TsMeta {
    /// Deterministic placeholders, until the model carries the target's settings.
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            version: "0.0.1".to_owned(),
            organization_name: None,
            docs_url: None,
            contact: None,
            stats: None,
            readme_examples: Vec::new(),
            license: "Apache-2.0".to_owned(),
            disable_mock_tests: false,
        }
    }

    /// The values a stainless.yml (as JSON) gives that the model does not carry; the rest stay
    /// placeholders.
    #[must_use]
    pub fn from_config(config: &Value) -> Self {
        let s = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_owned);
        let org = config.get("organization");
        let mut meta = Self::placeholder();
        meta.organization_name = s(org.and_then(|o| o.get("name")));
        meta.docs_url = s(org.and_then(|o| o.get("docs")));
        meta.contact = s(org.and_then(|o| o.get("contact")));
        let settings = config.get("settings");
        if let Some(l) = s(settings.and_then(|v| v.get("license"))) {
            meta.license = l;
        }
        meta.disable_mock_tests = settings
            .and_then(|v| v.get("disable_mock_tests"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if let Some(examples) = config
            .get("readme")
            .and_then(|r| r.get("example_requests"))
            .and_then(Value::as_object)
        {
            for (key, ex) in examples {
                if let Some(endpoint) = s(ex.get("endpoint")) {
                    meta.readme_examples.push((
                        key.clone(),
                        ExampleRequest {
                            endpoint,
                            params: ex.get("params").cloned().unwrap_or(Value::Null),
                            response_property: s(ex.get("response_property")),
                        },
                    ));
                }
            }
        }
        meta
    }
}

/// The resolved settings every emitter reads.
#[derive(Debug, Clone, PartialEq)]
pub struct TsSettings {
    /// The npm package, such as `isaacus` or `@knocklabs/mgmt`.
    pub package_name: String,
    /// GitHub `owner/repo` of the SDK, or empty.
    pub repo: String,
    pub version: String,
    /// The client class, such as `Isaacus`.
    pub client_class: String,
    /// `ISAACUS` in `ISAACUS_BASE_URL`.
    pub env_prefix: String,
    /// The organization as headings name it: `Isaacus`, `Knock Mgmt`.
    pub title: String,
    pub docs_url: Option<String>,
    pub contact: Option<String>,
    pub stats: Option<Stats>,
    pub readme_examples: Vec<(String, ExampleRequest)>,
    pub mcp_server: Option<McpServer>,
    pub license: String,
    pub disable_mock_tests: bool,
}

/// `knock mgmt` -> `Knock Mgmt`: each word's first letter upper-cased.
fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The one accessor for the target's settings (see the module docs).
#[must_use]
pub fn ts_settings(client: &Client, meta: &TsMeta) -> TsSettings {
    let target = client.target(TARGET);
    let package_name = target
        .and_then(|t| t.package_name.clone())
        .unwrap_or_else(|| client.package.name.clone());
    let repo = target
        .and_then(|t| t.production_repo.as_deref())
        .map(|r| r.split('#').next().unwrap_or_default().to_owned())
        .unwrap_or_default();
    // A target's own `options:` block is `options.options`.
    let mcp_server = target
        .and_then(|t| t.options.get("options"))
        .and_then(|o| o.get("mcp_server"))
        .and_then(|m| m.get("package_name"))
        .and_then(Value::as_str)
        .map(|p| McpServer {
            package_name: p.to_owned(),
        });
    let org = client
        .settings
        .as_ref()
        .map_or_else(|| client.package.name.clone(), |s| s.name.written.clone());
    let client_class = type_name(&split_words(&org), &client.casings);
    let env_prefix = client
        .settings
        .as_ref()
        .map_or_else(|| client_class.to_ascii_uppercase(), |s| s.env_prefix.clone());
    TsSettings {
        package_name,
        repo,
        version: meta.version.clone(),
        client_class,
        env_prefix,
        title: title_case(meta.organization_name.as_deref().unwrap_or(&org)),
        docs_url: meta.docs_url.clone(),
        contact: meta.contact.clone(),
        stats: meta.stats.clone(),
        readme_examples: meta.readme_examples.clone(),
        mcp_server,
        license: meta.license.clone(),
        disable_mock_tests: meta.disable_mock_tests,
    }
}
