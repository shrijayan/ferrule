//! What the Go runtime template needs from the plugin (`runtime/go`, rendered by
//! `ferrule_runtime.render_go_files`): its options and the text of its three slots, plus
//! `internal/version.go`, which the runtime leaves to the generator.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ApiKeyLocation, AuthRole, ClientOpt, OptType, SecurityScheme, SendLocation};
use ferrule_sdk_model::{FerCode, FerruleError};

use super::Gen;
use crate::naming::{exported, split_words, words};
use crate::printer::{HEADER, comment, quote};

/// `GoOptions` of the runtime renderer, field for field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub module_path: String,
    pub package_name: String,
    pub client_name: String,
    pub org_name: String,
    pub max_retries: u32,
    pub config_fields: String,
    pub config_clone: String,
    pub client_options: String,
}

impl RuntimeOptions {
    /// The options as the JSON object `ferrule generate` passes to `GoOptions(**options)`.
    #[must_use]
    pub fn to_json(&self) -> String {
        let value = serde_json::json!({
            "module_path": self.module_path,
            "package_name": self.package_name,
            "client_name": self.client_name,
            "org_name": self.org_name,
            "max_retries": self.max_retries,
            "config_fields": self.config_fields,
            "config_clone": self.config_clone,
            "client_options": self.client_options,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// `RequestConfig` aligns its fields to `RequestTimeout`, the longest runtime field: a name
/// column of 15 (`Clone` keys: 16 with the colon). A longer setting widens the column, and
/// `ferrule_runtime.render_go_files` pads the runtime's lines in the same gofmt run to match.
const CONFIG_NAME_WIDTH: usize = 15;

fn config_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        OptType::String => "*string",
        OptType::Number => "*float64",
        OptType::Integer => "*int64",
        OptType::Boolean => "*bool",
    }
}

fn go_param_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        OptType::String => "string",
        OptType::Number => "float64",
        OptType::Integer => "int64",
        OptType::Boolean => "bool",
    }
}

/// Where a client setting is sent, from its `send` or its security scheme. `WithHeader` and
/// `WithQuery` take strings, so a number or boolean setting goes through `fmt.Sprint` (no
/// oracle sends one).
///
/// # Errors
///
/// FER201 for an API key sent in a cookie, which the Go runtime has no option for.
fn apply(g: &Gen<'_>, o: &ClientOpt, field: &str) -> Result<Option<String>, FerruleError> {
    let value = if o.ty == OptType::String {
        format!("*r.{field}")
    } else {
        format!("fmt.Sprint(*r.{field})")
    };
    if let Some(send) = &o.send {
        return Ok(Some(match send.location {
            SendLocation::Query => format!("WithQuery({}, {value})", quote(&send.name)),
            _ => format!("WithHeader({}, {value})", quote(&send.name)),
        }));
    }
    let Some(auth) = &o.auth else { return Ok(None) };
    let Some(scheme) = g
        .client
        .settings
        .as_ref()
        .and_then(|s| s.security_schemes.iter().find(|s| s.name == auth.security_scheme))
    else {
        return Ok(None);
    };
    Ok(match (&scheme.scheme, auth.role) {
        // `x-stainless-auth.scheme_keyword` replaces `Bearer`.
        (SecurityScheme::Http { keyword, .. }, AuthRole::Value) => {
            let keyword = keyword.as_deref().unwrap_or("Bearer").replace('%', "%%");
            Some(format!(
                "WithHeader(\"authorization\", fmt.Sprintf({}, {value}))",
                quote(&format!("{keyword} %s"))
            ))
        }
        (SecurityScheme::ApiKey { location, name }, _) => match location {
            ApiKeyLocation::Header => Some(format!("WithHeader({}, {value})", quote(name))),
            ApiKeyLocation::Query => Some(format!("WithQuery({}, {value})", quote(name))),
            ApiKeyLocation::Cookie => {
                return Err(FerruleError::new(
                    FerCode::Fer201,
                    format!(
                        "#/components/securitySchemes/{}",
                        scheme.name.replace('~', "~0").replace('/', "~1")
                    ),
                    format!(
                        "the Go SDK cannot send the API key of client setting {} in a cookie",
                        quote(&o.name.written)
                    ),
                    format!(
                        "send it with send_in_header or send_as_query_param on client_settings.opts.{}",
                        o.name.written
                    ),
                ));
            }
        },
        _ => None,
    })
}

/// The runtime options and slot texts for this SDK.
///
/// # Errors
///
/// FER201 for a client setting the Go runtime cannot send (see [`apply`]).
pub fn runtime_options(g: &Gen<'_>, client_name: &str) -> Result<RuntimeOptions, FerruleError> {
    let mut fields = String::new();
    let mut clone = String::new();
    let mut options = String::new();
    if let Some(s) = &g.client.settings {
        for env in &s.environments {
            let name = exported(&words(&env.name), g.casings);
            let url = if env.url.ends_with('/') {
                env.url.clone()
            } else {
                format!("{}/", env.url)
            };
            let _ = write!(
                options,
                "\n// WithEnvironment{name} returns a RequestOption that sets the current\n// environment to be the {} environment. An environment specifies which base URL\n// to use by default.\nfunc WithEnvironment{name}() RequestOption {{\n\treturn requestconfig.WithDefaultBaseURL({})\n}}\n",
                quote(&env.name.written),
                quote(&url)
            );
        }
        for o in &s.opts {
            let field = exported(&split_words(&o.name.written), g.casings);
            let pad = (CONFIG_NAME_WIDTH).max(field.len() + 1);
            let _ = writeln!(fields, "\t{field:<pad$}{}", config_type(o));
            let key = format!("{field}:");
            let _ = writeln!(clone, "\t\t{key:<w$}cfg.{field},", w = pad + 1);
            options.push('\n');
            comment(
                &mut options,
                0,
                &format!(
                    "With{field} returns a RequestOption that sets the client setting {}.",
                    quote(&o.name.written)
                ),
            );
            let _ = writeln!(options, "func With{field}(value {}) RequestOption {{", go_param_type(o));
            options
                .push_str("\treturn requestconfig.PreRequestOptionFunc(func(r *requestconfig.RequestConfig) error {\n");
            if o.ty == OptType::String {
                options.push_str("\t\tif value == \"\" {\n\t\t\treturn fmt.Errorf(\"default param cannot be empty string\")\n\t\t}\n");
            }
            let _ = writeln!(options, "\t\tr.{field} = &value");
            match apply(g, o, &field)? {
                Some(call) => {
                    let _ = writeln!(options, "\t\treturn r.Apply({call})");
                }
                None => options.push_str("\t\treturn nil\n"),
            }
            options.push_str("\t})\n}\n");
        }
    }
    Ok(RuntimeOptions {
        module_path: g.target.module_path.clone(),
        package_name: g.target.package_name.clone(),
        client_name: client_name.to_owned(),
        org_name: g.org.clone(),
        max_retries: g.client.settings.as_ref().map_or(2, |s| s.max_retries),
        config_fields: fields,
        config_clone: clone,
        client_options: options,
    })
}

/// `internal/version.go`.
#[must_use]
pub fn version_file(g: &Gen<'_>) -> (String, String) {
    (
        "internal/version.go".to_owned(),
        format!(
            "{HEADER}\n\npackage internal\n\nconst PackageVersion = {} // x-release-please-version\n",
            quote(&g.target.version)
        ),
    )
}
