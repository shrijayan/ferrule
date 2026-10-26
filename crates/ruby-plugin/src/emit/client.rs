//! `lib/<gem>/client.rb` and its `rbi`/`sig` views: the client's defaults, its options
//! (`client_settings.opts`), how it authenticates, and an attribute per top-level resource.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{
    ApiKeyLocation, AuthRole, ClientOpt, ClientSettings, HttpAuthScheme, OptType, SecurityScheme,
};

use super::{Gen, View};
use crate::doc::{assign, print_rb, text};
use crate::printer::{COMMENT_WIDTH, comment, float, quote, wrap};

/// How the client authenticates: `auth_headers` or `auth_query`, and the hash it returns.
pub struct Auth {
    /// `auth_query` or `auth_headers`.
    pub method: &'static str,
    /// The method's body lines (`.rb`).
    pub body: Vec<String>,
}

fn opt_name(o: &ClientOpt) -> String {
    o.name.written.clone()
}

fn opt_scheme<'s>(s: &'s ClientSettings, o: &ClientOpt) -> Option<&'s SecurityScheme> {
    let auth = o.auth.as_ref()?;
    s.security_schemes
        .iter()
        .find(|n| n.name == auth.security_scheme)
        .map(|n| &n.scheme)
}

/// The authentication of the client, from the options that carry a security scheme.
#[must_use]
pub fn auth(g: &Gen<'_>) -> Option<Auth> {
    let s = g.client.settings.as_ref()?;
    let mut query = Vec::new();
    let mut headers = Vec::new();
    let mut basic: (Option<String>, Option<String>) = (None, None);
    for o in &s.opts {
        let name = opt_name(o);
        match (opt_scheme(s, o), o.auth.as_ref().map(|a| a.role)) {
            (Some(SecurityScheme::ApiKey { location, name: wire }), _) => {
                let entry = format!("{} => @{name}", quote(wire));
                if *location == ApiKeyLocation::Query {
                    query.push(entry);
                } else {
                    headers.push(entry);
                }
            }
            (
                Some(SecurityScheme::Http {
                    scheme: HttpAuthScheme::Basic,
                    ..
                }),
                Some(AuthRole::Password),
            ) => {
                basic.1 = Some(name);
            }
            (
                Some(SecurityScheme::Http {
                    scheme: HttpAuthScheme::Basic,
                    ..
                }),
                _,
            ) => basic.0 = Some(name),
            (Some(SecurityScheme::Http { .. }), _) => {
                headers.push(format!("\"authorization\" => \"Bearer #{{@{name}}}\""));
            }
            _ => {}
        }
    }
    if let (Some(user), Some(password)) = basic {
        return Some(Auth {
            method: "auth_headers",
            body: vec![
                format!("return {{}} if @{user}.nil? || @{password}.nil?"),
                String::new(),
                format!("base64_credentials = [\"#{{@{user}}}:#{{@{password}}}\"].pack(\"m0\")"),
                "{\"authorization\" => \"Basic #{base64_credentials}\"}".to_owned(),
            ],
        });
    }
    if !query.is_empty() {
        return Some(Auth {
            method: "auth_query",
            body: vec![format!("{{{}}}", query.join(", "))],
        });
    }
    (!headers.is_empty()).then(|| Auth {
        method: "auth_headers",
        body: vec![format!("{{{}}}", headers.join(", "))],
    })
}

fn yard_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        OptType::String => "String",
        OptType::Number => "Float",
        OptType::Integer => "Integer",
        OptType::Boolean => "Boolean",
    }
}

fn rbi_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        OptType::String => "String",
        OptType::Number => "Float",
        OptType::Integer => "Integer",
        OptType::Boolean => "T::Boolean",
    }
}

fn rbs_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        OptType::String => "String",
        OptType::Number => "Float",
        OptType::Integer => "Integer",
        OptType::Boolean => "bool",
    }
}

/// The default of an option's parameter: its environment variable, with the config's default.
fn opt_default(o: &ClientOpt) -> String {
    let fallback = o.default.as_ref().map(|d| match d {
        serde_json::Value::String(s) => quote(s),
        other => other.to_string(),
    });
    match (&o.read_env, fallback) {
        (Some(env), Some(d)) => format!("ENV.fetch({}, {d})", quote(env)),
        (Some(env), None) => format!("ENV[{}]", quote(env)),
        (None, Some(d)) => d,
        (None, None) => "nil".to_owned(),
    }
}

/// A parameter's description: the option's own text, then where it defaults from.
fn opt_doc(o: &ClientOpt) -> String {
    let mut d = o.description.clone().unwrap_or_default().trim().to_owned();
    if let Some(env) = &o.read_env {
        if !d.is_empty() {
            d.push(' ');
        }
        let _ = write!(d, "Defaults to `ENV[{}]`", quote(env));
    }
    d
}

/// The `initialize` parameters after the options, with their docs and Sorbet types.
struct Fixed {
    name: &'static str,
    doc: String,
    yard: &'static str,
    rbi: &'static str,
    rbs: &'static str,
    default: String,
    rbi_default: String,
}

fn fixed(g: &Gen<'_>, s: &ClientSettings) -> Vec<Fixed> {
    let m = &g.m;
    let constant = |c: &str| (format!("self.class::{c}"), format!("{m}::Client::{c}"));
    let (retries, retries_rbi) = constant("DEFAULT_MAX_RETRIES");
    let (timeout, timeout_rbi) = constant("DEFAULT_TIMEOUT_IN_SECONDS");
    let (initial, initial_rbi) = constant("DEFAULT_INITIAL_RETRY_DELAY");
    let (max, max_rbi) = constant("DEFAULT_MAX_RETRY_DELAY");
    vec![
        Fixed {
            name: "base_url",
            doc: format!(
                "Override the default base URL for the API, e.g., `\"https://api.example.com/v2/\"`. Defaults to `ENV[\"{}_BASE_URL\"]`",
                s.env_prefix
            ),
            yard: "String, nil",
            rbi: "T.nilable(String)",
            rbs: "String?",
            default: format!("ENV[\"{}_BASE_URL\"]", s.env_prefix),
            rbi_default: format!("ENV[\"{}_BASE_URL\"]", s.env_prefix),
        },
        Fixed {
            name: "max_retries",
            doc: "Max number of retries to attempt after a failed retryable request.".to_owned(),
            yard: "Integer",
            rbi: "Integer",
            rbs: "Integer",
            default: retries,
            rbi_default: retries_rbi,
        },
        Fixed {
            name: "timeout",
            doc: String::new(),
            yard: "Float",
            rbi: "Float",
            rbs: "Float",
            default: timeout,
            rbi_default: timeout_rbi,
        },
        Fixed {
            name: "initial_retry_delay",
            doc: String::new(),
            yard: "Float",
            rbi: "Float",
            rbs: "Float",
            default: initial,
            rbi_default: initial_rbi,
        },
        Fixed {
            name: "max_retry_delay",
            doc: String::new(),
            yard: "Float",
            rbi: "Float",
            rbs: "Float",
            default: max,
            rbi_default: max_rbi,
        },
    ]
}

/// A YARD `@param` whose description wraps at 80 columns after the tag.
fn param_doc(out: &mut String, pad: &str, name: &str, ty: &str, doc: &str) {
    let lines = wrap(doc, COMMENT_WIDTH);
    match lines.split_first() {
        None => {
            let _ = writeln!(out, "{pad}# @param {name} [{ty}]");
        }
        Some((first, rest)) => {
            let _ = writeln!(out, "{pad}# @param {name} [{ty}] {first}");
            for l in rest {
                let _ = writeln!(out, "{pad}# {l}");
            }
        }
    }
}

fn default_lines(s: &ClientSettings, view: View) -> Vec<(String, String, String)> {
    let initial = s.initial_retry_delay_seconds.unwrap_or(0.5);
    let max = s.max_retry_delay_seconds.unwrap_or(8.0);
    let float_value = |v: f64| match view {
        View::Rbi => format!("T.let({}, Float)", float(v)),
        _ => float(v),
    };
    vec![
        (
            "Default max number of retries to attempt after a failed retryable request.".to_owned(),
            "DEFAULT_MAX_RETRIES".to_owned(),
            s.max_retries.to_string(),
        ),
        (
            "Default per-request timeout.".to_owned(),
            "DEFAULT_TIMEOUT_IN_SECONDS".to_owned(),
            float_value(s.timeout_seconds),
        ),
        (
            "Default initial retry delay in seconds.\nOverall delay is calculated using exponential backoff + jitter."
                .to_owned(),
            "DEFAULT_INITIAL_RETRY_DELAY".to_owned(),
            float_value(initial),
        ),
        (
            "Default max retry delay in seconds.".to_owned(),
            "DEFAULT_MAX_RETRY_DELAY".to_owned(),
            float_value(max),
        ),
    ]
}

/// The top-level resources, as the client's attributes.
fn top<'g>(g: &'g Gen<'_>) -> impl Iterator<Item = &'g super::Res<'g>> {
    g.resources.iter().filter(|r| r.depth() == 0)
}

/// The YARD block of `Client#initialize`.
fn initialize_doc(g: &Gen<'_>, s: &ClientSettings, out: &mut String) {
    out.push_str("\n    # Creates and returns a new client for interacting with the API.\n    #\n");
    let mut first = true;
    for o in &s.opts {
        if !first {
            out.push_str("    #\n");
        }
        first = false;
        param_doc(
            out,
            "    ",
            &opt_name(o),
            &format!("{}, nil", yard_type(o)),
            &opt_doc(o),
        );
    }
    for f in fixed(g, s) {
        if !first {
            out.push_str("    #\n");
        }
        first = false;
        param_doc(out, "    ", f.name, f.yard, &f.doc);
    }
}

fn rb(g: &Gen<'_>, s: &ClientSettings) -> String {
    let m = &g.m;
    let mut out = String::from("# frozen_string_literal: true\n\n");
    let _ = writeln!(out, "module {m}");
    let _ = writeln!(out, "  class Client < {m}::Internal::Transport::BaseClient");
    for (i, (doc, name, value)) in default_lines(s, View::Rb).into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for l in doc.lines() {
            let _ = writeln!(out, "    # {l}");
        }
        let _ = writeln!(out, "    {name} = {value}");
    }
    for o in &s.opts {
        out.push('\n');
        comment(
            &mut out,
            4,
            &wrap(o.description.as_deref().unwrap_or(""), COMMENT_WIDTH),
        );
        let nil = if o.nullable { ", nil" } else { "" };
        let _ = writeln!(out, "    # @return [{}{nil}]", yard_type(o));
        let _ = writeln!(out, "    attr_reader :{}", opt_name(o));
    }
    for r in top(g) {
        out.push('\n');
        let _ = writeln!(out, "    # @return [{}]", r.path(m));
        let _ = writeln!(out, "    attr_reader :{}", r.attr);
    }
    if let Some(a) = auth(g) {
        out.push('\n');
        out.push_str("    # @api private\n    #\n    # @return [Hash{String=>String}]\n");
        let _ = writeln!(out, "    private def {}", a.method);
        for l in &a.body {
            if l.is_empty() {
                out.push('\n');
            } else {
                let _ = writeln!(out, "      {l}");
            }
        }
        out.push_str("    end\n");
    }
    initialize_doc(g, s, &mut out);
    out.push_str("    def initialize(\n");
    let mut params: Vec<String> = s
        .opts
        .iter()
        .map(|o| format!("{}: {}", opt_name(o), opt_default(o)))
        .collect();
    params.extend(fixed(g, s).into_iter().map(|f| format!("{}: {}", f.name, f.default)));
    let _ = writeln!(out, "      {}", params.join(",\n      "));
    out.push_str("    )\n");
    let base = s
        .default_environment
        .as_ref()
        .and_then(|d| s.environments.iter().find(|e| &e.name.written == d))
        .or_else(|| s.environments.first())
        .map_or_else(|| g.client.base_url.clone(), |e| e.url.clone());
    let _ = writeln!(out, "      base_url ||= {}", quote(&base));
    for o in s.opts.iter().filter(|o| !o.nullable) {
        let name = opt_name(o);
        let env = o.read_env.as_deref().unwrap_or("");
        let message = format!("{name} is required, and can be set via environ: \"{env}\"");
        let _ = writeln!(out, "\n      if {name}.nil?");
        let _ = writeln!(out, "        raise ArgumentError.new({})", quote(&message));
        out.push_str("      end\n");
    }
    let _ = write!(
        out,
        "\n      headers = {{}}\n      custom_headers_env = ENV[\"{}_CUSTOM_HEADERS\"]\n",
        s.env_prefix
    );
    out.push_str(CUSTOM_HEADERS);
    out.push('\n');
    for o in &s.opts {
        let name = opt_name(o);
        let conv = if o.nullable { "&.to_s" } else { ".to_s" };
        let _ = writeln!(out, "      @{name} = {name}{conv}");
    }
    out.push_str(
        "\n      super(\n        base_url: base_url,\n        timeout: timeout,\n        max_retries: max_retries,\n        \
         initial_retry_delay: initial_retry_delay,\n        max_retry_delay: max_retry_delay,\n        headers: headers\n      )\n",
    );
    let mut first = true;
    for r in top(g) {
        if first {
            out.push('\n');
        }
        first = false;
        let value = text(format!("{}.new(client: self)", r.path(m)));
        let _ = writeln!(out, "      {}", print_rb(&assign(&format!("@{}", r.attr), value), 6));
    }
    out.push_str("    end\n  end\nend\n");
    out
}

const CUSTOM_HEADERS: &str = r#"      unless custom_headers_env.nil?
        parsed = {}
        custom_headers_env.split("\n").each do |line|
          colon = line.index(":")
          unless colon.nil?
            parsed[line[0...colon].strip] = line[(colon + 1)..].strip
          end
        end
        headers = parsed.merge(headers)
      end
"#;

fn rbi(g: &Gen<'_>, s: &ClientSettings) -> String {
    let m = &g.m;
    let mut out = String::from("# typed: strong\n\n");
    let _ = writeln!(out, "module {m}");
    let _ = writeln!(out, "  class Client < {m}::Internal::Transport::BaseClient");
    for (i, (_, name, value)) in default_lines(s, View::Rbi).into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "    {name} = {value}");
    }
    for o in &s.opts {
        out.push('\n');
        comment(
            &mut out,
            4,
            &wrap(o.description.as_deref().unwrap_or(""), COMMENT_WIDTH),
        );
        let t = if o.nullable {
            format!("T.nilable({})", rbi_type(o))
        } else {
            rbi_type(o).to_owned()
        };
        let _ = writeln!(out, "    sig {{ returns({t}) }}");
        let _ = writeln!(out, "    attr_reader :{}", opt_name(o));
    }
    for r in top(g) {
        out.push('\n');
        let _ = writeln!(out, "    sig {{ returns({}) }}", r.path(m));
        let _ = writeln!(out, "    attr_reader :{}", r.attr);
    }
    if let Some(a) = auth(g) {
        out.push('\n');
        out.push_str("    # @api private\n    sig { override.returns(T::Hash[String, String]) }\n");
        let _ = writeln!(out, "    private def {}\n    end", a.method);
    }
    out.push_str("\n    # Creates and returns a new client for interacting with the API.\n");
    let mut types: Vec<String> = s
        .opts
        .iter()
        .map(|o| format!("{}: T.nilable({})", opt_name(o), rbi_type(o)))
        .collect();
    types.extend(fixed(g, s).into_iter().map(|f| format!("{}: {}", f.name, f.rbi)));
    let _ = writeln!(
        out,
        "    sig {{ params({}).returns(T.attached_class) }}",
        types.join(", ")
    );
    out.push_str("    def self.new(\n");
    let mut args: Vec<(String, String)> = s
        .opts
        .iter()
        .map(|o| (format!("{}: {}", opt_name(o), opt_default(o)), opt_doc(o)))
        .collect();
    args.extend(
        fixed(g, s)
            .into_iter()
            .map(|f| (format!("{}: {}", f.name, f.rbi_default), f.doc)),
    );
    for (i, (a, d)) in args.iter().enumerate() {
        comment(&mut out, 6, &wrap(d, COMMENT_WIDTH));
        let comma = if i + 1 < args.len() { "," } else { "" };
        let _ = writeln!(out, "      {a}{comma}");
    }
    out.push_str("    )\n    end\n  end\nend\n");
    out
}

fn rbs(g: &Gen<'_>, s: &ClientSettings) -> String {
    let m = &g.m;
    let mut out = String::new();
    let _ = writeln!(out, "module {m}");
    let _ = writeln!(out, "  class Client < {m}::Internal::Transport::BaseClient");
    let _ = writeln!(out, "    DEFAULT_MAX_RETRIES: {}", s.max_retries);
    for c in [
        "DEFAULT_TIMEOUT_IN_SECONDS",
        "DEFAULT_INITIAL_RETRY_DELAY",
        "DEFAULT_MAX_RETRY_DELAY",
    ] {
        let _ = writeln!(out, "\n    {c}: Float");
    }
    for o in &s.opts {
        let q = if o.nullable { "?" } else { "" };
        let _ = writeln!(out, "\n    attr_reader {}: {}{q}", opt_name(o), rbs_type(o));
    }
    for r in top(g) {
        let _ = writeln!(out, "\n    attr_reader {}: {}", r.attr, r.path(m));
    }
    if let Some(a) = auth(g) {
        let _ = writeln!(out, "\n    private def {}: -> ::Hash[String, String]", a.method);
    }
    let mut args: Vec<String> = s
        .opts
        .iter()
        .map(|o| format!("?{}: {}?", opt_name(o), rbs_type(o)))
        .collect();
    args.extend(fixed(g, s).into_iter().map(|f| format!("?{}: {}", f.name, f.rbs)));
    let _ = writeln!(out, "\n    def initialize: ({}) -> void", args.join(", "));
    out.push_str("  end\nend\n");
    out
}

/// `client.rb` and its views; nothing without `x-ferrule-client`.
#[must_use]
pub fn client_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let Some(s) = g.client.settings.as_ref() else {
        return Vec::new();
    };
    vec![
        (format!("{}/client.rb", g.lib(View::Rb)), rb(g, s)),
        (format!("{}/client.rbi", g.lib(View::Rbi)), rbi(g, s)),
        (format!("{}/client.rbs", g.lib(View::Rbs)), rbs(g, s)),
    ]
}
