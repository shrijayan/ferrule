//! `client.go`: the `Client` struct with one field per top-level service, `DefaultClientOptions`
//! from the environment, `NewClient`, and the generic request methods.

use std::fmt::Write as _;

use ferrule_sdk_model::model::OptType;
use serde_json::Value;

use super::Gen;
use crate::naming::{exported, words};
use crate::printer::{ALIGN, comment, go_file, quote};
use crate::types::Pkg;

/// The generic request methods after `NewClient`; text from Stainless's Go v1 client (OneBusAway
/// go-sdk, Apache-2.0; see NOTICE).
const REQUEST_METHODS: &str = include_str!("client_methods.go.txt");

/// The template without its provenance line.
fn request_methods() -> &'static str {
    REQUEST_METHODS.split_once('\n').map_or("", |(_, rest)| rest)
}

/// The environment variables the client reads, as its docs list them: client settings, then the
/// base URL.
fn env_vars(g: &Gen<'_>) -> Vec<String> {
    let mut out: Vec<String> = g
        .client
        .settings
        .iter()
        .flat_map(|s| s.opts.iter().filter_map(|o| o.read_env.clone()))
        .collect();
    if let Some(s) = &g.client.settings {
        out.push(format!("{}_BASE_URL", s.env_prefix));
    }
    out
}

/// A client setting's `default` as a Go literal; `None` for a null or structured default.
fn default_literal(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(quote(s)),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// `option.WithEnvironment<Name>()` for the default environment, if any.
#[must_use]
pub fn default_environment(g: &Gen<'_>) -> Option<String> {
    let s = g.client.settings.as_ref()?;
    let env = s
        .default_environment
        .as_ref()
        .and_then(|d| s.environments.iter().find(|e| e.name.written == *d))
        .or_else(|| s.environments.first())?;
    Some(format!("WithEnvironment{}", exported(&words(&env.name), g.casings)))
}

/// `client.go`.
#[must_use]
pub fn client_file(g: &Gen<'_>) -> (String, String) {
    let mut ctx = g.ctx(Pkg::Root);
    for p in ["context", "net/http", "os", "slices", "strings"] {
        ctx.import(p);
    }
    ctx.import_module("internal/requestconfig");
    ctx.import_module("option");
    let mut out = String::new();
    comment(
        &mut out,
        0,
        &format!(
            "Client creates a struct with services and top level methods that help with interacting with the {} \
             API. You should not instantiate this client directly, and instead use the [NewClient] method instead.",
            g.org
        ),
    );
    // v2 holds the client and its services by value.
    let ptr = if g.target.v2() { "" } else { "*" };
    out.push_str("type Client struct {\n");
    let _ = writeln!(out, "\tOptions{ALIGN}[]option.RequestOption");
    let top: Vec<_> = g.services.iter().filter(|s| s.depth == 0).collect();
    for s in &top {
        let _ = writeln!(out, "\t{}{ALIGN}{ptr}{}", s.field, s.service);
    }
    out.push_str("}\n\n");

    let vars = env_vars(g).join(", ");
    default_client_options(g, &mut out, &vars);

    comment(
        &mut out,
        0,
        &format!(
            "NewClient generates a new client with the default option read from the environment ({vars}). The \
             option passed in as arguments are applied after these default arguments, and all option will be \
             passed down to the services and requests that this client makes."
        ),
    );
    let amp = if g.target.v2() { "" } else { "&" };
    let _ = write!(
        out,
        "func NewClient(opts ...option.RequestOption) (r {ptr}Client) {{\n\topts = append(DefaultClientOptions(), opts...)\n\n\tr = {amp}Client{{Options: opts}}\n\n"
    );
    for s in &top {
        let _ = writeln!(out, "\tr.{} = New{}(opts...)", s.field, s.service);
    }
    out.push_str("\n\treturn\n}\n\n");
    // v2 spells `interface{}` as `any`.
    if g.target.v2() {
        out.push_str(&request_methods().replace("interface{}", "any"));
    } else {
        out.push_str(request_methods());
    }
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    ("client.go".to_owned(), go_file(&g.target.package_name, &imports, &out))
}

/// `DefaultClientOptions`, documented with the environment variables it reads (`vars`).
fn default_client_options(g: &Gen<'_>, out: &mut String, vars: &str) {
    comment(
        out,
        0,
        &format!(
            "DefaultClientOptions read from the environment ({vars}). This should be used to initialize new clients."
        ),
    );
    out.push_str("func DefaultClientOptions() []option.RequestOption {\n");
    let mut defaults = vec!["option.WithHTTPClient(defaultHTTPClient())".to_owned()];
    if let Some(env) = default_environment(g) {
        defaults.push(format!("option.{env}()"));
    }
    let _ = writeln!(out, "\tdefaults := []option.RequestOption{{{}}}", defaults.join(", "));
    if let Some(s) = &g.client.settings {
        let _ = writeln!(
            out,
            "\tif o, ok := os.LookupEnv(\"{}_BASE_URL\"); ok {{\n\t\tdefaults = append(defaults, option.WithBaseURL(o))\n\t}}",
            s.env_prefix
        );
        // Configured defaults, before the environment and the caller's options (imagekit-go's
        // `option.WithPassword("do_not_set")`); default headers after them (no oracle has one).
        for o in &s.opts {
            if let Some(value) = o.default.as_ref().and_then(default_literal) {
                let _ = writeln!(
                    out,
                    "\tdefaults = append(defaults, option.With{}({value}))",
                    exported(&words(&o.name), g.casings)
                );
            }
        }
        for h in &s.default_headers {
            let _ = writeln!(
                out,
                "\tdefaults = append(defaults, option.WithHeader({}, {}))",
                quote(&h.name),
                quote(&h.value)
            );
        }
        for o in &s.opts {
            if let Some(env) = &o.read_env
                && o.ty == OptType::String
            {
                let _ = writeln!(
                    out,
                    "\tif o, ok := os.LookupEnv(\"{env}\"); ok {{\n\t\tdefaults = append(defaults, option.With{}(o))\n\t}}",
                    exported(&words(&o.name), g.casings)
                );
            }
        }
        let _ = writeln!(
            out,
            "\tif o, ok := os.LookupEnv(\"{}_CUSTOM_HEADERS\"); ok {{\n\t\tfor _, line := range strings.Split(o, \"\\n\") {{\n\t\t\tcolon := strings.Index(line, \":\")\n\t\t\tif colon >= 0 {{\n\t\t\t\tdefaults = append(defaults, option.WithHeader(strings.TrimSpace(line[:colon]), strings.TrimSpace(line[colon+1:])))\n\t\t\t}}\n\t\t}}\n\t}}",
            s.env_prefix
        );
    }
    out.push_str("\treturn defaults\n}\n\n");
}
