//! `src/client.ts`: the client class. The options, constructor and auth come from the client
//! settings; the request machinery between `stringifyQuery` and `buildBody` is the fixed fragment
//! every current-layout Stainless SDK shares (`templates/client_tail.ts.tmpl`); then the error
//! statics, the resources and the `declare namespace`. The `node` layout's client is
//! `src/index.ts` ([`emit_node_index`]): a subclass of `Core.APIClient` (the request machinery is
//! the runtime's `src/core.ts`), followed by the SDK's exports.
//!
//! The templates are text of isaacus-dev/isaacus-typescript@c56af182 and, for the pagination
//! methods, knocklabs/knock-mgmt-node@ca88fdba; the `node` templates (`templates/node/index_*`)
//! of OneBusAway/js-sdk@f3ca3f47 (Apache-2.0, see `NOTICE`). Modified by The Ferrule Authors:
//! per-SDK names and values replaced by `%%TOKEN%%`s, pagination behind `%%if pagination%%`, an
//! API key sent in the query behind `%%if query_auth%%`.

use ferrule_sdk_model::model::{ClientOpt, SecurityScheme, SendLocation};

use super::resource::exports_of;
use super::template::render as fill;
use super::{File, HEADER};
use crate::ctx::{Ctx, TsResource};
use crate::doc::{concat, text};
use crate::naming::words_of;
use crate::pretty::{Expr, WIDTH, assignment, render, specifiers};

const HEAD: &str = include_str!("templates/client_head.ts.tmpl");
const TAIL: &str = include_str!("templates/client_tail.ts.tmpl");
const NODE_HEAD: &str = include_str!("templates/node/index_head.ts.tmpl");
const NODE_TAIL: &str = include_str!("templates/node/index_tail.ts.tmpl");

const IMPORTS_TOP: &str = "import type { RequestInit, RequestInfo, BodyInit } from './internal/builtin-types';
import type { HTTPMethod, PromiseOrValue, MergedRequestInit, FinalizedRequestInit } from './internal/types';
import { uuid4 } from './internal/utils/uuid';
import { validatePositiveInteger, isAbsoluteURL, safeJSON } from './internal/utils/values';
import { sleep } from './internal/utils/sleep';
export type { Logger, LogLevel } from './internal/utils/log';
import { castToError, isAbortError } from './internal/errors';
import type { APIResponseProps } from './internal/parse';
import { getPlatformHeaders } from './internal/detect-platform';
import * as Shims from './internal/shims';
import * as Opts from './internal/request-options';
import { stringifyQuery } from './internal/utils/query';
import { VERSION } from './version';
import * as Errors from './core/error';";

const IMPORTS_BOTTOM: &str = "import { type Fetch } from './internal/builtin-types';
import { HeadersLike, NullableHeaders, buildHeaders } from './internal/headers';
import { FinalRequestOptions, RequestOptions } from './internal/request-options';
import { readEnv } from './internal/utils/env';
import {
  type LogLevel,
  type Logger,
  formatRequestDetails,
  loggerFor,
  parseLogLevel,
} from './internal/utils/log';
import { isEmptyObj } from './internal/utils/values';";

const ERRORS: [&str; 12] = [
    "APIError",
    "APIConnectionError",
    "APIConnectionTimeoutError",
    "APIUserAbortError",
    "NotFoundError",
    "ConflictError",
    "RateLimitError",
    "BadRequestError",
    "AuthenticationError",
    "InternalServerError",
    "PermissionDeniedError",
    "UnprocessableEntityError",
];

/// The auth option the client reads from the environment, if the settings have one.
fn auth_opt<'a>(ctx: &Ctx<'a>) -> Option<&'a ClientOpt> {
    ctx.client.settings.as_ref()?.opts.iter().find(|o| o.auth.is_some())
}

/// The `authHeaders` body for the auth option's security scheme.
fn auth_headers(ctx: &Ctx<'_>, opt: &ClientOpt, prop: &str) -> String {
    let scheme = ctx.client.settings.as_ref().and_then(|s| {
        let name = &opt.auth.as_ref()?.security_scheme;
        s.security_schemes.iter().find(|n| &n.name == name).map(|n| &n.scheme)
    });
    match scheme {
        Some(SecurityScheme::ApiKey { name, .. }) => {
            format!("    return buildHeaders([{{ '{name}': this.{prop} }}]);")
        }
        _ => format!("    return buildHeaders([{{ Authorization: `Bearer ${{this.{prop}}}` }}]);"),
    }
}

/// `import { ... } from './resources/x';` for the top-level resources: files first, then packages.
fn resource_imports(ctx: &Ctx<'_>) -> Vec<String> {
    let mut leaves: Vec<&TsResource<'_>> = ctx.resources.iter().filter(|r| !r.package).collect();
    let mut packages: Vec<&TsResource<'_>> = ctx.resources.iter().filter(|r| r.package).collect();
    leaves.sort_by_key(|r| r.module());
    packages.sort_by_key(|r| r.module());
    leaves
        .into_iter()
        .chain(packages)
        .map(|r| {
            let mut names = vec![r.class.clone()];
            if !r.res.standalone {
                names.extend(exports_of(ctx, r));
            }
            names.sort();
            render(
                &specifiers("import ", &names, &format!(" from './{}';", r.module())),
                0,
                WIDTH,
            )
        })
        .collect()
}

/// The resources on the client, each with its description.
fn members(ctx: &Ctx<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for r in &ctx.resources {
        if let Some(d) = r.description.as_deref().filter(|d| !d.is_empty()) {
            out.push("  /**".to_owned());
            for l in d.lines() {
                out.push(if l.is_empty() {
                    "   *".to_owned()
                } else {
                    format!("   * {l}")
                });
            }
            out.push("   */".to_owned());
        }
        let left = text(format!("{}: API.{}", r.prop, r.class));
        let right = Expr::Raw(format!("new API.{}(this)", r.class));
        let member = concat(vec![assignment(left, " =", &right, false), text(";")]);
        out.push(render(&member, 2, WIDTH));
    }
    out
}

/// The `export declare namespace <Client>` blocks of the resources, then the shared models.
fn namespace(ctx: &Ctx<'_>) -> Vec<String> {
    let client = &ctx.settings.client_class;
    let options = if ctx.node() { "Core" } else { "Opts" };
    let mut out = vec![
        format!("export declare namespace {client} {{"),
        format!("  export type RequestOptions = {options}.RequestOptions;"),
    ];
    for r in &ctx.resources {
        out.push(String::new());
        let mut names = vec![format!("{} as {}", r.class, r.class)];
        if !r.res.standalone {
            names.extend(exports_of(ctx, r).into_iter().map(|t| format!("type {t} as {t}")));
        }
        out.push(render(&specifiers("export ", &names, ";"), 2, WIDTH));
    }
    // The current layout lists them too (imagekit-typescript); not emitted there yet, as that
    // changes the golden petstore SDK.
    let shared = if ctx.node() { ctx.shared_models() } else { Vec::new() };
    if !shared.is_empty() {
        out.push(String::new());
        for d in shared {
            let t = ctx.type_of(&d.r#ref);
            out.push(format!("  export type {t} = API.{t};"));
        }
    }
    out.push("}".to_owned());
    out
}

/// The documentation lines of the auth option in `ClientOptions`, at three spaces.
fn opt_doc(opt: Option<&ClientOpt>) -> String {
    let env = opt.and_then(|o| o.read_env.clone()).unwrap_or_default();
    let doc = opt
        .and_then(|o| o.description.clone())
        .unwrap_or_else(|| format!("Defaults to process.env['{env}']."));
    crate::comment::format_markdown(&doc, 1000)
        .into_iter()
        .map(|l| {
            if l.is_empty() {
                "   *".to_owned()
            } else {
                format!("   * {l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `authHeaders` body of a `node`-layout client: the scheme's header, or nothing when the
/// key goes in the query.
fn node_auth_headers(ctx: &Ctx<'_>, opt: &ClientOpt, prop: &str) -> String {
    if opt.send.as_ref().is_some_and(|s| s.location == SendLocation::Query) {
        return "    return {};".to_owned();
    }
    let scheme = ctx.client.settings.as_ref().and_then(|s| {
        let name = &opt.auth.as_ref()?.security_scheme;
        s.security_schemes.iter().find(|n| &n.name == name).map(|n| &n.scheme)
    });
    match scheme {
        Some(SecurityScheme::ApiKey { name, .. }) => {
            format!("    return {{ {}: this.{prop} }};", crate::js::property_key(name))
        }
        _ => format!("    return {{ Authorization: `Bearer ${{this.{prop}}}` }};"),
    }
}

/// The `environments` of the client settings, the default first: (name, base URL).
#[must_use]
pub fn environments(ctx: &Ctx<'_>) -> Vec<(String, String)> {
    let Some(s) = ctx.client.settings.as_ref() else {
        return Vec::new();
    };
    let mut envs: Vec<(String, String)> = s
        .environments
        .iter()
        .map(|e| (e.name.written.clone(), e.url.clone()))
        .collect();
    if let Some(i) = s
        .default_environment
        .as_ref()
        .and_then(|d| envs.iter().position(|(k, _)| k == d))
    {
        let default = envs.remove(i);
        envs.insert(0, default);
    }
    envs
}

/// The base URL a client without options uses: the default environment's, else the spec's.
#[must_use]
pub fn default_base_url(ctx: &Ctx<'_>) -> String {
    environments(ctx)
        .first()
        .map_or_else(|| ctx.client.base_url.clone(), |(_, url)| url.clone())
}

/// `src/index.ts` of the `node` layout: the client class, its resources, the namespace and the
/// package's exports.
#[must_use]
pub fn emit_node_index(ctx: &Ctx<'_>) -> File {
    let s = &ctx.settings;
    let client = s.client_class.as_str();
    let opt = auth_opt(ctx);
    let opt_words = opt.map(|o| words_of(&o.name)).unwrap_or_default();
    let prop = ctx.member(&opt_words);
    let opt_env = opt.and_then(|o| o.read_env.clone()).unwrap_or_default();
    let placeholder = crate::js::string_literal(&super::api_tests::placeholder(&opt_words));
    let query_key = opt
        .and_then(|o| o.send.as_ref())
        .filter(|send| send.location == SendLocation::Query)
        .map(|send| crate::js::property_key(&send.name));
    let auth = opt.map(|o| node_auth_headers(ctx, o, &prop)).unwrap_or_default();
    let doc = opt_doc(opt);
    let timeout = timeout_ms(ctx);
    let envs = environments(ctx);
    let default_env = envs.first().map(|(k, _)| k.clone()).unwrap_or_default();
    let env_lines: Vec<String> = envs
        .iter()
        .map(|(k, url)| format!("  {}: {},", crate::js::property_key(k), crate::js::string_literal(url)))
        .collect();
    let env_docs: Vec<String> = envs
        .iter()
        .map(|(k, url)| format!("   * - `{k}` corresponds to `{url}`"))
        .collect();
    let base_url = default_base_url(ctx);
    let (env_lines, env_docs) = (env_lines.join("\n"), env_docs.join("\n"));
    let vars = [
        ("OPT_DOC", doc.as_str()),
        ("OPT", prop.as_str()),
        ("OPT_ENV", opt_env.as_str()),
        ("OPT_PLACEHOLDER", placeholder.as_str()),
        ("ENV", s.env_prefix.as_str()),
        ("TITLE", s.title.as_str()),
        ("CLIENT", client),
        ("BASE_URL", base_url.as_str()),
        ("TIMEOUT", timeout.as_str()),
        ("QUERY_KEY", query_key.as_deref().unwrap_or_default()),
        ("AUTH_HEADERS", auth.as_str()),
        ("ENVIRONMENTS", env_lines.as_str()),
        ("ENVIRONMENT_DOCS", env_docs.as_str()),
        ("DEFAULT_ENVIRONMENT", default_env.as_str()),
    ];
    let flags = [
        ("query_auth", query_key.is_some()),
        ("nullable", opt.is_some_and(|o| o.nullable)),
        ("environments", envs.len() > 1),
    ];
    let mut out = vec![
        HEADER.to_owned(),
        String::new(),
        "import { type Agent } from './_shims/index';".to_owned(),
        "import * as Core from './core';".to_owned(),
        "import * as Errors from './error';".to_owned(),
        "import * as Uploads from './uploads';".to_owned(),
        "import * as API from './resources/index';".to_owned(),
    ];
    out.extend(resource_imports(ctx));
    out.push(String::new());
    out.push(fill(NODE_HEAD, &vars, &flags).trim_end_matches('\n').to_owned());
    out.push(String::new());
    out.extend(members(ctx));
    out.push(String::new());
    out.push(fill(NODE_TAIL, &vars, &flags).trim_end_matches('\n').to_owned());
    out.push("}".to_owned());
    out.push(String::new());
    for r in &ctx.resources {
        out.push(format!("{client}.{} = {};", r.class, r.class));
    }
    out.push(String::new());
    out.extend(namespace(ctx));
    out.push(String::new());
    out.push("export { toFile, fileFromPath } from './uploads';".to_owned());
    let mut errors = vec![format!("{client}Error")];
    errors.extend(ERRORS.iter().map(|&e| e.to_owned()));
    out.push(render(&specifiers("export ", &errors, " from './error';"), 0, WIDTH));
    out.push(String::new());
    out.push(format!("export default {client};"));
    ("src/index.ts".to_owned(), out.join("\n") + "\n")
}

/// `src/client.ts`.
#[must_use]
pub fn emit_client(ctx: &Ctx<'_>) -> File {
    let s = &ctx.settings;
    let client = s.client_class.as_str();
    let opt = auth_opt(ctx);
    let opt_words = opt.map(|o| words_of(&o.name)).unwrap_or_default();
    let prop = ctx.member(&opt_words);
    let opt_env = opt.and_then(|o| o.read_env.clone()).unwrap_or_default();
    let opt_doc = opt
        .and_then(|o| o.description.clone())
        .unwrap_or_else(|| format!("Defaults to process.env['{opt_env}']."));
    let placeholder = crate::js::string_literal(&super::api_tests::placeholder(&opt_words));
    let auth = opt.map(|o| auth_headers(ctx, o, &prop)).unwrap_or_default();
    let doc_lines: Vec<String> = crate::comment::format_markdown(&opt_doc, 1000)
        .into_iter()
        .map(|l| {
            if l.is_empty() {
                "   *".to_owned()
            } else {
                format!("   * {l}")
            }
        })
        .collect();
    let doc = doc_lines.join("\n");
    let base_url = ctx.client.base_url.clone();
    let head = fill(
        HEAD,
        &[
            ("OPT_DOC", &doc),
            ("OPT", &prop),
            ("OPT_ENV", &opt_env),
            ("OPT_PLACEHOLDER", &placeholder),
            ("ENV", &s.env_prefix),
            ("TITLE", &s.title),
            ("CLIENT", client),
            ("BASE_URL", &base_url),
            ("AUTH_HEADERS", &auth),
        ],
        &[],
    );
    let tail = fill(TAIL, &[("CLIENT", client)], &[("pagination", false)]);
    let mut out = vec![HEADER.to_owned(), String::new(), IMPORTS_TOP.to_owned()];
    out.push("import * as Uploads from './core/uploads';".to_owned());
    out.push("import * as API from './resources/index';".to_owned());
    out.push("import { APIPromise } from './core/api-promise';".to_owned());
    out.extend(resource_imports(ctx));
    out.push(IMPORTS_BOTTOM.to_owned());
    out.push(String::new());
    out.push(head.trim_end_matches('\n').to_owned());
    out.push(String::new());
    out.push("  /**".to_owned());
    out.push("   * Basic re-implementation of `qs.stringify` for primitive types.".to_owned());
    out.push("   */".to_owned());
    out.push(tail.trim_end_matches('\n').to_owned());
    out.push(String::new());
    out.push(format!("  static {client} = this;"));
    out.push(format!("  static DEFAULT_TIMEOUT = {}; // 1 minute", timeout_ms(ctx)));
    out.push(String::new());
    out.push(format!("  static {client}Error = Errors.{client}Error;"));
    for e in ERRORS {
        out.push(format!("  static {e} = Errors.{e};"));
    }
    out.push(String::new());
    out.push("  static toFile = Uploads.toFile;".to_owned());
    out.push(String::new());
    out.extend(members(ctx));
    out.push("}".to_owned());
    out.push(String::new());
    for r in &ctx.resources {
        out.push(format!("{client}.{} = {};", r.class, r.class));
    }
    out.push(String::new());
    out.extend(namespace(ctx));
    ("src/client.ts".to_owned(), out.join("\n") + "\n")
}

fn timeout_ms(ctx: &Ctx<'_>) -> String {
    let seconds = ctx.client.settings.as_ref().map_or(60.0, |s| s.timeout_seconds);
    crate::js::number_to_string(seconds * 1000.0)
}

/// Whether the SDK serialises queries with the vendored `qs` (a query parameter is an array or
/// an object) rather than the basic `stringifyQuery`.
#[must_use]
pub fn uses_qs(ctx: &Ctx<'_>) -> bool {
    use ferrule_sdk_model::model::Type;
    fn nested(t: &Type) -> bool {
        match t {
            Type::Array { .. } | Type::Map { .. } | Type::Object(_) => true,
            Type::Nullable { inner } => nested(inner),
            Type::Union(u) => u.variants.iter().any(|v| nested(&v.ty)),
            _ => false,
        }
    }
    ctx.all().iter().any(|r| {
        r.res
            .methods
            .iter()
            .any(|m| m.query_params.iter().any(|p| nested(&p.field.ty)))
    })
}

/// `src/version.ts`.
#[must_use]
pub fn emit_version(ctx: &Ctx<'_>) -> File {
    (
        "src/version.ts".to_owned(),
        format!(
            "export const VERSION = '{}'; // x-release-please-version\n",
            ctx.settings.version
        ),
    )
}
