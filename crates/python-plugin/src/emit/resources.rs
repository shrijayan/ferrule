//! `resources/<name>.py`: the sync and async resource classes and their raw/streaming wrappers.
//! One pass per class with a sync/async mode flag (CLAUDE.md: DRY across sync and async).
//! Port of `python-plugin/src/emit/resources.ts`.

use std::collections::HashSet;

use super::common::{
    HEADER, args_docstring, leaf, method_params, method_ref, py_string, rel_import, resource_class_base,
    resource_module, return_model, stainless_method_order,
};
use super::format::{Bracket, Expr, bracket, call, print_expr};
use super::models::all_list;
use super::params::{ParamsContext, name_params};
use crate::diagnostics::{FerCode, FerruleError};
use crate::model::{HttpVerb, ParamLocation, SdkClient, SdkMethod, SdkParam, SdkResource, SdkType};
use crate::printer::PythonPrinter;
use crate::type_mapper::{AnnotationContext, is_nullable, to_annotation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Sync,
    Async,
}

impl Mode {
    pub const BOTH: [Self; 2] = [Self::Sync, Self::Async];

    fn is_async(self) -> bool {
        self == Self::Async
    }

    /// `Async` or the empty string.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        if self.is_async() { "Async" } else { "" }
    }
}

const OPTIONS_ARGS: [&str; 4] = [
    "extra_headers=extra_headers",
    "extra_query=extra_query",
    "extra_body=extra_body",
    "timeout=timeout",
];

/// `AsyncStopResourceWithRawResponse` and friends.
#[must_use]
pub fn class_name(resource: &SdkResource, mode: Mode, suffix: &str) -> String {
    format!("{}{}Resource{suffix}", mode.prefix(), resource_class_base(resource))
}

fn unsupported(method: &SdkMethod, what: &str) -> FerruleError {
    FerruleError::new(
        FerCode::Fer201,
        method.pointer.clone(),
        format!("{what} is not generated yet"),
        "leave the operation out, or file an issue with the spec fragment",
    )
}

struct FileState<'a> {
    here: &'a str,
    client: &'a SdkClient,
}

fn imp(s: &FileState<'_>, p: &mut PythonPrinter, target: &str, name: &str) {
    p.import_from(&rel_import(s.here, target), name);
}

/// One keyword argument of a method signature.
fn arg_line(s: &FileState<'_>, p: &mut PythonPrinter, q: &SdkParam, ctx: &dyn AnnotationContext) -> String {
    let mut ann = to_annotation(&q.ty, p, ctx);
    if q.required {
        return format!("{}: {ann},", q.py_name);
    }
    if !is_nullable(&q.ty) && matches!(q.ty, SdkType::Unknown) {
        "object".clone_into(&mut ann);
    }
    imp(s, p, "_types", "Omit");
    imp(s, p, "_types", "omit");
    format!("{}: {ann} | Omit = omit,", q.py_name)
}

/// `path_template("/a/{b_id}", b_id=b_id)` or the plain path string.
fn path_expr(s: &FileState<'_>, p: &mut PythonPrinter, method: &SdkMethod) -> Expr {
    if method.path_params.is_empty() {
        return py_string(&method.path).into();
    }
    imp(s, p, "_utils", "path_template");
    let mut template = method.path.clone();
    for q in &method.path_params {
        template = template.replacen(&format!("{{{}}}", q.wire_name), &format!("{{{}}}", q.py_name), 1);
    }
    let mut items: Vec<Expr> = vec![py_string(&template).into()];
    items.extend(method.path_params.iter().map(|q| format!("{0}={0}", q.py_name).into()));
    call("path_template(", items, false).into()
}

/// `maybe_transform({...}, module.Params)`; awaited in async methods that await.
fn transform_expr(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    key: &str,
    params: &[&SdkParam],
    params_name: &str,
    awaits: bool,
) -> Bracket {
    let f = if awaits {
        "async_maybe_transform"
    } else {
        "maybe_transform"
    };
    imp(s, p, "_utils", f);
    let dict = bracket(
        "{",
        params
            .iter()
            .map(|q| format!("{}: {}", py_string(&q.py_name), q.py_name).into())
            .collect(),
        params.len() > 1,
        "}",
    );
    call(
        &format!("{key}={}{f}(", if awaits { "await " } else { "" }),
        vec![dict.into(), params_name.into()],
        false,
    )
}

fn emit_method(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    method: &SdkMethod,
    mode: Mode,
    holder: Option<&SdkResource>,
) -> Result<(), FerruleError> {
    let is_async = mode.is_async();
    let page = method.pagination.as_ref();
    let stream = method.streaming.as_ref();
    if stream
        .and_then(|st| st.param_discriminator.as_deref())
        .is_some_and(|d| !d.is_empty())
    {
        return Err(unsupported(method, "a param-switched streaming method"));
    }
    if method.body.is_some() && method.body_params.is_empty() {
        return Err(unsupported(method, "a request body that is not an object"));
    }
    let ret = return_model(holder, method);
    let params = method_params(method);
    let params_ref = if params.is_empty() {
        None
    } else {
        Some(method_ref(holder, method, "Params"))
    };
    let leaf_module = params_ref
        .as_ref()
        .map_or("", |r| r.module.rsplit('.').next().unwrap_or(""));
    let named = name_params(&params, "");
    let ctx = ParamsContext {
        named: &named,
        qualify: format!("{leaf_module}."),
        here: s.here.to_owned(),
        shadowed: HashSet::new(),
    };

    let model = ret
        .as_ref()
        .map_or_else(|| "None".to_owned(), |r| ctx.ref_name(p, &r.r#ref));
    let mut returns = model.clone();
    let sync_or_async = if is_async { "Async" } else { "Sync" };
    if let Some(page) = page {
        let cls = format!("{sync_or_async}{}", page.page);
        imp(s, p, "pagination", &cls);
        if is_async {
            imp(s, p, "_base_client", "AsyncPaginator");
        }
        returns = if is_async {
            format!("AsyncPaginator[{model}, {cls}[{model}]]")
        } else {
            format!("{cls}[{model}]")
        };
    } else if stream.is_some() {
        let cls = if is_async { "AsyncStream" } else { "Stream" };
        imp(s, p, "_streaming", cls);
        returns = format!("{cls}[{model}]");
    }
    // A paginated async method is a plain def returning an AsyncPaginator.
    let awaits = is_async && page.is_none();

    let opened = {
        p.line(&format!("{}def {}(", if awaits { "async " } else { "" }, method.name));
        p.open()
    };
    p.line("self,");
    for q in &method.path_params {
        let l = arg_line(s, p, q, &ctx);
        p.line(&l);
    }
    p.line("*,");
    for q in &params {
        let l = arg_line(s, p, q, &ctx);
        p.line(&l);
    }
    p.line(
        "# Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.",
    );
    p.line("# The extra values given here take precedence over values defined on the client or passed to this method.");
    p.line("extra_headers: Headers | None = None,");
    p.line("extra_query: Query | None = None,");
    p.line("extra_body: Body | None = None,");
    p.line("timeout: float | httpx.Timeout | None | NotGiven = not_given,");
    p.close(opened);
    p.line(&format!(") -> {returns}:"));

    let opened = p.open();
    let has_args = !method.path_params.is_empty() || !params.is_empty();
    for l in args_docstring(method, if has_args { Some(&params) } else { None }) {
        p.line(&l);
    }
    for q in &method.path_params {
        p.block(&format!("if not {}:", q.py_name), |p| {
            p.line(&format!(
                "raise ValueError(f\"Expected a non-empty value for `{0}` but received {{{0}!r}}\")",
                q.py_name
            ));
        });
    }
    if ret.is_none() && stream.is_none() {
        p.line("extra_headers = {\"Accept\": \"*/*\", **(extra_headers or {})}");
    }
    let params_name = params_ref.as_ref().map_or_else(String::new, |r| {
        format!("{}.{}", r.module.rsplit('.').next().unwrap_or(""), r.name)
    });
    if let Some(r) = &params_ref {
        let parts: Vec<&str> = r.module.split('.').collect();
        let mut target = vec!["types"];
        target.extend(&parts[..parts.len() - 1]);
        imp(s, p, &target.join("."), parts[parts.len() - 1]);
    }
    let query: Vec<&SdkParam> = params
        .iter()
        .copied()
        .filter(|q| method.query_params.iter().any(|x| std::ptr::eq(x, *q)))
        .collect();
    let body: Vec<&SdkParam> = params
        .iter()
        .copied()
        .filter(|q| method.body_params.iter().any(|x| std::ptr::eq(x, *q)))
        .collect();
    let in_body =
        page.is_some_and(|pg| pg.param_location == ParamLocation::Body) || method.http_method != HttpVerb::Get;
    let mut items: Vec<Expr> = vec![path_expr(s, p, method)];
    if let Some(page) = page {
        items.push(format!("page={sync_or_async}{}[{model}]", page.page).into());
    }
    if !body.is_empty() && in_body {
        items.push(transform_expr(s, p, "body", &body, &params_name, awaits && page.is_none()).into());
    }
    imp(s, p, "_base_client", "make_request_options");
    let options: Vec<Expr> = OPTIONS_ARGS.iter().map(|&a| a.into()).collect();
    if query.is_empty() {
        items.push(call("options=make_request_options(", options, false).into());
    } else {
        let mut options = options;
        options.push(transform_expr(s, p, "query", &query, &params_name, awaits).into());
        items.push(call("options=make_request_options(", options, true).into());
    }
    if page.is_some() {
        items.push(format!("model={model}").into());
    } else if ret.is_none() && stream.is_none() {
        imp(s, p, "_types", "NoneType");
        items.push("cast_to=NoneType".into());
    } else {
        items.push(format!("cast_to={model}").into());
    }
    if page.is_some() && method.http_method != HttpVerb::Get {
        items.push(format!("method=\"{}\"", method.http_method.as_str()).into());
    }
    if stream.is_some() {
        items.push("stream=True".into());
        items.push(
            format!(
                "stream_cls={}[{model}]",
                if is_async { "AsyncStream" } else { "Stream" }
            )
            .into(),
        );
    }
    let f = if page.is_some() {
        "_get_api_list".to_owned()
    } else {
        format!("_{}", method.http_method.as_str())
    };
    print_expr(
        p,
        &call(&format!("self.{f}("), items, true).into(),
        &format!("return {}", if awaits { "await " } else { "" }),
        "",
    );
    p.close(opened);
    Ok(())
}

fn emit_resource_class(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    resource: &SdkResource,
    mode: Mode,
) -> Result<(), FerruleError> {
    let docs = format!("https://www.github.com/{}", s.client.package.repo);
    imp(
        s,
        p,
        "_resource",
        if mode.is_async() {
            "AsyncAPIResource"
        } else {
            "SyncAPIResource"
        },
    );
    imp(s, p, "_compat", "cached_property");
    p.line(&format!(
        "class {}({}APIResource):",
        class_name(resource, mode, ""),
        if mode.is_async() { "Async" } else { "Sync" }
    ));
    let opened = p.open();
    for sub in &resource.subresources {
        imp(s, p, &resource_module(sub), &class_name(sub, mode, ""));
        p.line("@cached_property");
        p.block(
            &format!("def {}(self) -> {}:", leaf(sub), class_name(sub, mode, "")),
            |p| {
                p.line(&format!("return {}(self._client)", class_name(sub, mode, "")));
            },
        );
        p.blank();
    }
    let raw = class_name(resource, mode, "WithRawResponse");
    p.line("@cached_property");
    p.block(&format!("def with_raw_response(self) -> {raw}:"), |p| {
        p.line("\"\"\"");
        p.line("This property can be used as a prefix for any HTTP method call to return");
        p.line("the raw response object instead of the parsed content.");
        p.blank();
        p.line(&format!(
            "For more information, see {docs}#accessing-raw-response-data-eg-headers"
        ));
        p.line("\"\"\"");
        p.line(&format!("return {raw}(self)"));
    });
    p.blank();
    let streaming = class_name(resource, mode, "WithStreamingResponse");
    p.line("@cached_property");
    p.block(&format!("def with_streaming_response(self) -> {streaming}:"), |p| {
        p.line("\"\"\"");
        p.line("An alternative to `.with_raw_response` that doesn't eagerly read the response body.");
        p.blank();
        p.line(&format!("For more information, see {docs}#with_streaming_response"));
        p.line("\"\"\"");
        p.line(&format!("return {streaming}(self)"));
    });
    for method in stainless_method_order(&resource.methods) {
        p.blank();
        emit_method(s, p, method, mode, Some(resource))?;
    }
    p.close(opened);
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum WrapperKind {
    Raw,
    Streamed,
}

fn emit_wrapper(s: &FileState<'_>, p: &mut PythonPrinter, resource: &SdkResource, mode: Mode, kind: WrapperKind) {
    let attr = leaf(resource);
    let (suffix, word) = match kind {
        WrapperKind::Raw => ("WithRawResponse", "raw"),
        WrapperKind::Streamed => ("WithStreamingResponse", "streamed"),
    };
    let f = format!(
        "{}to_{word}_response_wrapper",
        if mode.is_async() { "async_" } else { "" }
    );
    p.line(&format!("class {}:", class_name(resource, mode, suffix)));
    let opened = p.open();
    p.line(&format!(
        "def __init__(self, {attr}: {}) -> None:",
        class_name(resource, mode, "")
    ));
    let init = p.open();
    p.line(&format!("self._{attr} = {attr}"));
    let methods = stainless_method_order(&resource.methods);
    if !methods.is_empty() {
        imp(s, p, "_response", &f);
        p.blank();
    }
    for method in methods {
        p.block(&format!("self.{} = {f}(", method.name), |p| {
            p.line(&format!("{attr}.{},", method.name));
        });
        p.line(")");
    }
    p.close(init);
    for sub in &resource.subresources {
        let cls = class_name(sub, mode, suffix);
        imp(s, p, &resource_module(sub), &cls);
        p.blank();
        p.line("@cached_property");
        p.block(&format!("def {}(self) -> {cls}:", leaf(sub)), |p| {
            print_expr(
                p,
                &call(
                    &format!("{cls}("),
                    vec![format!("self._{attr}.{}", leaf(sub)).into()],
                    false,
                )
                .into(),
                "return ",
                "",
            );
        });
    }
    p.close(opened);
}

/// Renders the module of one resource (`resources/<path>.py`, or `<path>/<name>.py` with children).
///
/// # Errors
///
/// FER201 for a method this emitter cannot generate yet.
pub fn emit_resource_file(client: &SdkClient, resource: &SdkResource) -> Result<String, FerruleError> {
    let mut p = PythonPrinter::new(&[HEADER]);
    let here = resource_module(resource);
    let s = FileState { here: &here, client };
    p.import_from("__future__", "annotations");
    p.import_module("httpx");
    for name in ["Body", "Query", "Headers", "NotGiven", "not_given"] {
        imp(&s, &mut p, "_types", name);
    }
    // Imports render above the body whatever order they are registered in.
    for l in all_list(&[
        class_name(resource, Mode::Sync, ""),
        class_name(resource, Mode::Async, ""),
    ]) {
        p.line(&l);
    }
    for mode in Mode::BOTH {
        p.blank().blank();
        emit_resource_class(&s, &mut p, resource, mode)?;
    }
    for (mode, kind) in [
        (Mode::Sync, WrapperKind::Raw),
        (Mode::Async, WrapperKind::Raw),
        (Mode::Sync, WrapperKind::Streamed),
        (Mode::Async, WrapperKind::Streamed),
    ] {
        p.blank().blank();
        emit_wrapper(&s, &mut p, resource, mode, kind);
    }
    Ok(p.render())
}

/// Emits `$client` methods into a client class (used by the client emitter).
///
/// # Errors
///
/// FER201 for a method this emitter cannot generate yet.
pub fn emit_client_methods(
    p: &mut PythonPrinter,
    client: &SdkClient,
    here: &str,
    mode: Mode,
) -> Result<(), FerruleError> {
    let s = FileState { here, client };
    for method in stainless_method_order(&client.client_methods) {
        p.blank();
        emit_method(&s, p, method, mode, None)?;
    }
    Ok(())
}
