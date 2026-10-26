//! `src/Client.php` (the client options, the top-level services, the default headers and the
//! credentials, in the headers or the query) and `src/Version.php`.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ApiKeyLocation, AuthRole, ClientOpt, HttpAuthScheme, OptType, SecurityScheme};

use super::Gen;
use super::file::{docblock, php_file};
use crate::layout::{cat, indent, list, render, text};
use crate::naming::{camel, words};
use crate::types::{ClassPath, literal};

/// `src/Client.php` and `src/Version.php`.
#[must_use]
pub fn client_files(g: &Gen<'_>) -> Vec<(String, String)> {
    vec![
        ("src/Client.php".to_owned(), client(g)),
        ("src/Version.php".to_owned(), version(g)),
    ]
}

fn version(g: &Gen<'_>) -> String {
    format!(
        "<?php\n\ndeclare(strict_types=1);\n\nnamespace {};\n\n// x-release-please-start-version\nconst VERSION = '{}';\n// x-release-please-end\n",
        g.ns, g.target.version
    )
}

fn opt_type(o: &ClientOpt) -> &'static str {
    match o.ty {
        OptType::String => "string",
        OptType::Number => "float",
        OptType::Integer => "int",
        OptType::Boolean => "bool",
    }
}

/// `$this->x = (string) ($x ?? Util::getenv('X') ?: 'default');`.
fn opt_assignment(o: &ClientOpt, name: &str, util: &str) -> String {
    let cast = format!("({})", opt_type(o));
    let mut parts = vec![text(format!("$this->{name} = {cast} (${name} ?? "))];
    if let Some(env) = &o.read_env {
        parts.push(list(format!("{util}::getenv("), vec![text(format!("'{env}'"))], ")"));
    }
    if let Some(d) = o.default.as_ref().and_then(default_literal) {
        if o.read_env.is_some() {
            parts.push(text(format!(" ?: {d}")));
        } else {
            parts.push(text(d));
        }
    } else if o.read_env.is_none() {
        parts.push(text("null"));
    }
    parts.push(text(");"));
    format!("{}{}", indent(2), render(&cat(parts), 2, 4, 0))
}

fn default_literal(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(literal(&ferrule_sdk_model::model::Literal::String(s.clone()))),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Where the client's credentials go: `authHeaders()` merged into the headers, or `authQuery()`
/// into the query (an API key `in: query`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthIn {
    Headers,
    Query,
}

/// Where the credentials go and the body of the method returning them, from the client's
/// security schemes and the options that fill them. An API key in a cookie is not sent, as the
/// Python plugin does not send it.
fn auth(g: &Gen<'_>, opts: &[(String, &ClientOpt)]) -> Option<(AuthIn, String)> {
    let s = g.client.settings.as_ref()?;
    let names: Vec<&String> = s.security.iter().flatten().collect();
    let scheme = s.security_schemes.iter().find(|n| names.contains(&&n.name))?;
    let of = |role: AuthRole| {
        opts.iter()
            .find(|(_, o)| {
                o.auth
                    .as_ref()
                    .is_some_and(|a| a.security_scheme == scheme.name && a.role == role)
            })
            .map(|(n, _)| n.clone())
    };
    let ind2 = indent(2);
    let ind3 = indent(3);
    match &scheme.scheme {
        SecurityScheme::Http {
            scheme: HttpAuthScheme::Basic,
            ..
        } => {
            let user = of(AuthRole::Username)?;
            let pass = of(AuthRole::Password)?;
            Some((
                AuthIn::Headers,
                format!(
                    "{ind2}if (!$this->{user} && !$this->{pass}) {{\n{ind3}return [];\n{ind2}}}\n\n{ind2}$base64_credentials = base64_encode(\n{ind3}\"{{$this->{user}}}:{{$this->{pass}}}\"\n{ind2});\n\n{ind2}return ['Authorization' => \"Basic {{$base64_credentials}}\"];\n"
                ),
            ))
        }
        SecurityScheme::Http {
            scheme: HttpAuthScheme::Bearer,
            ..
        } => {
            let token = of(AuthRole::Value)?;
            Some((
                AuthIn::Headers,
                format!(
                    "{ind2}if (!$this->{token}) {{\n{ind3}return [];\n{ind2}}}\n\n{ind2}return ['Authorization' => \"Bearer {{$this->{token}}}\"];\n"
                ),
            ))
        }
        SecurityScheme::ApiKey {
            location: ApiKeyLocation::Header,
            name,
        } => {
            let key = of(AuthRole::Value)?;
            Some((
                AuthIn::Headers,
                format!(
                    "{ind2}if (!$this->{key}) {{\n{ind3}return [];\n{ind2}}}\n\n{ind2}return ['{name}' => $this->{key}];\n"
                ),
            ))
        }
        // `authQuery()` as Stainless's PHP SDKs write it (nps-php, hubspot-sdk-php).
        SecurityScheme::ApiKey {
            location: ApiKeyLocation::Query,
            name,
        } => {
            let key = of(AuthRole::Value)?;
            Some((
                AuthIn::Query,
                format!("{ind2}return $this->{key} ? ['{name}' => $this->{key}] : [];\n"),
            ))
        }
        _ => None,
    }
}

/// `authHeaders()` or `authQuery()`, and the `buildRequest()` that merges what it returns; none
/// without credentials.
fn auth_members(g: &Gen<'_>, opts: &[(String, &ClientOpt)], request_options: &str) -> Vec<String> {
    let Some((into, body)) = auth(g, opts) else {
        return Vec::new();
    };
    let (func, query, headers) = match into {
        AuthIn::Headers => ("authHeaders", "$query", "[...$this->authHeaders(), ...$headers]"),
        AuthIn::Query => ("authQuery", "[...$this->authQuery(), ...$query]", "$headers"),
    };
    vec![
        format!(
            "    /** @return array<string,string> */\n    protected function {func}(): array\n    {{\n{body}    }}\n"
        ),
        format!(
            "    /**\n     * @internal\n     *\n     * @param string|list<string> $path\n     * @param array<string,mixed> $query\n     * @param array<string,string|int|list<string|int>|null> $headers\n     * @param RequestOpts|null $opts\n     *\n     * @return array{{NormalizedRequest, {request_options}}}\n     */\n    protected function buildRequest(\n        string $method,\n        string|array $path,\n        array $query,\n        array $headers,\n        mixed $body,\n        {request_options}|array|null $opts,\n    ): array {{\n        return parent::buildRequest(\n            method: $method,\n            path: $path,\n            query: {query},\n            headers: {headers},\n            body: $body,\n            opts: $opts,\n        );\n    }}\n"
        ),
    ]
}

fn client(g: &Gen<'_>) -> String {
    let path = ClassPath::new(vec!["Client".to_owned()]);
    let mut imp = g.imports(&[], Some(&path));
    let casings = g.casings();
    let s = g.client.settings.as_ref();
    let opts: Vec<(String, &ClientOpt)> = s
        .map(|s| s.opts.iter().map(|o| (camel(&words(&o.name), casings), o)).collect())
        .unwrap_or_default();
    let psr17 = imp.external("Http\\Discovery\\Psr17FactoryDiscovery");
    let psr18 = imp.external("Http\\Discovery\\Psr18ClientDiscovery");
    let base = imp.runtime(&["Core", "BaseClient"]);
    let streaming = imp.runtime(&["Core", "Implementation", "StreamingHttpClient"]);
    let util = imp.runtime(&["Core", "Util"]);
    let mut members: Vec<String> = Vec::new();
    for (name, o) in &opts {
        members.push(format!("    public {} ${name};\n", opt_type(o)));
    }
    let top: Vec<&super::Res<'_>> = g.resources.iter().filter(|r| r.segs.len() == 1).collect();
    for r in &top {
        let svc = imp.class(&r.class("Services", "Service"));
        members.push(format!(
            "    /**\n     * @api\n     */\n    public {svc} ${};\n",
            r.attr
        ));
    }
    let request_options = imp.class(&ClassPath::new(vec!["RequestOptions".to_owned()]));
    let mut params: Vec<String> = opts
        .iter()
        .map(|(n, o)| format!("?{} ${n} = null", opt_type(o)))
        .collect();
    params.push("?string $baseUrl = null".to_owned());
    params.push(format!("{request_options}|array|null $requestOptions = null"));
    let mut ctor = String::from("    /**\n     * @param RequestOpts|null $requestOptions\n     */\n");
    ctor.push_str(&super::model::signature("public function __construct", params, "", 1));
    ctor.push_str(" {\n");
    for (name, o) in &opts {
        ctor.push_str(&opt_assignment(o, name, &util));
        ctor.push('\n');
    }
    ctor.push('\n');
    let env_prefix = s.map_or_else(String::new, |s| s.env_prefix.clone());
    let base_url = g.client.base_url.clone();
    let base_stmt = cat(vec![
        text("$baseUrl ??= "),
        list(
            format!("{util}::getenv("),
            vec![text(format!("'{env_prefix}_BASE_URL'"))],
            ")",
        ),
        text(format!(" ?: '{base_url}';")),
    ]);
    let _ = write!(ctor, "{}{}\n\n", indent(2), render(&base_stmt, 2, 4, 0));
    let _ = write!(
        ctor,
        "        $options = {request_options}::parse(\n            {request_options}::with(\n                uriFactory: {psr17}::findUriFactory(),\n                streamFactory: {psr17}::findStreamFactory(),\n                requestFactory: {psr17}::findRequestFactory(),\n                transporter: {psr18}::find(),\n            ),\n            $requestOptions,\n        );\n\n"
    );
    let _ = write!(
        ctor,
        "        if (is_null($options->streamingTransporter)) {{\n            assert(!is_null($options->transporter));\n            $options->streamingTransporter = new {streaming}($options->transporter);\n        }}\n\n"
    );
    let org = s.map_or_else(|| g.ns.clone(), |s| s.name.written.clone());
    let _ = write!(
        ctor,
        "        /** @var array<string, string|null> $headers */\n        $headers = [\n            'Content-Type' => 'application/json',\n            'Accept' => 'application/json',\n            'User-Agent' => sprintf('{org}/PHP %s', VERSION),\n            'X-Stainless-Lang' => 'php',\n            'X-Stainless-Package-Version' => '{}',\n            'X-Stainless-Arch' => {util}::machtype(),\n            'X-Stainless-OS' => {util}::ostype(),\n            'X-Stainless-Runtime' => php_sapi_name(),\n            'X-Stainless-Runtime-Version' => phpversion(),\n        ];\n\n",
        g.target.version
    );
    let _ = write!(
        ctor,
        "        $customHeadersEnv = {util}::getenv('{env_prefix}_CUSTOM_HEADERS');\n        if (null !== $customHeadersEnv) {{\n            foreach (explode(\"\\n\", $customHeadersEnv) as $line) {{\n                $colon = strpos($line, ':');\n                if (false !== $colon) {{\n                    $headers[trim(substr($line, 0, $colon))] = trim(substr($line, $colon + 1));\n                }}\n            }}\n        }}\n\n"
    );
    ctor.push_str("        parent::__construct(\n            headers: $headers,\n            baseUrl: $baseUrl,\n            options: $options\n        );\n");
    if !top.is_empty() {
        ctor.push('\n');
    }
    for r in &top {
        let svc = imp.class(&r.class("Services", "Service"));
        let _ = writeln!(ctor, "        $this->{} = new {svc}($this);", r.attr);
    }
    ctor.push_str("    }\n");
    members.push(ctor);
    members.extend(auth_members(g, &opts, &request_options));
    let head = vec![
        format!(
            "@phpstan-import-type NormalizedRequest from \\{}\\Core\\BaseClient",
            g.ns
        ),
        format!("@phpstan-import-type RequestOpts from \\{}\\RequestOptions", g.ns),
    ];
    let mut body = docblock(&[head], 0);
    let _ = writeln!(body, "class Client extends {base}\n{{");
    body.push_str(&members.join("\n"));
    body.push_str("}\n");
    php_file(&g.ns, &imp, &body)
}
