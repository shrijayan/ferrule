//! `_client.py`: the sync and async client classes, their raw/streaming wrappers and the
//! `Client`/`AsyncClient` aliases, from `SdkClientSettings` and the resource tree. One pass per
//! class with a sync/async mode flag. Port of `python-plugin/src/emit/client.ts`.

use super::common::{HEADER, leaf, py_fstring, py_string, resource_module};
use super::format::{call, print_expr};
use super::models::all_list;
use super::resources::{Mode, class_name, emit_client_methods};
use crate::diagnostics::{FerCode, FerruleError};
use crate::js;
use crate::model::{
    ApiKeyLocation, HttpAuthScheme, OptType, SdkClient, SdkClientOpt, SdkClientSettings, SdkResource,
    SdkSecurityScheme, SendLocation,
};
use crate::printer::{LINE_LENGTH, PythonPrinter};

const HERE: &str = "_client";

const STATUS_ERRORS: [(&str, &str); 8] = [
    ("== 400", "BadRequestError"),
    ("== 401", "AuthenticationError"),
    ("== 403", "PermissionDeniedError"),
    ("== 404", "NotFoundError"),
    ("== 409", "ConflictError"),
    ("== 422", "UnprocessableEntityError"),
    ("== 429", "RateLimitError"),
    (">= 500", "InternalServerError"),
];

fn py_type(t: OptType) -> &'static str {
    match t {
        OptType::String => "str",
        OptType::Number => "float",
        OptType::Integer => "int",
        OptType::Boolean => "bool",
    }
}

fn opt_type(opt: &SdkClientOpt) -> String {
    let t = py_type(opt.ty);
    if opt.nullable {
        format!("{t} | None")
    } else {
        t.to_owned()
    }
}

/// `resources.generations` for the package of a resource with subresources.
fn package_module(r: &SdkResource) -> String {
    let mut parts = vec!["resources"];
    parts.extend(r.path.iter().map(String::as_str));
    parts.join(".")
}

struct ClientState<'a> {
    client: &'a SdkClient,
    settings: &'a SdkClientSettings,
}

impl ClientState<'_> {
    fn client_name(&self, mode: Mode, suffix: &str) -> String {
        format!("{}{}{suffix}", mode.prefix(), self.settings.class_name)
    }

    fn error_name(&self) -> String {
        format!("{}Error", self.settings.class_name)
    }

    fn scheme(&self, name: &str) -> Option<&SdkSecurityScheme> {
        self.settings
            .security_schemes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, s)| s)
    }
}

fn lines(p: &mut PythonPrinter, ls: &[&str]) {
    for l in ls {
        p.line(l);
    }
}

fn emit_init(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode) -> Result<(), FerruleError> {
    let settings = s.settings;
    let is_async = mode == Mode::Async;
    if settings.environments.len() > 1 {
        return Err(FerruleError::new(
            FerCode::Fer201,
            "#/x-ferrule-client/environments",
            "a client with several environments is not generated yet",
            "keep one environment in stainless.yml",
        ));
    }
    p.block("def __init__(", |p| {
        p.line("self,");
        p.line("*,");
        for o in &settings.opts {
            p.line(&format!("{}: {} | None = None,", o.name, py_type(o.ty)));
        }
        lines(
            p,
            &[
                "base_url: str | httpx.URL | None = None,",
                "timeout: float | Timeout | None | NotGiven = not_given,",
                "max_retries: int = DEFAULT_MAX_RETRIES,",
                "default_headers: Mapping[str, str] | None = None,",
                "default_query: Mapping[str, object] | None = None,",
                "# Configure a custom httpx client.",
            ],
        );
        p.line(&format!(
            "# We provide a `Default{}HttpxClient` class that you can pass to retain the default values we use for `limits`, `timeout` & `follow_redirects`.",
            if is_async { "Async" } else { "" }
        ));
        p.line(&format!(
            "# See the [httpx documentation](https://www.python-httpx.org/api/#{}) for more details.",
            if is_async { "asyncclient" } else { "client" }
        ));
        p.line(&format!("http_client: httpx.{} | None = None,", if is_async { "AsyncClient" } else { "Client" }));
        lines(
            p,
            &[
                "# Enable or disable schema validation for data returned by the API.",
                "# When enabled an error APIResponseValidationError is raised",
                "# if the API responds with invalid data for the expected schema.",
                "#",
                "# This parameter may be removed or changed in the future.",
                "# If you rely on this feature, please open a GitHub issue",
                "# outlining your use-case to help us decide if it should be",
                "# part of our public interface in the future.",
                "_strict_response_validation: bool = False,",
            ],
        );
    });
    p.line(") -> None:");
    p.indented(|p| emit_init_body(s, p, mode));
    Ok(())
}

/// The body of `__init__`: docstring, options from the environment, base URL and headers.
fn emit_init_body(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode) {
    let settings = s.settings;
    let is_async = mode == Mode::Async;
    let env = settings.environments.first();
    {
        let from_env: Vec<&SdkClientOpt> = settings.opts.iter().filter(|o| o.read_env.is_some()).collect();
        p.line(&format!(
            "\"\"\"Construct a new {} {} client instance.",
            if is_async { "async" } else { "synchronous" },
            s.client_name(mode, "")
        ));
        if let [only] = from_env.as_slice() {
            p.blank();
            p.line(&format!(
                "This automatically infers the `{}` argument from the `{}` environment variable if it is not provided.",
                only.name,
                only.read_env.as_deref().unwrap_or("")
            ));
        } else if from_env.len() > 1 {
            p.blank();
            p.line(
                "This automatically infers the following arguments from their corresponding environment variables if they are not provided:",
            );
            for o in &from_env {
                p.line(&format!(
                    "- `{}` from `{}`",
                    o.name,
                    o.read_env.as_deref().unwrap_or("")
                ));
            }
        }
        p.line("\"\"\"");
        for o in &settings.opts {
            if let Some(env) = &o.read_env {
                p.block(&format!("if {} is None:", o.name), |p| {
                    p.line(&format!("{} = os.environ.get({})", o.name, py_string(env)));
                });
            }
            if !o.nullable {
                p.import_from("._exceptions", &s.error_name());
                let how = o.read_env.as_ref().map_or_else(
                    || format!("by passing {} to the client", o.name),
                    |env| {
                        format!(
                            "either by passing {} to the client or by setting the {env} environment variable",
                            o.name
                        )
                    },
                );
                p.block(&format!("if {} is None:", o.name), |p| {
                    let message = py_string(&format!("The {} client option must be set {how}", o.name));
                    print_expr(
                        p,
                        &call(&format!("raise {}(", s.error_name()), vec![message.into()], false).into(),
                        "",
                        "",
                    );
                });
            }
            p.line(&format!("self.{0} = {0}", o.name));
            p.blank();
        }
        p.block("if base_url is None:", |p| {
            p.line(&format!(
                "base_url = os.environ.get(\"{}_BASE_URL\")",
                settings.env_prefix
            ));
        });
        p.block("if base_url is None:", |p| {
            p.line(&format!(
                "base_url = {}",
                py_fstring(env.map_or(s.client.base_url.as_str(), |e| e.url.as_str()))
            ));
        });
        p.blank();
        emit_init_headers(settings, p);
    }
}

/// The end of `__init__`: headers from the environment and the base class call.
fn emit_init_headers(settings: &SdkClientSettings, p: &mut PythonPrinter) {
    {
        p.import_from("._utils", "is_mapping_t");
        p.line(&format!(
            "custom_headers_env = os.environ.get(\"{}_CUSTOM_HEADERS\")",
            settings.env_prefix
        ));
        p.block("if custom_headers_env is not None:", |p| {
            p.line("parsed: dict[str, str] = {}");
            p.block("for line in custom_headers_env.split(\"\\n\"):", |p| {
                p.line("colon = line.find(\":\")");
                p.block("if colon >= 0:", |p| {
                    p.line("parsed[line[:colon].strip()] = line[colon + 1 :].strip()");
                });
            });
            p.line("default_headers = {**parsed, **(default_headers if is_mapping_t(default_headers) else {})}");
        });
        p.blank();
        p.block("super().__init__(", |p| {
            lines(
                p,
                &[
                    "version=__version__,",
                    "base_url=base_url,",
                    "max_retries=max_retries,",
                    "timeout=timeout,",
                    "http_client=http_client,",
                    "custom_headers=default_headers,",
                    "custom_query=default_query,",
                    "_strict_response_validation=_strict_response_validation,",
                ],
            );
        });
        p.line(")");
    }
}

fn emit_resource_properties(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode) {
    for r in &s.client.resources {
        let cls = class_name(r, mode, "");
        p.blank();
        p.line("@cached_property");
        p.block(&format!("def {}(self) -> {cls}:", leaf(r)), |p| {
            import_line(p, &format!(".{}", package_module(r)), &cls);
            p.blank();
            p.line(&format!("return {cls}(self)"));
        });
    }
}

fn auth_lines(s: &ClientState<'_>) -> Option<Vec<String>> {
    let settings = s.settings;
    for alt in &settings.security {
        for name in alt {
            let scheme = s.scheme(name);
            let opt = settings
                .opts
                .iter()
                .find(|o| o.auth.as_ref().is_some_and(|a| a.security_scheme == *name));
            let (Some(scheme), Some(opt)) = (scheme, opt) else {
                continue;
            };
            match scheme {
                SdkSecurityScheme::Http {
                    scheme: HttpAuthScheme::Bearer,
                    ..
                } => {
                    return Some(vec![
                        format!("{0} = self.{0}", opt.name),
                        format!("return {{\"Authorization\": f\"Bearer {{{}}}\"}}", opt.name),
                    ]);
                }
                SdkSecurityScheme::ApiKey {
                    location: ApiKeyLocation::Header,
                    name: header,
                } => {
                    return Some(vec![
                        format!("{0} = self.{0}", opt.name),
                        format!("return {{{}: {}}}", py_string(header), opt.name),
                    ]);
                }
                _ => {}
            }
        }
    }
    None
}

fn emit_client_class(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode) -> Result<(), FerruleError> {
    let settings = s.settings;
    let is_async = mode == Mode::Async;
    p.line(&format!(
        "class {}({}APIClient):",
        s.client_name(mode, ""),
        if is_async { "Async" } else { "Sync" }
    ));
    let opened = p.open();
    if !settings.opts.is_empty() {
        p.line("# client options");
        for o in &settings.opts {
            p.line(&format!("{}: {}", o.name, opt_type(o)));
        }
        p.blank();
    }
    emit_init(s, p, mode)?;
    emit_resource_properties(s, p, mode);
    for (prop, suffix) in [
        ("with_raw_response", "WithRawResponse"),
        ("with_streaming_response", "WithStreamedResponse"),
    ] {
        p.blank();
        p.line("@cached_property");
        let name = s.client_name(mode, suffix);
        p.block(&format!("def {prop}(self) -> {name}:"), |p| {
            p.line(&format!("return {name}(self)"));
        });
    }
    emit_request_properties(s, p, mode);
    p.blank();
    emit_copy(s, p, mode);
    if !s.client.client_methods.is_empty() {
        emit_client_methods(p, s.client, HERE, mode)?;
    }
    for o in &settings.opts {
        let scheme = o.auth.as_ref().and_then(|a| s.scheme(&a.security_scheme));
        if matches!(
            scheme,
            Some(SdkSecurityScheme::ApiKey {
                location: ApiKeyLocation::Query,
                ..
            })
        ) {
            p.blank();
            p.block(&format!("def _get_{}_query_param(self) -> str | None:", o.name), |p| {
                p.line(&format!("return self.{}", o.name));
            });
        }
    }
    p.blank();
    p.line("@override");
    p.block("def _make_status_error(", |p| {
        lines(
            p,
            &[
                "self,",
                "err_msg: str,",
                "*,",
                "body: object,",
                "response: httpx.Response,",
            ],
        );
    });
    p.line(") -> APIStatusError:");
    p.indented(|p| {
        for (i, (cond, err)) in STATUS_ERRORS.iter().enumerate() {
            if i > 0 {
                p.blank();
            }
            p.block(&format!("if response.status_code {cond}:"), |p| {
                p.line(&format!(
                    "return _exceptions.{err}(err_msg, response=response, body=body)"
                ));
            });
        }
        p.line("return APIStatusError(err_msg, response=response, body=body)");
    });
    p.close(opened);
    Ok(())
}

/// The `qs`, `auth_headers`, `default_headers` and `default_query` properties.
fn emit_request_properties(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode) {
    let settings = s.settings;
    let is_async = mode == Mode::Async;
    p.blank();
    p.line("@property");
    p.line("@override");
    p.block("def qs(self) -> Querystring:", |p| {
        let format = match settings.array_format {
            crate::model::ArrayFormat::Comma => "comma",
            crate::model::ArrayFormat::Repeat => "repeat",
            crate::model::ArrayFormat::Indices => "indices",
            crate::model::ArrayFormat::Brackets => "brackets",
        };
        p.line(&format!("return Querystring(array_format={})", py_string(format)));
    });
    p.blank();
    p.line("@property");
    p.line("@override");
    p.block("def auth_headers(self) -> dict[str, str]:", |p| {
        for l in auth_lines(s).unwrap_or_else(|| vec!["return {}".to_owned()]) {
            p.line(&l);
        }
    });
    p.blank();
    p.line("@property");
    p.line("@override");
    p.block("def default_headers(self) -> dict[str, str | Omit]:", |p| {
        p.block("return {", |p| {
            p.line("**super().default_headers,");
            p.line(&format!(
                "\"X-Stainless-Async\": {},",
                if is_async {
                    "f\"async:{get_async_library()}\""
                } else {
                    "\"false\""
                }
            ));
            for (k, v) in &settings.default_headers {
                p.line(&format!("{}: {},", py_string(k), py_string(v)));
            }
            p.line("**self._custom_headers,");
        });
        p.line("}");
    });
    let query_opts: Vec<&SdkClientOpt> = settings
        .opts
        .iter()
        .filter(|o| o.send.as_ref().is_some_and(|x| x.location == SendLocation::Query))
        .collect();
    if !query_opts.is_empty() {
        p.blank();
        p.line("@property");
        p.line("@override");
        p.block("def default_query(self) -> dict[str, object]:", |p| {
            p.block("return {", |p| {
                p.line("**super().default_query,");
                for o in &query_opts {
                    let name = o.send.as_ref().map_or("", |x| x.name.as_str());
                    p.line(&format!("{}: self.{},", py_string(name), o.name));
                }
                p.line("**self._custom_query,");
            });
            p.line("}");
        });
    }
}

fn emit_copy(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode) {
    let settings = s.settings;
    p.block("def copy(", |p| {
        p.line("self,");
        p.line("*,");
        for o in &settings.opts {
            p.line(&format!("{}: {} | None = None,", o.name, py_type(o.ty)));
        }
        p.line("base_url: str | httpx.URL | None = None,");
        p.line("timeout: float | Timeout | None | NotGiven = not_given,");
        p.line(&format!(
            "http_client: httpx.{} | None = None,",
            if mode == Mode::Async { "AsyncClient" } else { "Client" }
        ));
        lines(
            p,
            &[
                "max_retries: int | NotGiven = not_given,",
                "default_headers: Mapping[str, str] | None = None,",
                "set_default_headers: Mapping[str, str] | None = None,",
                "default_query: Mapping[str, object] | None = None,",
                "set_default_query: Mapping[str, object] | None = None,",
                "_extra_kwargs: Mapping[str, Any] = {},",
            ],
        );
    });
    p.line(") -> Self:");
    p.indented(|p| {
        p.line("\"\"\"");
        p.line("Create a new client instance re-using the same options given to the current client with optional overriding.");
        p.line("\"\"\"");
        p.block("if default_headers is not None and set_default_headers is not None:", |p| {
            p.line("raise ValueError(\"The `default_headers` and `set_default_headers` arguments are mutually exclusive\")");
        });
        p.blank();
        p.block("if default_query is not None and set_default_query is not None:", |p| {
            p.line("raise ValueError(\"The `default_query` and `set_default_query` arguments are mutually exclusive\")");
        });
        p.blank();
        p.line("headers = self._custom_headers");
        p.block("if default_headers is not None:", |p| {
            p.line("headers = {**headers, **default_headers}");
        });
        p.block("elif set_default_headers is not None:", |p| {
            p.line("headers = set_default_headers");
        });
        p.blank();
        p.line("params = self._custom_query");
        p.block("if default_query is not None:", |p| {
            p.line("params = {**params, **default_query}");
        });
        p.block("elif set_default_query is not None:", |p| {
            p.line("params = set_default_query");
        });
        p.blank();
        p.line("http_client = http_client or self._client");
        p.block("return self.__class__(", |p| {
            for o in &settings.opts {
                p.line(&format!("{0}={0} or self.{0},", o.name));
            }
            lines(
                p,
                &[
                    "base_url=base_url or self.base_url,",
                    "timeout=self.timeout if isinstance(timeout, NotGiven) else timeout,",
                    "http_client=http_client,",
                    "max_retries=max_retries if is_given(max_retries) else self.max_retries,",
                    "default_headers=headers,",
                    "default_query=params,",
                    "**_extra_kwargs,",
                ],
            );
        });
        p.line(")");
    });
    p.blank();
    p.line("# Alias for `copy` for nicer inline usage, e.g.");
    p.line("# client.with_options(timeout=10).foo.create(...)");
    p.line("with_options = copy");
}

fn emit_wrapper_class(s: &ClientState<'_>, p: &mut PythonPrinter, mode: Mode, raw: bool) {
    let suffix = if raw { "WithRawResponse" } else { "WithStreamedResponse" };
    let res_suffix = if raw {
        "WithRawResponse"
    } else {
        "WithStreamingResponse"
    };
    p.block(&format!("class {}:", s.client_name(mode, suffix)), |p| {
        p.line(&format!("_client: {}", s.client_name(mode, "")));
        p.blank();
        p.block(
            &format!("def __init__(self, client: {}) -> None:", s.client_name(mode, "")),
            |p| {
                p.line("self._client = client");
            },
        );
        for r in &s.client.resources {
            let cls = class_name(r, mode, res_suffix);
            p.blank();
            p.line("@cached_property");
            self_def(p, leaf(r), &format!("{}.{cls}", leaf(r)));
            p.indented(|p| {
                import_line(p, &format!(".{}", package_module(r)), &cls);
                p.blank();
                print_expr(
                    p,
                    &call(
                        &format!("{cls}("),
                        vec![format!("self._client.{}", leaf(r)).into()],
                        false,
                    )
                    .into(),
                    "return ",
                    "",
                );
            });
        }
    });
}

/// `def name(self) -> T:`, split as ruff splits it when it is too long.
fn self_def(p: &mut PythonPrinter, name: &str, returns: &str) {
    let one = format!("def {name}(self) -> {returns}:");
    if js::len(&format!("{}{one}", p.indentation())) <= LINE_LENGTH {
        p.line(&one);
        return;
    }
    p.line(&format!("def {name}("));
    p.indented(|p| {
        p.line("self,");
    });
    p.line(&format!(") -> {returns}:"));
}

/// A function-local `from module import name`, wrapped as isort wraps it.
fn import_line(p: &mut PythonPrinter, module: &str, name: &str) {
    let one = format!("from {module} import {name}");
    if js::len(&format!("{}{one}", p.indentation())) <= LINE_LENGTH {
        p.line(&one);
        return;
    }
    p.line(&format!("from {module} import ("));
    p.indented(|p| {
        p.line(&format!("{name},"));
    });
    p.line(")");
}

/// Renders `_client.py`.
///
/// # Errors
///
/// FER103 without client settings, FER201 for what the emitter cannot generate yet.
pub fn emit_client_file(client: &SdkClient) -> Result<String, FerruleError> {
    let Some(settings) = &client.settings else {
        return Err(FerruleError::new(
            FerCode::Fer103,
            "#/x-ferrule-client",
            "missing x-ferrule-client",
            "regenerate the annotated spec with ferrule-convert",
        ));
    };
    let mut p = PythonPrinter::new(&[HEADER]);
    let s = ClientState { client, settings };
    p.import_from("__future__", "annotations");
    p.import_module("os");
    for n in ["TYPE_CHECKING", "Any", "Mapping"] {
        p.import_from("typing", n);
    }
    for n in ["Self", "override"] {
        p.import_from("typing_extensions", n);
    }
    p.import_module("httpx");
    p.import_from(".", "_exceptions");
    p.import_from("._qs", "Querystring");
    for n in [
        "Omit",
        "Timeout",
        "NotGiven",
        "Transport",
        "ProxiesTypes",
        "RequestOptions",
        "not_given",
    ] {
        p.import_from("._types", n);
    }
    for n in ["is_given", "get_async_library"] {
        p.import_from("._utils", n);
    }
    for m in ["._types", "._utils", "._base_client"] {
        p.explode(m);
    }
    p.import_from("._compat", "cached_property");
    p.import_from("._version", "__version__");
    p.import_from("._streaming", "Stream as Stream");
    p.import_from("._streaming", "AsyncStream as AsyncStream");
    p.import_from("._exceptions", "APIStatusError");
    for n in ["DEFAULT_MAX_RETRIES", "SyncAPIClient", "AsyncAPIClient"] {
        p.import_from("._base_client", n);
    }

    if !client.resources.is_empty() {
        p.block("if TYPE_CHECKING:", |p| {
            for l in type_checking_imports(client, &p.indentation()) {
                p.raw(&l);
            }
        });
        p.blank();
    }
    for l in all_list(&[
        "Timeout".to_owned(),
        "Transport".to_owned(),
        "ProxiesTypes".to_owned(),
        "RequestOptions".to_owned(),
        s.client_name(Mode::Sync, ""),
        s.client_name(Mode::Async, ""),
        "Client".to_owned(),
        "AsyncClient".to_owned(),
    ]) {
        p.line(&l);
    }
    for mode in Mode::BOTH {
        p.blank().blank();
        emit_client_class(&s, &mut p, mode)?;
    }
    for (mode, raw) in [
        (Mode::Sync, true),
        (Mode::Async, true),
        (Mode::Sync, false),
        (Mode::Async, false),
    ] {
        p.blank().blank();
        emit_wrapper_class(&s, &mut p, mode, raw);
    }
    p.blank().blank();
    p.line(&format!("Client = {}", s.client_name(Mode::Sync, "")));
    p.blank();
    p.line(&format!("AsyncClient = {}", s.client_name(Mode::Async, "")));
    Ok(p.render())
}

/// The `if TYPE_CHECKING:` imports, sorted like the module's own imports.
fn type_checking_imports(client: &SdkClient, indent: &str) -> Vec<String> {
    let mut p = PythonPrinter::default();
    for r in &client.resources {
        p.import_from(".resources", leaf(r));
        p.import_from(&format!(".{}", resource_module(r)), &class_name(r, Mode::Sync, ""));
        p.import_from(&format!(".{}", resource_module(r)), &class_name(r, Mode::Async, ""));
    }
    p.import_block(indent)
}
