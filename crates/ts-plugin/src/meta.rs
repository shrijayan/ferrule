//! Settings of the TypeScript target that the annotated spec does not carry yet.
//!
//! Stainless has two TypeScript targets: `typescript` (the current layout) and `node` (the legacy
//! layout: `src/core.ts`, `src/_shims/**`, the client in `src/index.ts`). [`Layout`] says which
//! one is generated; the same emitters write both. The package name, repository and options come
//! from the model (`Client::target`, `x-ferrule-targets`); the version, organization, stats and
//! README examples are not in the annotated spec yet and come in [`TsMeta`]. Every emitter reads
//! them through [`ts_settings`] only.

use ferrule_sdk_model::Client;
use serde_json::Value;

use crate::naming::{split_words, type_name};

/// The stainless.yml `targets` key of the current TypeScript layout.
pub const TARGET: &str = "typescript";

/// The stainless.yml `targets` key of the legacy TypeScript layout.
pub const NODE_TARGET: &str = "node";

/// Which TypeScript layout an SDK has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// The `typescript` target: `src/client.ts`, `src/core/**`, `src/internal/**`.
    Typescript,
    /// The `node` target: `src/index.ts` holds the client, `src/core.ts` the request machinery,
    /// `src/_shims/**` the fetch shims; resources take `Core.RequestOptions`.
    Node,
}

impl Layout {
    /// The stainless.yml `targets` key.
    #[must_use]
    pub const fn target(self) -> &'static str {
        match self {
            Self::Typescript => TARGET,
            Self::Node => NODE_TARGET,
        }
    }

    /// The layout of the target the config has: `typescript` when it has that target (or
    /// neither), else `node`.
    #[must_use]
    pub fn of(client: &Client) -> Self {
        if client.target(TARGET).is_none() && client.target(NODE_TARGET).is_some() {
            Self::Node
        } else {
            Self::Typescript
        }
    }
}

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
    /// `organization.security_contact`; the contact when absent.
    pub security_contact: Option<String>,
    /// The year in `LICENSE`.
    pub copyright_year: u32,
    /// `targets.typescript.publish.npm.auth_method: oidc` (`publish` is not in the model).
    pub npm_oidc: bool,
    pub stats: Option<Stats>,
    /// `readme.example_requests` by key (`default`, `headline`, `nested_params`).
    pub readme_examples: Vec<(String, ExampleRequest)>,
    /// `settings.license`.
    pub license: String,
    /// `settings.disable_mock_tests`: generated tests are `test.skip`.
    pub disable_mock_tests: bool,
    /// `settings.sort_schema_properties` (default `true`): properties sorted by name, else in
    /// the order the spec writes them.
    pub sort_schema_properties: bool,
    /// Where stainless.yml declares each model: (`components.schemas` key, resource keys).
    /// `TypeRef.path` is the Python module path, which loses the resource of a model declared by
    /// a non-standalone resource.
    pub model_homes: Vec<(String, Vec<String>)>,
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
            security_contact: None,
            copyright_year: 2026,
            npm_oidc: false,
            stats: None,
            readme_examples: Vec::new(),
            license: "Apache-2.0".to_owned(),
            disable_mock_tests: false,
            sort_schema_properties: true,
            model_homes: Vec::new(),
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
        meta.security_contact = s(org.and_then(|o| o.get("security_contact")));
        meta.npm_oidc = [TARGET, NODE_TARGET].iter().any(|t| {
            config
                .pointer(&format!("/targets/{t}/publish/npm/auth_method"))
                .and_then(Value::as_str)
                == Some("oidc")
        });
        let settings = config.get("settings");
        if let Some(l) = s(settings.and_then(|v| v.get("license"))) {
            meta.license = l;
        }
        meta.disable_mock_tests = settings
            .and_then(|v| v.get("disable_mock_tests"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        meta.sort_schema_properties = settings
            .and_then(|v| v.get("sort_schema_properties"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if let Some(resources) = config.get("resources").and_then(Value::as_object) {
            for (key, r) in resources {
                model_homes(r, &mut vec![key.clone()], &mut meta.model_homes);
            }
        }
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

/// The `components.schemas` key a `models` entry names: `X`, `#/components/schemas/X` or
/// `{ openapi_uri: ... }`.
fn model_schema(v: &Value) -> Option<String> {
    let uri = v.as_str().or_else(|| v.get("openapi_uri").and_then(Value::as_str))?;
    Some(uri.strip_prefix("#/components/schemas/").unwrap_or(uri).to_owned())
}

/// Collects the models of resource `r` (at `keys`) and of its subresources.
fn model_homes(r: &Value, keys: &mut Vec<String>, out: &mut Vec<(String, Vec<String>)>) {
    if let Some(models) = r.get("models").and_then(Value::as_object) {
        for v in models.values() {
            if let Some(schema) = model_schema(v) {
                out.push((schema, keys.clone()));
            }
        }
    }
    if let Some(subs) = r.get("subresources").and_then(Value::as_object) {
        for (k, s) in subs {
            keys.push(k.clone());
            model_homes(s, keys, out);
            keys.pop();
        }
    }
}

/// The resolved settings every emitter reads.
#[derive(Debug, Clone, PartialEq)]
pub struct TsSettings {
    pub layout: Layout,
    /// The npm package, such as `isaacus` or `@knocklabs/mgmt`.
    pub package_name: String,
    /// GitHub `owner/repo` of the SDK, or empty.
    pub repo: String,
    /// Its default branch: `production_repo`'s `#branch`, else `main`.
    pub branch: String,
    pub version: String,
    /// The client class, such as `Isaacus`.
    pub client_class: String,
    /// `ISAACUS` in `ISAACUS_BASE_URL`.
    pub env_prefix: String,
    /// The organization as headings name it: `Isaacus`, `Knock Mgmt`.
    pub title: String,
    pub docs_url: Option<String>,
    pub contact: Option<String>,
    pub security_contact: Option<String>,
    pub copyright_year: u32,
    pub npm_oidc: bool,
    pub stats: Option<Stats>,
    pub readme_examples: Vec<(String, ExampleRequest)>,
    pub mcp_server: Option<McpServer>,
    pub license: String,
    pub disable_mock_tests: bool,
    pub sort_schema_properties: bool,
    pub model_homes: Vec<(String, Vec<String>)>,
}

/// `knock mgmt` -> `Knock Mgmt`, `onebusaway sdk` -> `Onebusaway SDK`: each word spelled as in a
/// type name (acronyms upper-cased, custom casings applied).
fn title_case(words: &[String], casings: &[ferrule_sdk_model::model::CustomCasing]) -> String {
    words
        .iter()
        .map(|w| type_name(&[w], casings))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The one accessor for the target's settings (see the module docs), for `layout`'s target.
#[must_use]
pub fn ts_settings(client: &Client, meta: &TsMeta, layout: Layout) -> TsSettings {
    let target = client.target(layout.target());
    let package_name = target
        .and_then(|t| t.package_name.clone())
        .unwrap_or_else(|| client.package.name.clone());
    let production = target.and_then(|t| t.production_repo.as_deref()).unwrap_or_default();
    let (repo, branch) = production.split_once('#').unwrap_or((production, "main"));
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
        layout,
        package_name,
        repo: repo.to_owned(),
        branch: branch.to_owned(),
        version: meta.version.clone(),
        client_class,
        env_prefix,
        title: title_case(
            &split_words(meta.organization_name.as_deref().unwrap_or(&org)),
            &client.casings,
        ),
        docs_url: meta.docs_url.clone(),
        contact: meta.contact.clone(),
        security_contact: meta.security_contact.clone(),
        copyright_year: meta.copyright_year,
        npm_oidc: meta.npm_oidc,
        stats: meta.stats.clone(),
        readme_examples: meta.readme_examples.clone(),
        mcp_server,
        license: meta.license.clone(),
        disable_mock_tests: meta.disable_mock_tests,
        sort_schema_properties: meta.sort_schema_properties,
        model_homes: meta.model_homes.clone(),
    }
}
