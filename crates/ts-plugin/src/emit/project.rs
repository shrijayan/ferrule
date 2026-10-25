//! The project files of the SDK: package and build configuration, CI workflows, scripts, the
//! README, the runtime's tests and `.stats.yml`, rendered from `templates/project/` (the current
//! layout) or `templates/node/project/` (the `node` layout, which shares the files the two
//! layouts have in common, [`NODE_SHARED`]).
//!
//! The templates are the files of isaacus-dev/isaacus-typescript@c56af182 and, for the `node`
//! layout, OneBusAway/js-sdk@f3ca3f47 (Apache-2.0, see `NOTICE`). Modified by The Ferrule
//! Authors: per-SDK names and values replaced by `%%TOKEN%%`s; `%%if mcp%%` (an MCP server
//! package, from knock-mgmt-node's output without one), `%%if oidc%%` (npm publishing by OIDC,
//! from knock-mgmt-node@ca88fdba), `%%if mock%%` (resource tests against the mock server:
//! `scripts/mock` and its lines in `scripts/test` and `CONTRIBUTING.md`, from
//! OneBusAway/js-sdk@f3ca3f47), `%%if qs%%`, `%%if attribution%%` (`readme.
//! include_stainless_attribution`) and `%%if query_auth%%` (an API key sent in the query);
//! Stainless branding replaced as `BRANDING.md` lists.

use std::fmt::Write as _;

use serde_json::Value;

use super::File;
use super::template::render;
use crate::ctx::Ctx;
use crate::doc::{concat, text};
use crate::examples::json_expr;
use crate::js;
use crate::naming::{camel_case, words_of};
use crate::pretty::{Expr, const_decl, expr, render as layout};

macro_rules! templates {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("templates/project/", $path, ".tmpl")))),*]
    };
}

/// Every project template: (path in the SDK, text).
const TEMPLATES: &[(&str, &str)] = templates![
    ".devcontainer/devcontainer.json",
    ".dockerignore",
    ".github/workflows/ci.yml",
    ".github/workflows/publish-npm.yml",
    ".github/workflows/release-doctor.yml",
    ".gitignore",
    ".prettierignore",
    ".prettierrc.json",
    ".release-please-manifest.json",
    "Brewfile",
    "CONTRIBUTING.md",
    "LICENSE",
    "README.md",
    "SECURITY.md",
    "bin/check-release-environment",
    "bin/publish-npm",
    "eslint.config.mjs",
    "examples/.keep",
    "jest.config.ts",
    "package.json",
    "release-please-config.json",
    "scripts/bootstrap",
    "scripts/build",
    "scripts/build-all",
    "scripts/fast-format",
    "scripts/format",
    "scripts/lint",
    "scripts/mock",
    "scripts/publish-packages.ts",
    "scripts/test",
    "scripts/utils/attw-report.cjs",
    "scripts/utils/check-is-in-git-install.sh",
    "scripts/utils/check-version.cjs",
    "scripts/utils/fix-index-exports.cjs",
    "scripts/utils/git-swap.sh",
    "scripts/utils/make-dist-package-json.cjs",
    "scripts/utils/postprocess-files.cjs",
    "src/lib/.keep",
    "tests/base64.test.ts",
    "tests/buildHeaders.test.ts",
    "tests/form.test.ts",
    "tests/index.test.ts",
    "tests/path.test.ts",
    "tests/stringifyQuery.test.ts",
    "tests/uploads.test.ts",
    "tsc-multi.json",
    "tsconfig.build.json",
    "tsconfig.deno.json",
    "tsconfig.dist-src.json",
    "tsconfig.json",
    "yarn.lock",
];

macro_rules! node_templates {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("templates/node/project/", $path, ".tmpl")))),*]
    };
}

/// The `node` layout's own project templates.
const NODE_TEMPLATES: &[(&str, &str)] = node_templates![
    ".eslintrc.js",
    ".github/workflows/ci.yml",
    ".github/workflows/publish-npm.yml",
    ".github/workflows/release-doctor.yml",
    ".gitignore",
    ".prettierignore",
    "CONTRIBUTING.md",
    "LICENSE",
    "README.md",
    "SECURITY.md",
    "bin/check-release-environment",
    "bin/publish-npm",
    "jest.config.ts",
    "package.json",
    "release-please-config.json",
    "scripts/bootstrap",
    "scripts/build",
    "scripts/fast-format",
    "scripts/format",
    "scripts/lint",
    "scripts/mock",
    "scripts/test",
    "scripts/utils/fix-index-exports.cjs",
    "scripts/utils/make-dist-package-json.cjs",
    "scripts/utils/postprocess-files.cjs",
    "tests/form.test.ts",
    "tests/index.test.ts",
    "tests/responses.test.ts",
    "tests/stringifyQuery.test.ts",
    "tests/uploads.test.ts",
    "tsc-multi.json",
    "tsconfig.build.json",
    "tsconfig.dist-src.json",
    "tsconfig.json",
    "yarn.lock",
];

/// The current layout's templates the `node` layout writes too: the files that are the same in
/// both (onebusaway-typescript and isaacus-typescript, names aside).
pub const NODE_SHARED: &[&str] = &[
    ".devcontainer/devcontainer.json",
    ".prettierrc.json",
    ".release-please-manifest.json",
    "Brewfile",
    "examples/.keep",
    "scripts/utils/check-is-in-git-install.sh",
    "scripts/utils/check-version.cjs",
    "scripts/utils/git-swap.sh",
    "src/lib/.keep",
    "tsconfig.deno.json",
];

/// The templates of the SDK's layout: (path in the SDK, text).
fn templates(ctx: &Ctx<'_>) -> Vec<(&'static str, &'static str)> {
    let mcp = ctx.settings.mcp_server.is_some();
    if ctx.node() {
        let mut out: Vec<(&str, &str)> = NODE_TEMPLATES.to_vec();
        out.extend(TEMPLATES.iter().filter(|(p, _)| NODE_SHARED.contains(p)));
        return out;
    }
    TEMPLATES
        .iter()
        .filter(|(p, _)| mcp || !MCP_ONLY.contains(p))
        .copied()
        .collect()
}

/// Files only an SDK with an MCP server package has.
const MCP_ONLY: &[&str] = &[".dockerignore", "scripts/build-all", "scripts/publish-packages.ts"];

/// Files only an SDK whose resource tests run against the mock server has (no
/// `settings.disable_mock_tests`).
const MOCK_ONLY: &[&str] = &["scripts/mock"];

const MCP_SECTION: &str = include_str!("templates/project/readme_mcp.md.tmpl");

/// The width prettier formats README snippets at.
const README_WIDTH: usize = 100;

/// `JSON.stringify` of a value, keys in their order.
fn json(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `encodeURIComponent`.
fn uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// The README's example request: `readme.example_requests.default`, else the first method.
struct Example {
    accessor: String,
    args: Vec<Expr>,
    var: String,
    property: Option<String>,
    response: String,
    params: Option<String>,
}

fn readme_example(ctx: &Ctx<'_>) -> Option<Example> {
    let default = ctx
        .settings
        .readme_examples
        .iter()
        .find(|(k, _)| k == "default")
        .map(|(_, e)| e);
    let all = ctx.all();
    let mut found = None;
    for r in &all {
        let mut mapper = crate::types::Mapper::new(ctx, &r.module());
        for plan in super::resource::plan_methods(ctx, r, &mut mapper) {
            let endpoint = format!("{} {}", plan.m.http_method.as_str(), plan.m.path);
            let wanted = default.is_none_or(|d| d.endpoint == endpoint);
            if wanted && found.is_none() {
                let var = ctx.node().then(|| super::resource::example_var(ctx, r, &plan));
                found = Some((
                    (*r).clone(),
                    plan.m,
                    plan.name.clone(),
                    plan.response.clone(),
                    plan.params_type.clone(),
                    var,
                ));
            }
        }
    }
    let (r, m, name, response, params_type, node_var) = found?;
    let chain = ctx.chain(&r);
    let accessor = format!("{}.{name}", r.accessor(&chain));
    let params = default.map(|d| &d.params);
    let mut args: Vec<Expr> = m
        .path_params
        .iter()
        .map(|p| {
            params
                .and_then(|v| v.get(&p.field.wire_name))
                .map_or_else(|| Expr::str(&p.field.wire_name), json_expr)
        })
        .collect();
    if let Some(Value::Object(map)) = params {
        let props: Vec<(String, Expr)> = map
            .iter()
            .filter(|(k, _)| !m.path_params.iter().any(|p| &p.field.wire_name == *k))
            .map(|(k, v)| (js::property_key(k), json_expr(v)))
            .collect();
        if !props.is_empty() {
            args.push(Expr::object(props));
        }
    }
    let base = response.rsplit('.').next().unwrap_or(&response).to_owned();
    // The `node` layout names the result as the method's `@example` does.
    let var = node_var.unwrap_or_else(|| camel_case(&crate::naming::split_words(&base), &[]));
    let property = m
        .response
        .as_ref()
        .and_then(|r| ctx.client.object_of(&r.ty))
        // The `node` layout prints no property of a response that extends a model
        // (onebusaway-typescript's `ResponseWrapper`).
        .filter(|o| !ctx.node() || o.bases.is_empty())
        .and_then(|o| {
            let mapper = crate::types::Mapper::new(ctx, "");
            mapper.ordered(&o.fields).first().map(|f| f.wire_name.clone())
        });
    let property = default.and_then(|d| d.response_property.clone()).or(property);
    Some(Example {
        accessor,
        args,
        var,
        property,
        response,
        params: params_type,
    })
}

/// The README's request with an `httpAgent` per request (the `node` layout's README).
fn http_agent(ex: &Example) -> String {
    let mut args = ex.args.clone();
    args.push(Expr::Object(crate::pretty::Object {
        props: vec![(
            "httpAgent".to_owned(),
            Expr::raw("new http.Agent({ keepAlive: false })"),
        )],
        expanded: true,
    }));
    let call = Expr::Await(Box::new(Expr::Call(ex.accessor.clone(), args)));
    lines(&concat(vec![expr(&call), text(";")]))
}

fn lines(doc: &crate::doc::Doc) -> String {
    layout(doc, 0, README_WIDTH)
}

/// The README's "Request & Response types" snippet: the params typed, then the result.
fn types_snippet(ctx: &Ctx<'_>, ex: &Example) -> String {
    let client = &ctx.settings.client_class;
    match &ex.params {
        // The `node` layout types the result alone when the call has no params.
        None if ctx.node() => {
            let args: Vec<String> = ex.args.iter().map(|a| layout(&expr(a), 0, usize::MAX / 2)).collect();
            format!(
                "const {}: {client}.{} = await {}({});",
                ex.var,
                ex.response,
                ex.accessor,
                args.join(", ")
            )
        }
        Some(params) => {
            let value = ex.args.last().cloned().unwrap_or(Expr::Null);
            let mut t = lines(&const_decl(&format!("params: {client}.{params}"), &value));
            let mut pos: Vec<String> = Vec::new();
            for a in &ex.args[..ex.args.len().saturating_sub(1)] {
                pos.push(layout(&expr(a), 0, usize::MAX / 2));
            }
            pos.push("params".to_owned());
            t.push('\n');
            let _ = write!(
                t,
                "const {}: {client}.{} = await {}({});",
                ex.var,
                ex.response,
                ex.accessor,
                pos.join(", ")
            );
            t
        }
        None => String::new(),
    }
}

/// The README's code snippets, by template token.
fn readme_snippets(ctx: &Ctx<'_>) -> Vec<(&'static str, String)> {
    let client = &ctx.settings.client_class;
    let Some(ex) = readme_example(ctx) else {
        return vec![
            ("USAGE", String::new()),
            ("TYPES", String::new()),
            ("ERRORS", String::new()),
            ("REQUEST_HEAD", String::new()),
            ("RAW_RESPONSE", String::new()),
            ("HTTP_AGENT", String::new()),
            ("ACCESSOR", String::new()),
        ];
    };
    let (head, method) = ex.accessor.rsplit_once('.').unwrap_or((&ex.accessor, ""));
    let call = Expr::Await(Box::new(Expr::Call(ex.accessor.clone(), ex.args.clone())));
    let mut usage = lines(&const_decl(&ex.var, &call));
    if let Some(p) = &ex.property {
        let _ = write!(usage, "\n\nconsole.log({}.{p});", ex.var);
    }
    let types = types_snippet(ctx, &ex);
    let catch = Expr::Arrow(
        "async (err) =>".to_owned(),
        vec![
            format!("if (err instanceof {client}.APIError) {{"),
            "  console.log(err.status); // 400".to_owned(),
            "  console.log(err.name); // BadRequestError".to_owned(),
            "  console.log(err.headers); // {server: 'nginx', ...}".to_owned(),
            "} else {".to_owned(),
            "  throw err;".to_owned(),
            "}".to_owned(),
        ],
    );
    let errors_chain = Expr::Await(Box::new(Expr::Chain(
        head.to_owned(),
        vec![(method.to_owned(), ex.args.clone()), ("catch".to_owned(), vec![catch])],
    )));
    let errors = lines(&const_decl(&ex.var, &errors_chain));
    let raw_args: Vec<String> = ex.args.iter().map(|a| layout(&expr(a), 0, usize::MAX / 2)).collect();
    let request_head = if raw_args.is_empty() {
        format!("await {}({{", ex.accessor)
    } else {
        format!("await {}({}, {{", ex.accessor, raw_args.join(", "))
    };
    let as_response = Expr::Await(Box::new(Expr::Chain(
        head.to_owned(),
        vec![
            (method.to_owned(), ex.args.clone()),
            ("asResponse".to_owned(), Vec::new()),
        ],
    )));
    let with_response = Expr::Await(Box::new(Expr::Chain(
        head.to_owned(),
        vec![
            (method.to_owned(), ex.args.clone()),
            ("withResponse".to_owned(), Vec::new()),
        ],
    )));
    let mut raw = lines(&const_decl("response", &as_response));
    raw.push_str("\nconsole.log(response.headers.get('X-My-Header'));");
    raw.push_str("\nconsole.log(response.statusText); // access the underlying Response object\n\n");
    raw.push_str(&lines(&const_decl(
        &format!("{{ data: {}, response: raw }}", ex.var),
        &with_response,
    )));
    raw.push_str("\nconsole.log(raw.headers.get('X-My-Header'));");
    if let Some(p) = &ex.property {
        let _ = write!(raw, "\nconsole.log({}.{p});", ex.var);
    } else if ctx.node() {
        let _ = write!(raw, "\nconsole.log({});", ex.var);
    }
    let http_agent = http_agent(&ex);
    vec![
        ("USAGE", usage),
        ("TYPES", types),
        ("ERRORS", errors),
        ("REQUEST_HEAD", request_head),
        ("RAW_RESPONSE", raw),
        ("HTTP_AGENT", http_agent),
        ("ACCESSOR", ex.accessor),
    ]
}

/// `docs.isaacus.com` for `https://docs.isaacus.com`: the URL without its scheme and a final `/`.
fn docs_label(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

/// The mock server's `--validator-*-array-format` and `--validator-*-object-format`: how the client
/// serialises array and nested query parameters.
fn query_formats(ctx: &Ctx<'_>) -> (&'static str, &'static str) {
    use ferrule_sdk_model::model::{ArrayFormat, NestedFormat};
    let Some(cs) = ctx.client.settings.as_ref() else {
        return ("repeat", "brackets");
    };
    let array = match cs.array_format {
        ArrayFormat::Comma => "comma",
        ArrayFormat::Repeat => "repeat",
        ArrayFormat::Indices => "indices",
        ArrayFormat::Brackets => "brackets",
    };
    let object = match cs.nested_format {
        NestedFormat::Brackets => "brackets",
        NestedFormat::Dots => "dots",
    };
    (array, object)
}

/// Every project file.
#[must_use]
pub fn project_files(ctx: &Ctx<'_>) -> Vec<File> {
    let s = &ctx.settings;
    let opt = ctx
        .client
        .settings
        .as_ref()
        .and_then(|cs| cs.opts.iter().find(|o| o.auth.is_some()));
    let opt_words = opt.map(|o| words_of(&o.name)).unwrap_or_default();
    let opt_prop = ctx.member(&opt_words);
    let opt_env = opt.and_then(|o| o.read_env.clone()).unwrap_or_default();
    let placeholder = super::api_tests::placeholder(&opt_words);
    let repo_name = s.repo.rsplit('/').next().unwrap_or_default().to_owned();
    let mcp = s.mcp_server.as_ref();
    let mcp_package = mcp.map(|m| m.package_name.clone()).unwrap_or_default();
    let bundle = format!(
        "{}_api.mcpb",
        s.package_name.trim_start_matches('@').replace(['/', '-'], "_")
    );
    let cursor =
        serde_json::json!({"command": "npx", "args": ["-y", mcp_package], "env": {opt_env.clone(): placeholder}});
    let vscode = serde_json::json!({"name": mcp_package, "command": "npx", "args": ["-y", mcp_package], "env": {opt_env.clone(): placeholder}});
    let docs_url = s.docs_url.clone().unwrap_or_default();
    let contact = s.contact.clone().unwrap_or_default();
    let security_contact = s.security_contact.clone().unwrap_or_else(|| contact.clone());
    let qs = crate::emit::client::uses_qs(ctx);
    let mock = !s.disable_mock_tests;
    let query_key = opt
        .and_then(|o| o.send.as_ref())
        .filter(|send| send.location == ferrule_sdk_model::model::SendLocation::Query)
        .map(|send| send.name.clone());
    let flags = [
        ("mcp", mcp.is_some()),
        ("oidc", s.npm_oidc),
        ("qs", qs),
        ("mock", mock),
        ("attribution", s.readme_attribution),
        ("query_auth", query_key.is_some()),
    ];
    let (array_format, object_format) = query_formats(ctx);
    let eslint_globs = if mcp.is_some() {
        "'tests/**', 'examples/**', 'packages/**'"
    } else {
        "'tests/**', 'examples/**'"
    };
    let mut owned: Vec<(&str, String)> = vec![
        ("REPO", s.repo.clone()),
        ("BRANCH", s.branch.clone()),
        ("REPO_NAME", repo_name),
        ("MCP_BUNDLE", bundle),
        ("MCP_PACKAGE", mcp_package),
        ("OPT_ENV", opt_env),
        ("OPT", opt_prop),
        ("OPT_PLACEHOLDER", js::string_literal(&placeholder)),
        (
            "OPT_PLACEHOLDER_ANOTHER",
            js::string_literal(&format!("another {placeholder}")),
        ),
        ("QUERY_KEY", query_key.clone().unwrap_or_default()),
        ("BASE_URL", ctx.client.base_url.clone()),
        ("ENV", s.env_prefix.clone()),
        ("CONTACT", contact),
        ("SECURITY_CONTACT", security_contact),
        ("DOCS_URL", docs_url.clone()),
        ("DOCS_LABEL", docs_label(&docs_url)),
        ("TITLE", s.title.clone()),
        ("YEAR", s.copyright_year.to_string()),
        ("CLIENT", s.client_class.clone()),
        ("PACKAGE", s.package_name.clone()),
        ("VERSION", s.version.clone()),
        ("CI_NODE", "22".to_owned()),
        ("ARRAY_FORMAT", array_format.to_owned()),
        ("OBJECT_FORMAT", object_format.to_owned()),
        ("ESLINT_TEST_GLOBS", eslint_globs.to_owned()),
        ("MCP_CURSOR_CONFIG", base64(json(&cursor).as_bytes())),
        ("MCP_VSCODE_CONFIG", uri_component(&json(&vscode))),
    ];
    owned.extend(readme_snippets(ctx));
    let base = owned;
    let borrowed: Vec<(&str, &str)> = base.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mcp_section = if mcp.is_some() {
        render(MCP_SECTION, &borrowed, &flags)
    } else {
        String::new()
    };
    let mut with_section = base.clone();
    with_section.push(("MCP_SECTION", mcp_section));
    let vars: Vec<(&str, &str)> = with_section.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mut out: Vec<File> = templates(ctx)
        .into_iter()
        .filter(|(p, _)| mock || !MOCK_ONLY.contains(p))
        .map(|(p, t)| (p.to_owned(), render(t, &vars, &flags)))
        .collect();
    if let Some(stats) = &s.stats {
        let endpoints: usize = ctx.all().iter().map(|r| r.res.methods.len()).sum();
        out.push((
            ".stats.yml".to_owned(),
            format!(
                "configured_endpoints: {endpoints}\nopenapi_spec_url: {}\nopenapi_spec_hash: {}\nconfig_hash: {}\n",
                stats.openapi_spec_url, stats.openapi_spec_hash, stats.config_hash
            ),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_like_javascript() {
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        assert_eq!(uri_component("{\"a\":\"b c\"}"), "%7B%22a%22%3A%22b%20c%22%7D");
        assert_eq!(docs_label("https://docs.knock.app/mapi"), "docs.knock.app");
    }
}
