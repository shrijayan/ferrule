//! The client: `client/<Client>Client{,Async}{,Impl}.kt`, `core/ClientOptions.kt` (its client
//! settings slots) and the client-okhttp module's `<Client>OkHttpClient{,Async}.kt`.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ApiKeyLocation, AuthRole, ClientOpt, HttpAuthScheme, SecurityScheme, SendLocation};
use serde_json::Value;

use crate::emit::Gen;
use crate::emit::tmpl::render;
use crate::kt::{HEADER, Imports, doc, file, string};
use crate::naming::{camel, constant, split_words, words};

const CLIENT_OPTIONS: &str = include_str!("templates/ClientOptions.kt.tmpl");
const OKHTTP_CLIENT: &str = include_str!("templates/OkHttpClient.kt.tmpl");

/// Every client file.
#[must_use]
pub fn client_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for async_ in [false, true] {
        out.push(interface_file(g, async_));
        out.push(impl_file(g, async_));
        out.push(okhttp_file(g, async_));
    }
    out.push(client_options_file(g));
    out
}

fn suffix(async_: bool) -> &'static str {
    if async_ { "Async" } else { "" }
}

fn modifier_ty(g: &Gen<'_>) -> &'static str {
    if g.java() {
        "Consumer<ClientOptions.Builder>"
    } else {
        "(ClientOptions.Builder) -> Unit"
    }
}

/// `<Client>Client` / `<Client>ClientAsync`.
fn client_class(g: &Gen<'_>, async_: bool) -> String {
    format!("{}Client{}", g.client_name(), suffix(async_))
}

fn interface_file(g: &Gen<'_>, async_: bool) -> (String, String) {
    let name = client_class(g, async_);
    let other = client_class(g, !async_);
    let mut imp = Imports::new(g.root());
    imp.core("ClientOptions");
    if g.java() {
        imp.add("java.util.function.Consumer");
    }
    let top = g.top_services();
    for &i in &top {
        let s = &g.services[i];
        imp.add(format!(
            "{}.{}Service{}",
            g.service_package(s, async_),
            s.name,
            suffix(async_)
        ));
    }
    let (mode, switch, switch_to) = if async_ {
        ("asynchronously", "synchronous", "sync")
    } else {
        ("synchronously", "asynchronous", "async")
    };
    let mut o = doc(&format!(
        "A client for interacting with the {} REST API {mode}. You can also switch to {switch} execution via the [{switch_to}] method.\n\nThis client performs best when you create a single instance and reuse it for all interactions with the REST API. This is because each client holds its own connection pool and thread pools. Reusing connections and threads reduces latency and saves memory. The client also handles rate limiting per client. This means that creating and using multiple instances at the same time will not respect rate limits.\n\nThe threads and connections that are held will be released automatically if they remain idle. But if you are writing an application that needs to aggressively release unused resources, then you may call [close].",
        g.target.title
    ));
    let _ = writeln!(o, "interface {name} {{\n");
    o.push_str(&doc(&format!(
        "Returns a version of this client that uses {switch} execution.\n\nThe returned client shares its resources, like its connection pool and thread pools, with this client."
    )));
    let _ = writeln!(o, "fun {switch_to}(): {other}\n");
    o.push_str(&doc(
        "Returns a view of this service that provides access to raw HTTP responses for each method.",
    ));
    o.push_str("fun withRawResponse(): WithRawResponse\n\n");
    o.push_str(&doc(
        "Returns a view of this service with the given option modifications applied.\n\nThe original service is not modified.",
    ));
    let _ = writeln!(o, "fun withOptions(modifier: {}): {name}\n", modifier_ty(g));
    for &i in &top {
        let s = &g.services[i];
        let _ = writeln!(o, "fun {}(): {}Service{}\n", s.accessor, s.name, suffix(async_));
    }
    o.push_str(&doc(
        "Closes this client, relinquishing any underlying resources.\n\nThis is purposefully not inherited from [AutoCloseable] because the client is long-lived and usually should not be synchronously closed via try-with-resources.\n\nIt's also usually not necessary to call this method at all. the default HTTP client automatically releases threads and connections if they remain idle, but if you are writing an application that needs to aggressively release unused resources, then you may call this method.",
    ));
    o.push_str("fun close()\n\n");
    o.push_str(&doc(&format!(
        "A view of [{name}] that provides access to raw HTTP responses for each method."
    )));
    o.push_str("interface WithRawResponse {\n\n");
    o.push_str(&doc(
        "Returns a view of this service with the given option modifications applied.\n\nThe original service is not modified.",
    ));
    let _ = writeln!(
        o,
        "fun withOptions(modifier: {}): {name}.WithRawResponse\n",
        modifier_ty(g)
    );
    for &i in &top {
        let s = &g.services[i];
        let _ = writeln!(
            o,
            "fun {}(): {}Service{}.WithRawResponse\n",
            s.accessor,
            s.name,
            suffix(async_)
        );
    }
    o.push_str("}\n}\n");
    let path = format!("{}/client/{name}.kt", g.core_main());
    (path, file(&format!("{}.client", g.root()), &imp, &o))
}

fn impl_file(g: &Gen<'_>, async_: bool) -> (String, String) {
    let base = client_class(g, async_);
    let name = format!("{base}Impl");
    let other = client_class(g, !async_);
    let mut imp = Imports::new(g.root());
    imp.core("ClientOptions");
    imp.core("getPackageVersion");
    if g.java() {
        imp.add("java.util.function.Consumer");
    }
    let top = g.top_services();
    for &i in &top {
        let s = &g.services[i];
        let pkg = g.service_package(s, async_);
        imp.add(format!("{pkg}.{}Service{}", s.name, suffix(async_)));
        imp.add(format!("{pkg}.{}Service{}Impl", s.name, suffix(async_)));
    }
    let switch_to = if async_ { "sync" } else { "async" };
    let apply = if g.java() { "modifier::accept" } else { "modifier" };
    let mut o = String::new();
    let _ = writeln!(
        o,
        "class {name}(private val clientOptions: ClientOptions) : {base} {{\n"
    );
    let _ = writeln!(
        o,
        "private val clientOptionsWithUserAgent =\nif (clientOptions.headers.names().contains(\"User-Agent\")) clientOptions\nelse\nclientOptions.toBuilder().putHeader(\"User-Agent\", \"${{javaClass.simpleName}}/{} ${{getPackageVersion()}}\").build()\n",
        g.target.flavor.display()
    );
    let _ = writeln!(
        o,
        "// Pass the original clientOptions so that this client sets its own User-Agent.\nprivate val {switch_to}: {other} by lazy {{ {other}Impl(clientOptions) }}\n"
    );
    let _ = writeln!(
        o,
        "private val withRawResponse: {base}.WithRawResponse by lazy {{\nWithRawResponseImpl(clientOptions)\n}}\n"
    );
    for &i in &top {
        let s = &g.services[i];
        let svc = format!("{}Service{}", s.name, suffix(async_));
        let _ = writeln!(
            o,
            "private val {}: {svc} by lazy {{ {svc}Impl(clientOptionsWithUserAgent) }}\n",
            s.accessor
        );
    }
    let _ = writeln!(o, "override fun {switch_to}(): {other} = {switch_to}\n");
    let _ = writeln!(
        o,
        "override fun withRawResponse(): {base}.WithRawResponse = withRawResponse\n"
    );
    let _ = writeln!(
        o,
        "override fun withOptions(modifier: {}): {base} =\n{name}(clientOptions.toBuilder().apply({apply}).build())\n",
        modifier_ty(g)
    );
    for &i in &top {
        let s = &g.services[i];
        let _ = writeln!(
            o,
            "override fun {a}(): {}Service{} = {a}\n",
            s.name,
            suffix(async_),
            a = s.accessor
        );
    }
    o.push_str("override fun close() = clientOptions.close()\n\n");
    let _ = writeln!(
        o,
        "class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :\n{base}.WithRawResponse {{\n"
    );
    for &i in &top {
        let s = &g.services[i];
        let svc = format!("{}Service{}", s.name, suffix(async_));
        let _ = writeln!(
            o,
            "private val {}: {svc}.WithRawResponse by lazy {{\n{svc}Impl.WithRawResponseImpl(clientOptions)\n}}\n",
            s.accessor
        );
    }
    let _ = writeln!(
        o,
        "override fun withOptions(\nmodifier: {}\n): {base}.WithRawResponse =\n{name}.WithRawResponseImpl(clientOptions.toBuilder().apply({apply}).build())\n",
        modifier_ty(g)
    );
    for &i in &top {
        let s = &g.services[i];
        let _ = writeln!(
            o,
            "override fun {a}(): {}Service{}.WithRawResponse = {a}\n",
            s.name,
            suffix(async_),
            a = s.accessor
        );
    }
    o.push_str("}\n}\n");
    let path = format!("{}/client/{name}.kt", g.core_main());
    (path, file(&format!("{}.client", g.root()), &imp, &o))
}

/// The default environment's URL.
fn base_url(g: &Gen<'_>) -> String {
    g.client.settings.as_ref().map_or_else(
        || g.client.base_url.clone(),
        |s| {
            let chosen = s
                .default_environment
                .as_ref()
                .and_then(|d| s.environments.iter().find(|e| e.name.written == *d))
                .or_else(|| s.environments.first());
            chosen.map_or_else(|| g.client.base_url.clone(), |e| e.url.clone())
        },
    )
}

/// The client options: `client_settings.opts`.
fn opts<'a>(g: &Gen<'a>) -> &'a [ClientOpt] {
    g.client.settings.as_ref().map_or(&[], |s| s.opts.as_slice())
}

/// An option is required when it can be neither null nor defaulted.
fn opt_required(o: &ClientOpt) -> bool {
    !o.nullable && o.default.is_none()
}

fn opt_name(g: &Gen<'_>, o: &ClientOpt) -> String {
    camel(&words(&o.name), g.casings)
}

fn opt_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        ferrule_sdk_model::model::OptType::String => "String",
        ferrule_sdk_model::model::OptType::Number => "Double",
        ferrule_sdk_model::model::OptType::Integer => "Long",
        ferrule_sdk_model::model::OptType::Boolean => "Boolean",
    }
}

fn literal(v: &Value) -> String {
    match v {
        Value::String(s) => string(s),
        other => other.to_string(),
    }
}

/// The system property an option is read from: `onebusawaysdk.onebusawayApiKey`.
fn property(g: &Gen<'_>, env: &str) -> String {
    format!(
        "{}.{}",
        g.client_name().to_ascii_lowercase(),
        camel(&split_words(env), g.casings)
    )
}

fn env_prefix(g: &Gen<'_>) -> String {
    g.client
        .settings
        .as_ref()
        .map_or_else(|| constant(&split_words(g.client_name())), |s| s.env_prefix.clone())
}

/// The `fromEnv` table: setter, system property, environment variable, required, default.
fn env_table(g: &Gen<'_>) -> String {
    let mut rows: Vec<[String; 5]> = vec![[
        "Setter".to_owned(),
        "System property".to_owned(),
        "Environment variable".to_owned(),
        "Required".to_owned(),
        "Default value".to_owned(),
    ]];
    for o in opts(g) {
        let Some(env) = &o.read_env else { continue };
        rows.push([
            format!("`{}`", opt_name(g, o)),
            format!("`{}`", property(g, env)),
            format!("`{env}`"),
            opt_required(o).to_string(),
            o.default
                .as_ref()
                .map_or_else(|| "-".to_owned(), |d| format!("`{}`", literal(d))),
        ]);
    }
    rows.push([
        "`baseUrl`".to_owned(),
        format!("`{}.baseUrl`", g.client_name().to_ascii_lowercase()),
        format!("`{}_BASE_URL`", env_prefix(g)),
        "true".to_owned(),
        format!("`{}`", string(&base_url(g))),
    ]);
    let widths: Vec<usize> = (0..5)
        .map(|c| rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0))
        .collect();
    let line = |cells: &[String]| {
        let mut s = String::from("         * |");
        for (c, cell) in cells.iter().enumerate() {
            let _ = write!(s, "{cell:<w$}|", w = widths[c]);
        }
        s
    };
    let mut out = vec![line(&rows[0])];
    out.push(line(&widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>()));
    out.extend(rows[1..].iter().map(|r| line(r)));
    out.join("\n")
}

/// How the client sends an option's value, as `build()` writes it.
fn auth_text(g: &Gen<'_>) -> String {
    let Some(settings) = g.client.settings.as_ref() else {
        return String::new();
    };
    let mut out = String::new();
    // Basic auth pairs the username and password options.
    let user = settings
        .opts
        .iter()
        .find(|o| o.auth.as_ref().is_some_and(|a| a.role == AuthRole::Username));
    let pass = settings
        .opts
        .iter()
        .find(|o| o.auth.as_ref().is_some_and(|a| a.role == AuthRole::Password));
    if let (Some(u), Some(p)) = (user, pass) {
        let (un, pn) = (opt_name(g, u), opt_name(g, p));
        let _ = writeln!(
            out,
            "{un}.let {{ username ->\n{pn}?.let {{ password ->\nif (!username.isEmpty() && !password.isEmpty()) {{\nheaders.replace(\n\"Authorization\",\n\"Basic ${{Base64.getEncoder().encodeToString(\"$username:$password\".toByteArray())}}\",\n)\n}}\n}}\n}}"
        );
    }
    for o in &settings.opts {
        let Some(auth) = &o.auth else { continue };
        if auth.role != AuthRole::Value {
            continue;
        }
        let scheme = settings
            .security_schemes
            .iter()
            .find(|s| s.name == auth.security_scheme)
            .map(|s| &s.scheme);
        let n = opt_name(g, o);
        let set = match (scheme, &o.send) {
            (Some(SecurityScheme::ApiKey { location, name }), _) => match location {
                ApiKeyLocation::Query => format!("queryParams.replace({}, it)", string(name)),
                _ => format!("headers.replace({}, it)", string(name)),
            },
            (
                Some(SecurityScheme::Http {
                    scheme: HttpAuthScheme::Bearer,
                    ..
                }),
                _,
            ) => "headers.replace(\"Authorization\", \"Bearer $it\")".to_owned(),
            (_, Some(send)) if send.location == SendLocation::Query => {
                format!("queryParams.replace({}, it)", string(&send.name))
            }
            (_, Some(send)) => format!("headers.replace({}, it)", string(&send.name)),
            _ => continue,
        };
        let q = if opt_required(o) { "" } else { "?" };
        let _ = writeln!(out, "{n}{q}.let {{\nif (!it.isEmpty()) {{\n{set}\n}}\n}}");
    }
    out
}

// One slot after another in the template's order: splitting it would scatter the slots.
#[allow(clippy::too_many_lines)]
fn client_options_file(g: &Gen<'_>) -> (String, String) {
    let java = g.java();
    let mut ctor = String::new();
    let mut getters = String::new();
    let mut required = String::new();
    let mut vars = String::new();
    let mut from = String::new();
    let mut setters = String::new();
    let mut from_env = format!(
        "(System.getProperty(\"{}.baseUrl\") ?: System.getenv(\"{}_BASE_URL\"))?.let {{ baseUrl(it) }}\n",
        g.client_name().to_ascii_lowercase(),
        env_prefix(g)
    );
    let mut checks = String::new();
    let mut args = String::new();
    for o in opts(g) {
        let n = opt_name(g, o);
        let t = opt_type(o);
        let desc = o.description.as_deref().unwrap_or("").trim();
        if opt_required(o) {
            ctor.push_str(&doc(desc));
            let jvm = if java {
                format!("@get:JvmName(\"{n}\") ")
            } else {
                String::new()
            };
            let _ = writeln!(ctor, "{jvm}val {n}: {t},");
            let _ = writeln!(required, "         * .{n}()");
            let _ = writeln!(vars, "private var {n}: {t}? = null");
            let _ = writeln!(checks, "val {n} = checkRequired({}, {n})", string(&n));
            setters.push_str(&doc(desc));
            let _ = writeln!(setters, "fun {n}({n}: {t}) = apply {{ this.{n} = {n} }}\n");
        } else {
            let _ = writeln!(ctor, "private val {n}: {t}?,");
            let mut gdoc = desc.to_owned();
            if let Some(d) = &o.default {
                if !gdoc.is_empty() {
                    gdoc.push_str("\n\n");
                }
                let _ = write!(gdoc, "Defaults to `{}`.", literal(d));
            }
            getters.push_str(&doc(&gdoc));
            if java {
                let _ = writeln!(getters, "fun {n}(): Optional<{t}> = Optional.ofNullable({n})\n");
            } else {
                let _ = writeln!(getters, "fun {n}(): {t}? = {n}\n");
            }
            let init = o.default.as_ref().map_or_else(|| "null".to_owned(), literal);
            let _ = writeln!(vars, "private var {n}: {t}? = {init}");
            setters.push_str(&doc(&gdoc));
            let _ = writeln!(setters, "fun {n}({n}: {t}?) = apply {{ this.{n} = {n} }}\n");
            if java {
                setters.push_str(&doc(&format!(
                    "Alias for calling [Builder.{n}] with `{n}.orElse(null)`."
                )));
                let _ = writeln!(setters, "fun {n}({n}: Optional<{t}>) = {n}({n}.getOrNull())\n");
            }
        }
        let _ = writeln!(from, "{n} = clientOptions.{n}");
        if let Some(env) = &o.read_env {
            let _ = writeln!(
                from_env,
                "(System.getProperty(\"{}\") ?: System.getenv(\"{env}\"))?.let {{ {n}(it) }}",
                property(g, env)
            );
        }
        let _ = writeln!(args, "{n},");
    }
    let envs: Vec<(String, String)> = g
        .client
        .settings
        .as_ref()
        .map(|s| {
            s.environments
                .iter()
                .map(|e| (format!("{}_URL", constant(&words(&e.name))), e.url.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut consts = String::new();
    for (k, url) in &envs {
        let _ = writeln!(consts, "const val {k} = {}\n", string(url));
    }
    let default_url = envs
        .iter()
        .find(|(_, u)| *u == base_url(g))
        .map_or_else(|| string(&base_url(g)), |(k, _)| k.clone());
    let auth = auth_text(g);
    let base = base_url(g);
    let prefix = env_prefix(g);
    let lang = g.target.flavor.key();
    let values = [
        ("HEADER", HEADER),
        ("PKG", g.root()),
        ("ARTIFACT", g.target.artifact.as_str()),
        ("BASE_URL", base.as_str()),
        ("DEFAULT_URL", default_url.as_str()),
        ("ENV_PREFIX", prefix.as_str()),
        ("LANG", lang),
    ];
    let table = env_table(g);
    let slots = [
        ("CTOR", ctor.as_str()),
        ("GETTERS", getters.as_str()),
        ("ENV_CONSTS", consts.trim_end()),
        ("REQUIRED", required.as_str()),
        ("VARS", vars.as_str()),
        ("FROM", from.as_str()),
        ("SETTERS", setters.trim_end()),
        ("ENV_TABLE", table.as_str()),
        ("FROM_ENV", from_env.as_str()),
        ("CHECKS", checks.as_str()),
        ("AUTH", auth.as_str()),
        ("ARGS", args.as_str()),
    ];
    // ktfmt drops the template's `Base64` import when no basic auth uses it.
    let text = render(CLIENT_OPTIONS, g.target.flavor, &values, &slots);
    let path = format!("{}/core/ClientOptions.kt", g.core_main());
    (path, text)
}

fn okhttp_file(g: &Gen<'_>, async_: bool) -> (String, String) {
    let mut setters = String::new();
    for o in opts(g) {
        let n = opt_name(g, o);
        let t = opt_type(o);
        let desc = o.description.as_deref().unwrap_or("").trim();
        if opt_required(o) {
            setters.push_str(&doc(desc));
            let _ = writeln!(setters, "fun {n}({n}: {t}) = apply {{ clientOptions.{n}({n}) }}\n");
        } else {
            let mut gdoc = desc.to_owned();
            if let Some(d) = &o.default {
                if !gdoc.is_empty() {
                    gdoc.push_str("\n\n");
                }
                let _ = write!(gdoc, "Defaults to `{}`.", literal(d));
            }
            setters.push_str(&doc(&gdoc));
            let _ = writeln!(setters, "fun {n}({n}: {t}?) = apply {{ clientOptions.{n}({n}) }}\n");
            if g.java() {
                setters.push_str(&doc(&format!(
                    "Alias for calling [Builder.{n}] with `{n}.orElse(null)`."
                )));
                let _ = writeln!(setters, "fun {n}({n}: Optional<{t}>) = {n}({n}.getOrNull())\n");
            }
        }
    }
    let base = base_url(g);
    let values = [
        ("HEADER", HEADER),
        ("PKG", g.root()),
        ("CLIENT", g.client_name()),
        ("ASYNC", suffix(async_)),
        ("BASE_URL", base.as_str()),
    ];
    let slots = [("OPTS", setters.trim_end())];
    let text = render(OKHTTP_CLIENT, g.target.flavor, &values, &slots);
    let path = format!(
        "{}-client-okhttp/src/main/kotlin/{}/client/okhttp/{}OkHttpClient{}.kt",
        g.target.artifact,
        g.root_path(),
        g.client_name(),
        suffix(async_)
    );
    (path, text)
}
