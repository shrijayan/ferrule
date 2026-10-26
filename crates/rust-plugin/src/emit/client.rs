//! The client (`src/client.rs`): the `Environment` enum, the client and its builder (options
//! from `client_settings`, read from the environment when not given), the `$client` methods,
//! and the blocking client (`src/blocking.rs`), whose accessors come from the same pass with
//! the blocking mode.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{
    ApiKeyLocation, AuthRole, ClientOpt, ClientSettings, HttpAuthScheme, OptType, SecurityScheme, SendLocation,
};
use ferrule_sdk_model::{FerCode, FerruleError};
use serde_json::Value;

use super::method::{Site, TestCase};
use super::resource::{Collected, accessors, methods};
use crate::context::{Gen, Module};
use crate::doc::{doc, inline};
use crate::naming::{field as field_name, pascal};

/// Option names the builder declares itself.
const BUILDER: [&str; 6] = [
    "base_url",
    "environment",
    "timeout",
    "max_retries",
    "header",
    "http_client",
];

/// One client option as the builder has it.
struct Opt<'o> {
    config: &'o ClientOpt,
    ident: String,
    ty: &'static str,
}

impl Opt<'_> {
    /// Whether the client needs a value: not nullable and no default.
    fn required(&self) -> bool {
        !self.config.nullable && self.config.default.is_none()
    }
}

fn opts(settings: Option<&ClientSettings>) -> Vec<Opt<'_>> {
    settings
        .map(|s| s.opts.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|opt| {
            let mut ident = field_name(&opt.name.words);
            if BUILDER.contains(&ident.as_str()) {
                ident.push_str("_option");
            }
            let ty = match opt.ty {
                OptType::String => "String",
                OptType::Number => "f64",
                OptType::Integer => "i64",
                OptType::Boolean => "bool",
            };
            Opt { config: opt, ident, ty }
        })
        .collect()
}

/// The example value of a required option, for tests: `"My API Key"`.
#[must_use]
pub fn required_options(g: &Gen<'_>) -> Vec<(String, String)> {
    opts(g.client.settings.as_ref())
        .iter()
        .filter(|o| o.required())
        .map(|o| {
            let value = match o.ty {
                "String" => format!("{:?}", format!("My {}", pascal_words(&o.config.name.words))),
                "bool" => "true".to_owned(),
                "f64" => "0.0".to_owned(),
                _ => "0".to_owned(),
            };
            (o.ident.clone(), value)
        })
        .collect()
}

fn pascal_words(words: &[String]) -> String {
    words
        .iter()
        .map(|w| match w.as_str() {
            "api" | "id" | "url" => w.to_uppercase(),
            _ => pascal(std::slice::from_ref(w), "", ""),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `src/client.rs`, and the tests of the `$client` methods.
///
/// # Errors
///
/// FER201 for a client option the Rust target cannot send (`send: path` or `body`).
pub fn client_module(g: &Gen<'_>, collected: &mut Collected) -> Result<(Module, Vec<TestCase>), FerruleError> {
    let client = &g.names.client;
    let settings = g.client.settings.as_ref();
    let mut module = g.new_module(vec!["client".to_owned()]);
    module.doc = format!("The `{client}` client and its builder.");
    module.rt.extend([
        "ArrayFormat",
        "Async",
        "Config",
        "Core",
        "Error",
        "NestedFormat",
        "Result",
        "from_env",
    ]);
    let env = environments(g);
    module.items.push(env.item.clone());
    let site = Site {
        core: "&self.core",
        mode: "Async",
        lifetime: "'_",
        mode_ty: "",
        params_path: "",
    };
    let mut functions = accessors(&g.client.resources, "crate::resources::", site);
    let (calls, tests) = methods(g, &mut module, client, &g.client.client_methods, site, "", collected)?;
    functions.extend(calls);
    let mut item = format!(
        "/// A client for the API. Cloning it is cheap: clones share one connection pool.\n#[derive(Debug, Clone)]\npub struct {client} {{\n    pub(crate) core: std::sync::Arc<Core>,\n}}\n\nimpl {client} {{\n"
    );
    let _ = write!(
        item,
        "    /// A client with every option read from the environment.\n    ///\n    /// # Errors\n    ///\n    /// [`Error::Config`] when a required option is not set.\n    pub fn new() -> Result<Self> {{\n        Self::builder().build()\n    }}\n\n    /// A builder, to set options before the environment is read.\n    pub fn builder() -> {client}Builder {{\n        {client}Builder::default()\n    }}\n"
    );
    for f in &functions {
        item.push('\n');
        item.push_str(f);
    }
    item.push_str("}\n");
    module.items.push(item);
    module.items.push(builder(g, settings, &env)?);
    Ok((module, tests))
}

/// The `Environment` enum and the default base URL.
struct Environments {
    item: String,
    /// The default: `Environment::Production.base_url()`, or the spec's URL.
    default: String,
}

fn environments(g: &Gen<'_>) -> Environments {
    let envs = g
        .client
        .settings
        .as_ref()
        .map(|s| s.environments.as_slice())
        .unwrap_or_default();
    if envs.is_empty() {
        return Environments {
            item: String::new(),
            default: format!("{:?}.to_owned()", g.client.base_url),
        };
    }
    let default = g
        .client
        .settings
        .as_ref()
        .and_then(|s| s.default_environment.as_deref())
        .and_then(|d| envs.iter().position(|e| e.name.written == d))
        .unwrap_or(0);
    let names: Vec<String> = envs.iter().map(|e| pascal(&e.name.words, "E", "Default")).collect();
    let mut item = String::from(
        "/// The servers the API runs on.\n#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]\npub enum Environment {\n",
    );
    for (i, (e, name)) in envs.iter().zip(&names).enumerate() {
        let _ = writeln!(item, "    /// `{}`", inline(&e.url));
        if i == default {
            item.push_str("    #[default]\n");
        }
        let _ = writeln!(item, "    {name},");
    }
    item.push_str("}\n\nimpl Environment {\n    /// The base URL of the environment.\n    #[must_use]\n    pub fn base_url(self) -> &'static str {\n        match self {\n");
    for (e, name) in envs.iter().zip(&names) {
        let _ = writeln!(item, "            Self::{name} => {:?},", e.url);
    }
    item.push_str("        }\n    }\n}\n");
    Environments {
        item,
        default: "Environment::default().base_url().to_owned()".to_owned(),
    }
}

fn builder(g: &Gen<'_>, settings: Option<&ClientSettings>, env: &Environments) -> Result<String, FerruleError> {
    let client = &g.names.client;
    let opts = opts(settings);
    let has_envs = !env.item.is_empty();
    let mut fields = String::new();
    for o in &opts {
        let _ = writeln!(fields, "    {}: Option<{}>,", o.ident, o.ty);
    }
    let _ = write!(
        fields,
        "    base_url: Option<String>,\n{}    timeout: Option<std::time::Duration>,\n    max_retries: Option<u32>,\n    headers: Vec<(String, String)>,\n    http_client: Option<reqwest::Client>,\n",
        if has_envs {
            "    environment: Option<Environment>,\n"
        } else {
            ""
        }
    );
    let mut setters = String::new();
    for o in &opts {
        let description = o
            .config
            .description
            .clone()
            .unwrap_or_else(|| format!("Sets `{}`.", o.config.name.written));
        setters.push_str(&doc(&description, "    "));
        if let Some(var) = &o.config.read_env {
            let _ = writeln!(setters, "    ///\n    /// Read from `{}` when not set.", inline(var));
        }
        let (arg, value) = if o.ty == "String" {
            ("impl Into<String>", "value.into()")
        } else {
            (o.ty, "value")
        };
        let _ = writeln!(
            setters,
            "    pub fn {}(mut self, value: {arg}) -> Self {{\n        self.{} = Some({value});\n        self\n    }}\n",
            o.ident, o.ident
        );
    }
    let prefix = settings.map_or_else(|| screaming_name(client), |s| s.env_prefix.clone());
    let _ = write!(
        setters,
        "    /// The URL every request goes to, instead of the environment's.\n    ///\n    /// Read from `{prefix}_BASE_URL` when not set.\n    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {{\n        self.base_url = Some(base_url.into());\n        self\n    }}\n\n"
    );
    if has_envs {
        setters.push_str("    /// The environment to send requests to.\n    pub fn environment(mut self, environment: Environment) -> Self {\n        self.environment = Some(environment);\n        self\n    }\n\n");
    }
    let timeout = settings.map_or(60.0, |s| s.timeout_seconds);
    let retries = settings.map_or(2, |s| s.max_retries);
    let _ = write!(
        setters,
        "    /// How long to wait for a response; {timeout} seconds when not set.\n    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {{\n        self.timeout = Some(timeout);\n        self\n    }}\n\n    /// How many times to retry a failed request; {retries} when not set.\n    pub fn max_retries(mut self, max_retries: u32) -> Self {{\n        self.max_retries = Some(max_retries);\n        self\n    }}\n\n    /// A header to send with every request.\n    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {{\n        self.headers.push((name.into(), value.into()));\n        self\n    }}\n\n    /// The HTTP client to send requests with, instead of a new one.\n    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {{\n        self.http_client = Some(http_client);\n        self\n    }}\n\n"
    );
    let _ = write!(
        setters,
        "    /// The client.\n    ///\n    /// # Errors\n    ///\n    /// [`Error::Config`] when a required option is not set, or both `base_url` and\n    /// `environment` are.\n    pub fn build(self) -> Result<{client}> {{\n        Ok({client} {{\n            core: std::sync::Arc::new(self.core()?),\n        }})\n    }}\n\n    /// The blocking client.\n    ///\n    /// # Errors\n    ///\n    /// As [`Self::build`], and [`Error::Config`] when its runtime cannot be started.\n    #[cfg(feature = \"blocking\")]\n    pub fn build_blocking(self) -> Result<crate::blocking::{client}> {{\n        crate::blocking::{client}::from_async(self.build()?)\n    }}\n\n"
    );
    setters.push_str(&core_fn(settings, &opts, &prefix, env)?);
    Ok(format!(
        "/// Options of a [`{client}`]; each option not set is read from the environment.\n#[derive(Debug, Clone, Default)]\npub struct {client}Builder {{\n{fields}}}\n\nimpl {client}Builder {{\n{setters}}}\n"
    ))
}

fn screaming_name(client: &str) -> String {
    crate::naming::screaming(&crate::naming::split_words(client))
}

fn core_fn(
    settings: Option<&ClientSettings>,
    opts: &[Opt<'_>],
    prefix: &str,
    env: &Environments,
) -> Result<String, FerruleError> {
    // Declared first: a push right after `Vec::new()` is clippy's `vec_init_then_push`.
    let mut body = String::from(
        "        let mut headers: Vec<(String, String)> = Vec::new();\n        let mut query: Vec<(String, String)> = Vec::new();\n",
    );
    for o in opts {
        let ident = &o.ident;
        let var = o.config.read_env.as_deref();
        let given = match (o.ty, var) {
            ("String", Some(var)) => format!("from_env(self.{ident}, {var:?})"),
            (_, Some(var)) => format!("self.{ident}.or_else(|| from_env(None, {var:?}).and_then(|v| v.parse().ok()))"),
            (_, None) => format!("self.{ident}"),
        };
        let local = ident.trim_start_matches("r#");
        let value = if let Some(default) = &o.config.default {
            format!("{given}.unwrap_or({})", default_expr(default, o.ty))
        } else if o.required() {
            let hint = var.map_or_else(String::new, |v| format!(" or set {v}"));
            let message = format!("missing {}: call `{ident}`{hint}", o.config.name.written);
            format!("{given}.ok_or_else(|| Error::Config({message:?}.to_owned()))?")
        } else {
            given
        };
        let _ = writeln!(body, "        let {local} = {value};");
    }
    let base = if env.item.is_empty() {
        format!(
            "        let base_url = from_env(self.base_url, \"{prefix}_BASE_URL\").unwrap_or_else(|| {});\n",
            env.default
        )
    } else {
        format!(
            "        let base_url = match (from_env(self.base_url, \"{prefix}_BASE_URL\"), self.environment) {{\n            (Some(_), Some(_)) => {{\n                return Err(Error::Config(\"set base_url or environment, not both\".to_owned()));\n            }}\n            (Some(url), None) => url,\n            (None, environment) => environment.unwrap_or_default().base_url().to_owned(),\n        }};\n"
        )
    };
    body.push_str(&base);
    let mut sends_query = false;
    for h in settings.map(|s| s.default_headers.as_slice()).unwrap_or_default() {
        let _ = writeln!(
            body,
            "        headers.push(({:?}.to_owned(), {:?}.to_owned()));",
            h.name, h.value
        );
    }
    for o in opts {
        let local = o.ident.trim_start_matches("r#");
        let text = if o.ty == "String" {
            local.to_owned()
        } else {
            format!("{local}.to_string()")
        };
        let Some((location, statement)) = send(settings, o, &text)? else {
            continue;
        };
        sends_query |= location == SendLocation::Query;
        let optional = o.config.default.is_none() && !o.required();
        if optional {
            let _ = writeln!(
                body,
                "        if let Some({local}) = {local} {{\n    {statement}\n        }}"
            );
        } else {
            let _ = writeln!(body, "{statement}");
        }
    }
    body.push_str("        headers.extend(self.headers);\n");
    if !sends_query {
        body = body.replace("let mut query:", "let query:");
    }
    body.push_str(&config_literal(settings));
    Ok(format!("    fn core(self) -> Result<Core> {{\n{body}    }}\n"))
}

/// The statements that build the `Config` and the `Core` from the locals of `core()`.
fn config_literal(s: Option<&ClientSettings>) -> String {
    let delay = |v: Option<f64>, d: &str| {
        v.map_or_else(
            || d.to_owned(),
            |v| format!("std::time::Duration::from_secs_f64({v:?})"),
        )
    };
    let array = match s.map(|s| s.array_format) {
        Some(ferrule_sdk_model::model::ArrayFormat::Comma) => "Comma",
        Some(ferrule_sdk_model::model::ArrayFormat::Indices) => "Indices",
        Some(ferrule_sdk_model::model::ArrayFormat::Brackets) => "Brackets",
        _ => "Repeat",
    };
    let nested = match s.map(|s| s.nested_format) {
        Some(ferrule_sdk_model::model::NestedFormat::Dots) => "Dots",
        _ => "Brackets",
    };
    let idempotency = s
        .and_then(|s| s.idempotency_header.as_ref())
        .map_or_else(|| "None".to_owned(), |h| format!("Some({h:?}.to_owned())"));
    format!(
        "        let config = Config {{\n            base_url,\n            timeout: self.timeout.unwrap_or(std::time::Duration::from_secs_f64({:?})),\n            max_retries: self.max_retries.unwrap_or({}),\n            initial_retry_delay: {},\n            max_retry_delay: {},\n            headers,\n            query,\n            idempotency_header: {idempotency},\n            user_agent: concat!(env!(\"CARGO_PKG_NAME\"), \"/\", env!(\"CARGO_PKG_VERSION\")).to_owned(),\n            array_format: ArrayFormat::{array},\n            nested_format: NestedFormat::{nested},\n        }};\n        Core::new(config, self.http_client)\n",
        s.map_or(60.0, |s| s.timeout_seconds),
        s.map_or(2, |s| s.max_retries),
        delay(
            s.and_then(|s| s.initial_retry_delay_seconds),
            "crate::rt::http::INITIAL_RETRY_DELAY"
        ),
        delay(
            s.and_then(|s| s.max_retry_delay_seconds),
            "crate::rt::http::MAX_RETRY_DELAY"
        ),
    )
}

fn default_expr(default: &Value, ty: &str) -> String {
    match (default, ty) {
        (Value::String(s), _) => format!("{s:?}.to_owned()"),
        (Value::Number(n), "f64") => format!("{:?}", n.as_f64().unwrap_or_default()),
        (other, _) => other.to_string(),
    }
}

/// Where option `o` is sent, and the statement that sends its value `text`.
fn send(
    settings: Option<&ClientSettings>,
    o: &Opt<'_>,
    text: &str,
) -> Result<Option<(SendLocation, String)>, FerruleError> {
    if let Some(send) = &o.config.send {
        return match send.location {
            SendLocation::Header => Ok(Some((
                SendLocation::Header,
                format!("        headers.push(({:?}.to_owned(), {text}));", send.name),
            ))),
            SendLocation::Query => Ok(Some((
                SendLocation::Query,
                format!("        query.push(({:?}.to_owned(), {text}));", send.name),
            ))),
            SendLocation::Body | SendLocation::Path => Err(FerruleError::new(
                FerCode::Fer201,
                format!("/x-ferrule-client/opts/{}", o.config.name.written),
                format!(
                    "the Rust target cannot send client option {} in the {:?}",
                    o.config.name.written, send.location
                ),
                "send it as a header or query param (ADR 0019)",
            )),
        };
    }
    let Some(auth) = &o.config.auth else { return Ok(None) };
    let scheme = settings
        .and_then(|s| s.security_schemes.iter().find(|n| n.name == auth.security_scheme))
        .map(|n| &n.scheme);
    let header = |name: &str, value: String| {
        Some((
            SendLocation::Header,
            format!("        headers.push(({name:?}.to_owned(), {value}));"),
        ))
    };
    Ok(match (scheme, auth.role) {
        (
            Some(SecurityScheme::Http {
                scheme: HttpAuthScheme::Basic,
                ..
            }),
            AuthRole::Username,
        ) => {
            let password = settings
                .and_then(|s| {
                    s.opts.iter().find(|p| {
                        p.auth
                            .as_ref()
                            .is_some_and(|a| a.security_scheme == auth.security_scheme && a.role == AuthRole::Password)
                    })
                })
                .map(|p| field_name(&p.name.words));
            password.and_then(|p| {
                header(
                    "authorization",
                    format!(
                        "crate::rt::basic_auth(&{text}, {}.as_deref().unwrap_or_default())",
                        p.trim_start_matches("r#")
                    ),
                )
            })
        }
        (
            Some(SecurityScheme::Http {
                scheme: HttpAuthScheme::Basic,
                ..
            }),
            _,
        ) => None,
        (Some(SecurityScheme::Http { keyword, .. }), AuthRole::Value) => {
            let keyword = keyword.clone().unwrap_or_else(|| "Bearer".to_owned());
            token(&keyword, &o.config.name.written)?;
            header("authorization", format!("format!(\"{keyword} {{{text}}}\")"))
        }
        (
            Some(SecurityScheme::ApiKey {
                location: ApiKeyLocation::Header,
                name,
            }),
            _,
        ) => header(name, text.to_owned()),
        (
            Some(SecurityScheme::ApiKey {
                location: ApiKeyLocation::Query,
                name,
            }),
            _,
        ) => Some((
            SendLocation::Query,
            format!("        query.push(({name:?}.to_owned(), {text}));"),
        )),
        (
            Some(SecurityScheme::ApiKey {
                location: ApiKeyLocation::Cookie,
                name,
            }),
            _,
        ) => {
            token(name, &o.config.name.written)?;
            header("cookie", format!("format!(\"{name}={{{text}}}\")"))
        }
        _ => None,
    })
}

/// Checks that `text`, written into a generated string, is an HTTP token (RFC 9110): letters,
/// digits and ``!#$%&'*+-.^_`|~``, so it needs no escaping.
fn token(text: &str, opt: &str) -> Result<(), FerruleError> {
    let ok = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c));
    if ok {
        return Ok(());
    }
    Err(FerruleError::new(
        FerCode::Fer101,
        format!("/x-ferrule-client/opts/{opt}"),
        format!("{text:?} is not an HTTP token"),
        "use an auth scheme keyword or cookie name of letters, digits and !#$%&'*+-.^_`|~",
    ))
}

/// `src/blocking.rs`: the blocking client, with the same accessors in the blocking mode.
///
/// # Errors
///
/// A `$client` method's FER diagnostic.
pub fn blocking_module(g: &Gen<'_>) -> Result<String, FerruleError> {
    let client = &g.names.client;
    let site = Site {
        core: "&self.inner.core",
        mode: "Blocking(&self.runtime)",
        lifetime: "'_",
        mode_ty: ", Blocking<'_>",
        params_path: "crate::client::",
    };
    let mut functions = accessors(&g.client.resources, "crate::resources::", site);
    let mut scratch = g.new_module(vec!["client".to_owned()]);
    let (calls, _) = methods(
        g,
        &mut scratch,
        client,
        &g.client.client_methods,
        site,
        "",
        &mut Collected::default(),
    )?;
    functions.extend(calls);
    let mut out = format!(
        "//! The blocking client (`blocking` feature): the same resources and requests, whose `send`\n//! blocks on a runtime the client owns.\n\nuse crate::rt::{{Blocking, Result, blocking_runtime}};\n\n/// A client whose calls block until they are done.\n#[derive(Debug)]\npub struct {client} {{\n    inner: crate::{client},\n    runtime: tokio::runtime::Runtime,\n}}\n\nimpl {client} {{\n    /// A client with every option read from the environment.\n    ///\n    /// # Errors\n    ///\n    /// [`crate::Error::Config`] when a required option is not set or the runtime cannot be\n    /// started.\n    pub fn new() -> Result<Self> {{\n        crate::{client}::builder().build_blocking()\n    }}\n\n    /// A blocking client that shares `inner`'s settings and connection pool.\n    ///\n    /// # Errors\n    ///\n    /// [`crate::Error::Config`] when the runtime cannot be started.\n    pub fn from_async(inner: crate::{client}) -> Result<Self> {{\n        Ok(Self {{\n            inner,\n            runtime: blocking_runtime()?,\n        }})\n    }}\n"
    );
    for f in &functions {
        out.push('\n');
        out.push_str(
            &f.replace("Request<'_, ", "crate::rt::Request<'_, ")
                .replace("Request::new(", "crate::rt::Request::new("),
        );
    }
    out.push_str("}\n");
    Ok(out)
}
