//! `I<Client>Client.cs` and `<Client>Client.cs`: the client's options as properties, its
//! services, and the raw-response client that sends requests with retries.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ClientOpt, OptType};

use crate::doc;
use crate::emit::Gen;
use crate::emit::model::sort_usings;
use crate::naming::{Casings, decapitalize, pascal, words};

/// One client option as C# declares it.
pub struct Opt {
    /// `PrivateKey`.
    pub name: String,
    /// `string` or `string?`.
    pub ty: String,
    pub nullable: bool,
    pub description: String,
    /// `IMAGEKIT_PRIVATE_KEY`.
    pub env: Option<String>,
    /// The default when the variable is unset, as a C# literal.
    pub default: Option<String>,
    /// Whether a security scheme reads it.
    pub auth: bool,
}

fn opt_type(t: OptType) -> &'static str {
    match t {
        OptType::String => "string",
        OptType::Number => "double",
        OptType::Integer => "long",
        OptType::Boolean => "bool",
    }
}

/// The client options of `client_settings.opts`.
#[must_use]
pub fn opts(g: &Gen<'_>) -> Vec<Opt> {
    let casings = Casings(&g.client.casings);
    g.client
        .settings
        .iter()
        .flat_map(|s| &s.opts)
        .map(|o: &ClientOpt| {
            let base = opt_type(o.ty);
            Opt {
                name: pascal(&words(&o.name), casings),
                ty: if o.nullable {
                    format!("{base}?")
                } else {
                    base.to_owned()
                },
                nullable: o.nullable,
                description: o.description.clone().unwrap_or_default(),
                env: o.read_env.clone(),
                default: o.default.as_ref().map(|v| match v {
                    serde_json::Value::String(s) => format!("\"{s}\""),
                    other => other.to_string(),
                }),
                auth: o.auth.is_some(),
            }
        })
        .collect()
}

const COMMON_PROPS: [(&str, &str); 5] = [
    ("HttpClient", "HttpClient"),
    ("BaseUrl", "string"),
    ("ResponseValidation", "bool"),
    ("MaxRetries", "int?"),
    ("Timeout", "TimeSpan?"),
];

fn interface_props(g: &Gen<'_>, out: &mut String) {
    for (name, ty) in COMMON_PROPS {
        let _ = write!(
            out,
            "    /// <inheritdoc cref=\"ClientOptions.{name}\" />\n    {ty} {name} {{ get; init; }}\n\n"
        );
    }
    for o in opts(g) {
        out.push_str(&doc::summary(&o.description, 4, doc::Place::Opt, &[]));
        let _ = write!(out, "    {} {} {{ get; init; }}\n\n", o.ty, o.name);
    }
}

/// `I<Client>Client.cs`.
#[must_use]
pub fn interface_file(g: &Gen<'_>) -> (String, String) {
    let t = g.target;
    let i = g.iclient();
    let mut out = sort_usings(
        &[
            "System",
            "System.Net.Http",
            "System.Threading",
            "System.Threading.Tasks",
            &format!("{}.Core", t.ns),
            &format!("{}.Services", t.ns),
        ]
        .map(str::to_owned),
    );
    let _ = write!(
        out,
        "\nnamespace {ns};\n\n/// <summary>\n/// A client for interacting with the {title} REST API.\n///\n/// <para>This client performs best when you create a single instance and reuse it\n/// for all interactions with the REST API. This is because each client holds its\n/// own connection pool and thread pools. Reusing connections and threads reduces\n/// latency and saves memory.</para>\n///\n/// <para>NOTE: Do not inherit from this type outside the SDK unless you're okay with\n/// breaking changes in non-major versions. We may add new methods in the future that\n/// cause existing derived classes to break.</para>\n/// </summary>\npublic interface {i} : IDisposable\n{{\n",
        ns = t.ns,
        title = t.title
    );
    interface_props(g, &mut out);
    let _ = write!(
        out,
        "    /// <summary>\n    /// Returns a view of this service that provides access to raw HTTP responses\n    /// for each method.\n    /// </summary>\n    {i}WithRawResponse WithRawResponse {{ get; }}\n\n    /// <summary>\n    /// Returns a view of this service with the given option modifications applied.\n    ///\n    /// <para>The original service is not modified.</para>\n    /// </summary>\n    {i} WithOptions(Func<ClientOptions, ClientOptions> modifier);\n"
    );
    for s in g.top_services() {
        let _ = write!(out, "\n    I{}Service {} {{ get; }}\n", s.prefix, s.prop);
    }
    let _ = write!(
        out,
        "}}\n\n/// <summary>\n/// A view of <see cref=\"{i}\"/> that provides access to raw HTTP responses for each method.\n/// </summary>\npublic interface {i}WithRawResponse : IDisposable\n{{\n"
    );
    interface_props(g, &mut out);
    let _ = write!(
        out,
        "    /// <summary>\n    /// Returns a view of this service with the given option modifications applied.\n    ///\n    /// <para>The original service is not modified.</para>\n    /// </summary>\n    {i}WithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier);\n"
    );
    for s in g.top_services() {
        let _ = write!(out, "\n    I{}ServiceWithRawResponse {} {{ get; }}\n", s.prefix, s.prop);
    }
    let _ = write!(
        out,
        "\n    /// <summary>\n    /// Sends a request to the {} REST API.\n    /// </summary>\n    Task<HttpResponse> Execute<T>(\n        HttpRequest<T> request,\n        CancellationToken cancellationToken = default\n    )\n        where T : ParamsBase;\n}}\n",
        t.title
    );
    (format!("src/{}/{i}.cs", t.ns), out)
}

fn option_props(g: &Gen<'_>, out: &mut String) {
    let mut all: Vec<(String, String)> = COMMON_PROPS
        .iter()
        .map(|(n, t)| ((*n).to_owned(), (*t).to_owned()))
        .collect();
    all.extend(opts(g).into_iter().map(|o| (o.name, o.ty)));
    for (name, ty) in all {
        let _ = write!(
            out,
            "    /// <inheritdoc/>\n    public {ty} {name}\n    {{\n        get {{ return this._options.{name}; }}\n        init {{ this._options.{name} = value; }}\n    }}\n\n"
        );
    }
}

fn lazy(ty: &str, prop: &str) -> String {
    let field = decapitalize(prop);
    format!(
        "    readonly Lazy<{ty}> _{field};\n    public {ty} {prop}\n    {{\n        get {{ return _{field}.Value; }}\n    }}\n"
    )
}

/// The retry backoff's seconds: `0.5` and `8.0`.
fn seconds(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        v.to_string()
    }
}

/// `<Client>Client.cs`.
#[must_use]
pub fn client_file(g: &Gen<'_>) -> (String, String) {
    let t = g.target;
    let i = g.iclient();
    let c = format!("{}Client", t.client);
    let settings = g.client.settings.as_ref();
    let initial = settings.and_then(|s| s.initial_retry_delay_seconds).unwrap_or(0.5);
    let max_delay = settings.and_then(|s| s.max_retry_delay_seconds).unwrap_or(8.0);
    let mut out = sort_usings(
        &[
            "System",
            "System.Collections.Generic",
            "System.IO",
            "System.Linq",
            "System.Net.Http",
            "System.Threading",
            "System.Threading.Tasks",
            &format!("{}.Core", t.ns),
            &format!("{}.Exceptions", t.ns),
            &format!("{}.Services", t.ns),
        ]
        .map(str::to_owned),
    );
    let _ = write!(
        out,
        "\nnamespace {};\n\n/// <inheritdoc/>\npublic sealed class {c} : {i}\n{{\n    readonly ClientOptions _options;\n\n",
        t.ns
    );
    option_props(g, &mut out);
    let _ = write!(
        out,
        "    readonly Lazy<{i}WithRawResponse> _withRawResponse;\n\n    /// <inheritdoc/>\n    public {i}WithRawResponse WithRawResponse\n    {{\n        get {{ return _withRawResponse.Value; }}\n    }}\n\n    /// <inheritdoc/>\n    public {i} WithOptions(Func<ClientOptions, ClientOptions> modifier)\n    {{\n        return new {c}(modifier(this._options));\n    }}\n"
    );
    for s in g.top_services() {
        out.push('\n');
        out.push_str(&lazy(&format!("I{}Service", s.prefix), &s.prop));
    }
    let _ = write!(
        out,
        "\n    public void Dispose() => this.HttpClient.Dispose();\n\n    public {c}()\n    {{\n        _options = new();\n\n        _withRawResponse = new(() => new {c}WithRawResponse(this._options));\n"
    );
    for s in g.top_services() {
        let _ = writeln!(
            out,
            "        _{} = new(() => new {}Service(this));",
            decapitalize(&s.prop),
            s.prefix
        );
    }
    let _ = write!(
        out,
        "    }}\n\n    public {c}(ClientOptions options)\n        : this()\n    {{\n        _options = options;\n    }}\n}}\n\n/// <inheritdoc/>\npublic sealed class {c}WithRawResponse : {i}WithRawResponse\n{{\n#if NET\n    static readonly Random Random = Random.Shared;\n#else\n    static readonly ThreadLocal<Random> _threadLocalRandom = new(() => new Random());\n\n    static Random Random\n    {{\n        get {{ return _threadLocalRandom.Value!; }}\n    }}\n#endif\n\n    internal static HttpMethod PatchMethod = new(\"PATCH\");\n\n    readonly ClientOptions _options;\n\n"
    );
    option_props(g, &mut out);
    let _ = write!(
        out,
        "    /// <inheritdoc/>\n    public {i}WithRawResponse WithOptions(Func<ClientOptions, ClientOptions> modifier)\n    {{\n        return new {c}WithRawResponse(modifier(this._options));\n    }}\n"
    );
    for s in g.top_services() {
        out.push('\n');
        out.push_str(&lazy(&format!("I{}ServiceWithRawResponse", s.prefix), &s.prop));
    }
    out.push_str(
        &super::template(EXECUTE)
            .replace("__CLIENT__", &t.client)
            .replace("__INITIAL__", &seconds(initial))
            .replace("__MAX__", &seconds(max_delay)),
    );
    let _ = write!(
        out,
        "\n    public {c}WithRawResponse()\n    {{\n        _options = new();\n\n"
    );
    for s in g.top_services() {
        let _ = writeln!(
            out,
            "        _{} = new(() => new {}ServiceWithRawResponse(this));",
            decapitalize(&s.prop),
            s.prefix
        );
    }
    let _ = write!(
        out,
        "    }}\n\n    public {c}WithRawResponse(ClientOptions options)\n        : this()\n    {{\n        _options = options;\n    }}\n}}\n"
    );
    (format!("src/{}/{c}.cs", t.ns), out)
}

const EXECUTE: &str = include_str!("client_execute.cs.txt");
