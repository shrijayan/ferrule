//! What the Go runtime template needs from the plugin (`runtime/go` or `runtime/go-v2`,
//! rendered by `ferrule_runtime.render_go_files`): its options and the text of its three slots,
//! plus `internal/version.go`, which the runtime leaves to the generator.
//!
//! v2 stores client settings by value and sets them as given (imagekit-go, knock-mgmt-go): the
//! credential itself with `RequestOptionFunc`, the others with `PreRequestOptionFunc`, and HTTP
//! Basic auth from the username and password pair.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ApiKeyLocation, AuthRole, ClientOpt, OptType, SecurityScheme, SendLocation};
use ferrule_sdk_model::{FerCode, FerruleError};

use super::Gen;
use crate::meta::Layout;
use crate::naming::{exported, split_words, words};
use crate::printer::{HEADER, quote};

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
    pub layout: Layout,
    /// v2: `field.go`'s file helper is `NewFile`, because the root package declares `File`.
    pub new_file: bool,
}

impl RuntimeOptions {
    /// The options as the JSON object `ferrule generate` passes to `GoOptions(**options)`.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut value = serde_json::json!({
            "module_path": self.module_path,
            "package_name": self.package_name,
            "client_name": self.client_name,
            "org_name": self.org_name,
            "max_retries": self.max_retries,
            "config_fields": self.config_fields,
            "config_clone": self.config_clone,
            "client_options": self.client_options,
        });
        // Only what differs from `GoOptions`' defaults, so v1 options read as before.
        if self.layout == Layout::V2 {
            value["layout"] = self.layout.as_str().into();
            value["new_file"] = self.new_file.into();
        }
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// `RequestConfig` aligns its fields to `RequestTimeout`, the longest runtime field: a name
/// column of 15 (`Clone` keys: 16 with the colon). A longer setting widens the column, and
/// `ferrule_runtime.render_go_files` pads the runtime's lines in the same gofmt run to match.
const CONFIG_NAME_WIDTH: usize = 15;

/// The `RequestConfig` field type of a client setting: a pointer in v1, the value in v2.
fn config_type(o: &ClientOpt, layout: Layout) -> &'static str {
    let ty = go_param_type(o);
    if layout == Layout::V2 {
        return ty;
    }
    match ty {
        "string" => "*string",
        "float64" => "*float64",
        "int64" => "*int64",
        _ => "*bool",
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

/// v2's `Authorization: Basic` header from the scheme's username and password settings.
fn basic(g: &Gen<'_>, scheme: &str) -> Option<String> {
    let s = g.client.settings.as_ref()?;
    let field = |role: AuthRole| {
        s.opts
            .iter()
            .find(|o| {
                o.auth
                    .as_ref()
                    .is_some_and(|a| a.security_scheme == scheme && a.role == role)
            })
            .map(|o| exported(&split_words(&o.name.written), g.casings))
    };
    let (user, password) = (field(AuthRole::Username)?, field(AuthRole::Password)?);
    Some(format!(
        "WithHeader(\"authorization\", fmt.Sprintf(\"Basic %s\", base64.StdEncoding.EncodeToString([]byte(r.{user}+\":\"+r.{password}))))"
    ))
}

/// Where a client setting is sent, from its `send` or its security scheme. `WithHeader` and
/// `WithQuery` take strings, so a number or boolean setting goes through `fmt.Sprint` (no
/// oracle sends one). v1 stores the setting as a pointer, v2 as the value.
///
/// # Errors
///
/// FER201 for an API key sent in a cookie, which the Go runtime has no option for.
fn apply(g: &Gen<'_>, o: &ClientOpt, field: &str) -> Result<Option<String>, FerruleError> {
    let deref = if g.target.v2() { "" } else { "*" };
    let value = if o.ty == OptType::String {
        format!("{deref}r.{field}")
    } else {
        format!("fmt.Sprint({deref}r.{field})")
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
        (SecurityScheme::Http { .. }, AuthRole::Username | AuthRole::Password) if g.target.v2() => {
            basic(g, &auth.security_scheme)
        }
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
        let v2 = g.target.v2();
        for o in &s.opts {
            let field = exported(&split_words(&o.name.written), g.casings);
            let pad = (CONFIG_NAME_WIDTH).max(field.len() + 1);
            let _ = writeln!(fields, "\t{field:<pad$}{}", config_type(o, g.target.layout));
            let key = format!("{field}:");
            let _ = writeln!(clone, "\t\t{key:<w$}cfg.{field},", w = pad + 1);
            // Stainless does not wrap this comment (imagekit-go's is 85 characters).
            let _ = write!(
                options,
                "\n// With{field} returns a RequestOption that sets the client setting {}.\n",
                quote(&o.name.written)
            );
            let _ = writeln!(options, "func With{field}(value {}) RequestOption {{", go_param_type(o));
            let credential = o
                .auth
                .as_ref()
                .is_some_and(|a| matches!(a.role, AuthRole::Value | AuthRole::Username));
            let func = if v2 && credential {
                "RequestOptionFunc"
            } else {
                "PreRequestOptionFunc"
            };
            let _ = writeln!(
                options,
                "\treturn requestconfig.{func}(func(r *requestconfig.RequestConfig) error {{"
            );
            if !v2 && o.ty == OptType::String {
                options.push_str("\t\tif value == \"\" {\n\t\t\treturn fmt.Errorf(\"default param cannot be empty string\")\n\t\t}\n");
            }
            let _ = writeln!(options, "\t\tr.{field} = {}value", if v2 { "" } else { "&" });
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
        layout: g.target.layout,
        new_file: false,
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
