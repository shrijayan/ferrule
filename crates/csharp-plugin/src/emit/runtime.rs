//! What the C# runtime template needs from the plugin (`runtime/csharp`, rendered by
//! `ferrule_runtime.render_csharp_files`): its names and the credentials block of
//! `ParamsBase.AddDefaultHeaders` (the `auth_headers` slot) and of `ParamsBase.QueryString` (the
//! `auth_query` slot).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ApiKeyLocation, AuthRole, HttpAuthScheme, SecurityScheme};

use super::Gen;
use crate::naming::{Casings, pascal, words};

/// `CSharpOptions` of the runtime renderer, field for field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub namespace: String,
    pub client_name: String,
    pub env_prefix: String,
    pub auth_headers: String,
    /// The `auth_query` slot of `ParamsBase.QueryString`; empty without an API key in the query.
    pub auth_query: String,
}

impl RuntimeOptions {
    /// The options as the JSON object `ferrule generate` passes to `CSharpOptions(**options)`.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut value = serde_json::json!({
            "namespace": self.namespace,
            "client_name": self.client_name,
            "env_prefix": self.env_prefix,
            "auth_headers": self.auth_headers,
        });
        // Written only when set, so the options of an SDK without one are unchanged.
        if !self.auth_query.is_empty() {
            value["auth_query"] = serde_json::Value::String(self.auth_query.clone());
        }
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// The statements that add the credentials, in the order the client options name the schemes:
/// those of `AddDefaultHeaders`, and those of `QueryString` (an API key in the query). An API key
/// in a cookie is not sent, as in the Python plugin.
fn credentials(g: &Gen<'_>) -> (String, String) {
    let Some(s) = g.client.settings.as_ref() else {
        return (String::new(), String::new());
    };
    let casings = Casings(&g.client.casings);
    let mut out = String::new();
    let mut query = String::new();
    let mut done: Vec<&str> = Vec::new();
    for o in &s.opts {
        let Some(auth) = &o.auth else { continue };
        if done.contains(&auth.security_scheme.as_str()) {
            continue;
        }
        let Some(scheme) = s.security_schemes.iter().find(|n| n.name == auth.security_scheme) else {
            continue;
        };
        done.push(&auth.security_scheme);
        let opt = |role: AuthRole| {
            s.opts
                .iter()
                .find(|o| {
                    o.auth
                        .as_ref()
                        .is_some_and(|a| a.security_scheme == scheme.name && a.role == role)
                })
                .map(|o| pascal(&words(&o.name), casings))
        };
        let name = pascal(&words(&o.name), casings);
        match &scheme.scheme {
            SecurityScheme::Http {
                scheme: HttpAuthScheme::Basic,
                ..
            } => {
                let user = opt(AuthRole::Username).unwrap_or_else(|| name.clone());
                let password = opt(AuthRole::Password).unwrap_or_else(|| name.clone());
                let _ = write!(
                    out,
                    "        if (options.{user} != null && options.{password} != null)\n        {{\n            request.Headers.Add(\n                \"Authorization\",\n                string.Format(\n                    \"Basic {{0}}\",\n                    Convert.ToBase64String(\n                        Encoding\n                            .GetEncoding(\"ISO-8859-1\")\n                            .GetBytes(\n                                string.Format(\"{{0}}:{{1}}\", options.{user}, options.{password})\n                            )\n                    )\n                )\n            );\n        }}\n"
                );
            }
            SecurityScheme::Http { .. } => {
                let _ = write!(
                    out,
                    "        if (options.{name} != null)\n        {{\n            request.Headers.Add(\"Authorization\", string.Format(\"Bearer {{0}}\", options.{name}));\n        }}\n"
                );
            }
            SecurityScheme::ApiKey {
                location: ApiKeyLocation::Header,
                name: header,
            } => {
                let _ = write!(
                    out,
                    "        if (options.{name} != null)\n        {{\n            request.Headers.Add(\"{header}\", options.{name});\n        }}\n"
                );
            }
            SecurityScheme::ApiKey {
                location: ApiKeyLocation::Query,
                name: param,
            } => {
                let _ = write!(
                    query,
                    "        if (options.{name} != null)\n        {{\n            collection.Add(\"{param}\", options.{name});\n        }}\n"
                );
            }
            _ => {}
        }
    }
    (out, query)
}

/// The runtime's options for this SDK.
#[must_use]
pub fn runtime_options(g: &Gen<'_>) -> RuntimeOptions {
    let (auth_headers, auth_query) = credentials(g);
    RuntimeOptions {
        namespace: g.target.ns.clone(),
        client_name: g.target.client.clone(),
        env_prefix: g.target.env_prefix.clone(),
        auth_headers,
        auth_query,
    }
}
