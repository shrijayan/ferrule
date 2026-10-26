//! `resources/<name>.py`: the sync and async resource classes and their raw/streaming wrappers. One
//! pass per class with a sync/async mode flag (CLAUDE.md: DRY across sync and async).

use std::collections::HashSet;

use super::common::{
    HEADER, args_docstring, has_inline_object, header_params, is_binary_body, is_header, is_param_switched,
    keyword_path_params, leaf, method_args, method_ref, py_fstring, py_string, rel_import, resource_class_base,
    resource_module, return_model, stainless_method_order, stream_event_model, stream_switch, with_bool_literal,
};
use super::format::{Bracket, Cond, Expr, bracket, call, flat, print_expr};
use super::models::all_list;
use super::params::{ParamsContext, UnionBody, name_params, name_union_body};
use crate::diagnostics::{FerCode, FerruleError};
use crate::model::{
    HttpVerb, ParamLocation, SdkClient, SdkLiteral, SdkMethod, SdkParam, SdkPrimitive, SdkResource, SdkType,
};
use crate::naming::{pascal_case, split_words};
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
    /// The shared models the file's methods return (see [`ParamsContext::outputs`]).
    outputs: HashSet<String>,
}

/// The shared models `methods` return or stream, by class name.
fn shared_outputs<'m>(
    holder: Option<&SdkResource>,
    methods: impl IntoIterator<Item = &'m SdkMethod>,
) -> HashSet<String> {
    methods
        .into_iter()
        .flat_map(|m| {
            [
                return_model(holder, m).map(|r| r.r#ref),
                stream_event_model(holder, m).map(|r| r.r#ref),
            ]
        })
        .flatten()
        .filter(|r| r.module.starts_with("shared."))
        .map(|r| r.name)
        .collect()
}

fn imp(s: &FileState<'_>, p: &mut PythonPrinter, target: &str, name: &str) {
    p.import_from(&rel_import(s.here, target), name);
}

/// An argument of a method signature: its name, annotation and whether it has no default.
struct Arg {
    name: String,
    ann: String,
    required: bool,
}

fn arg_of(p: &mut PythonPrinter, q: &SdkParam, ctx: &dyn AnnotationContext) -> Arg {
    let mut ann = to_annotation(&q.ty, p, ctx);
    if !q.required && !is_nullable(&q.ty) && matches!(q.ty, SdkType::Unknown) {
        "object".clone_into(&mut ann);
    }
    Arg {
        name: q.py_name.clone(),
        ann,
        required: q.required,
    }
}

/// A keyword argument: the raw-bytes body as [`binary_arg`], any other as [`arg_of`].
fn keyword_arg(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    method: &SdkMethod,
    q: &SdkParam,
    is_async: bool,
    ctx: &dyn AnnotationContext,
) -> Arg {
    if is_binary_body(method, q) {
        binary_arg(s, p, q, is_async)
    } else {
        arg_of(p, q, ctx)
    }
}

/// The argument of a raw-bytes body: file content (a path is read) or bytes, an async iterable
/// of them for the async client.
fn binary_arg(s: &FileState<'_>, p: &mut PythonPrinter, q: &SdkParam, is_async: bool) -> Arg {
    let bytes = if is_async { "AsyncBinaryTypes" } else { "BinaryTypes" };
    imp(s, p, "_types", "FileContent");
    imp(s, p, "_types", bytes);
    Arg {
        name: q.py_name.clone(),
        ann: format!("FileContent | {bytes}"),
        required: q.required,
    }
}

/// One argument line of a method signature.
fn arg_line(s: &FileState<'_>, p: &mut PythonPrinter, a: &Arg) -> String {
    if a.required {
        return format!("{}: {},", a.name, a.ann);
    }
    let (ty, default) = s.client.style.optional();
    imp(s, p, "_types", ty);
    imp(s, p, "_types", default);
    format!("{}: {} | {ty} = {default},", a.name, a.ann)
}

/// `def name(self, <path params>, *, <args>, <extras>) -> returns:`.
fn print_signature(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    name: &str,
    awaits: bool,
    path: &[Arg],
    args: &[Arg],
    returns: &str,
) {
    let opened = {
        p.line(&format!("{}def {name}(", if awaits { "async " } else { "" }));
        p.open()
    };
    p.line("self,");
    for a in path {
        let l = arg_line(s, p, a);
        p.line(&l);
    }
    p.line("*,");
    for a in args {
        let l = arg_line(s, p, a);
        p.line(&l);
    }
    p.line(
        "# Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.",
    );
    p.line("# The extra values given here take precedence over values defined on the client or passed to this method.");
    p.line("extra_headers: Headers | None = None,");
    p.line("extra_query: Query | None = None,");
    p.line("extra_body: Body | None = None,");
    p.line(&format!(
        "timeout: float | httpx.Timeout | None | NotGiven = {},",
        s.client.style.not_given()
    ));
    p.close(opened);
    p.line(&format!(") -> {returns}:"));
}

/// The `@overload` stubs of a union body, one per variant (imagekit `files.update`), and the
/// implementation's arguments: each in every variant's spelling (`Literal["S3"] |
/// Literal["WEB_PROXY"]`), required where every variant requires it.
fn print_overloads(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    method: &SdkMethod,
    body: &UnionBody<'_>,
    sig: (bool, &[Arg], &str),
    ctx: &dyn AnnotationContext,
) -> Vec<Arg> {
    let (awaits, path, returns) = sig;
    let mut merged: Vec<Arg> = Vec::new();
    let mut seen: Vec<Vec<String>> = Vec::new();
    for v in &body.variants {
        let args: Vec<Arg> = v.args.iter().map(|q| arg_of(p, q, ctx)).collect();
        p.line("@overload");
        p.import_from("typing_extensions", "overload");
        print_signature(s, p, &method.name, awaits, path, &args, returns);
        let opened = p.open();
        // Path parameters are not listed, as in the method docstrings without overloads.
        let documented: Vec<&SdkParam> = v
            .args
            .iter()
            .copied()
            .filter(|q| !method.path_params.iter().any(|x| std::ptr::eq(x, *q)))
            .collect();
        for l in args_docstring(method, Some(&documented)) {
            p.line(&l);
        }
        p.line("...");
        p.close(opened);
        p.blank();
        let required: Vec<String> = args.iter().filter(|a| a.required).map(|a| a.name.clone()).collect();
        if !seen.contains(&required) {
            seen.push(required);
        }
        for a in args {
            if !merged.iter().any(|m| m.name == a.name) {
                merged.push(Arg {
                    required: false,
                    ann: String::new(),
                    ..a
                });
            }
        }
    }
    for m in &mut merged {
        let mut anns: Vec<String> = Vec::new();
        let mut everywhere = true;
        for v in &body.variants {
            match v.args.iter().find(|q| q.py_name == m.name) {
                Some(q) => {
                    everywhere &= q.required;
                    let ann = arg_of(p, q, ctx).ann;
                    if !anns.contains(&ann) {
                        anns.push(ann);
                    }
                }
                None => everywhere = false,
            }
        }
        m.required = everywhere;
        m.ann = anns.join(" | ");
    }
    if seen.iter().any(|r| !r.is_empty()) {
        imp(s, p, "_utils", "required_args");
        let lists = seen
            .iter()
            .map(|r| bracket("[", r.iter().map(|n| py_string(n).into()).collect(), false, "]").into())
            .collect();
        print_expr(p, &call("@required_args(", lists, false).into(), "", "");
    }
    merged
}

/// The `@overload` stubs of a param-switched streaming method (ogx `responses.create`, stagehand
/// `sessions.act`), and the implementation's arguments:
/// - the switch `Literal[False]` (`Optional[...]` when nullable) and optional, in its place
///   among the arguments, returning the response;
/// - the switch `Literal[True]` and required, after the required arguments, returning
///   `Stream[<event>]`;
/// - the same with `bool`, returning either;
/// - `@required_args` with the required arguments, then those and the switch; the
///   implementation's switch is `<Literal[False] spelling> | Literal[True]`.
fn print_stream_overloads(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    method: &SdkMethod,
    switch: &SdkParam,
    params: &[&SdkParam],
    sig: (bool, &[Arg], &str, &str),
    ctx: &dyn AnnotationContext,
) -> Vec<Arg> {
    let (awaits, path, model, streamed) = sig;
    let is_switch = |q: &SdkParam| std::ptr::eq(q, switch);
    let all: Vec<&SdkParam> = keyword_path_params(method)
        .iter()
        .chain(params.iter().copied())
        .collect();
    let off = SdkParam {
        ty: with_bool_literal(&switch.ty, false),
        required: false,
        ..switch.clone()
    };
    let off_ann = arg_of(p, &off, ctx).ann;
    let on_ann = to_annotation(&SdkType::Literal(vec![SdkLiteral::Boolean(true)]), p, ctx);
    let bool_ann = to_annotation(&SdkType::Primitive(SdkPrimitive::Boolean), p, ctx);
    let required: Vec<&SdkParam> = all.iter().copied().filter(|q| q.required && !is_switch(q)).collect();
    let optional: Vec<&SdkParam> = all.iter().copied().filter(|q| !q.required && !is_switch(q)).collect();
    let switched_first: Vec<&SdkParam> = required
        .iter()
        .copied()
        .chain([switch])
        .chain(optional.iter().copied())
        .collect();
    let args_of = |p: &mut PythonPrinter, order: &[&SdkParam], switch_arg: (&str, bool)| -> Vec<Arg> {
        order
            .iter()
            .map(|q| {
                if is_switch(q) {
                    Arg {
                        name: q.py_name.clone(),
                        ann: switch_arg.0.to_owned(),
                        required: switch_arg.1,
                    }
                } else {
                    arg_of(p, q, ctx)
                }
            })
            .collect()
    };
    let either = format!("{model} | {streamed}");
    let overloads: [(&[&SdkParam], (&str, bool), &str); 3] = [
        (&all, (&off_ann, false), model),
        (&switched_first, (&on_ann, true), streamed),
        (&switched_first, (&bool_ann, true), &either),
    ];
    for (order, switch_arg, returns) in overloads {
        let args = args_of(p, order, switch_arg);
        p.line("@overload");
        p.import_from("typing_extensions", "overload");
        print_signature(s, p, &method.name, awaits, path, &args, returns);
        let opened = p.open();
        // Path parameters are not listed, as in the method docstrings without overloads.
        let documented: Vec<&SdkParam> = order
            .iter()
            .copied()
            .filter(|q| !method.path_params.iter().any(|x| std::ptr::eq(x, *q)))
            .collect();
        for l in args_docstring(method, Some(&documented)) {
            p.line(&l);
        }
        p.line("...");
        p.close(opened);
        p.blank();
    }
    imp(s, p, "_utils", "required_args");
    let names = |extra: Option<&SdkParam>| -> Expr {
        let items = required
            .iter()
            .copied()
            .chain(extra)
            .map(|q| py_string(&q.py_name).into())
            .collect();
        bracket("[", items, false, "]").into()
    };
    print_expr(
        p,
        &call("@required_args(", vec![names(None), names(Some(switch))], false).into(),
        "",
        "",
    );
    args_of(p, &all, (&format!("{off_ann} | {on_ann}"), false))
}

/// `path_template("/a/{b_id}", b_id=b_id)`, `f"/a/{b_id}"` for a runtime without
/// `path_template`, or the plain path string.
fn path_expr(s: &FileState<'_>, p: &mut PythonPrinter, method: &SdkMethod) -> Expr {
    if method.path_params.is_empty() {
        return py_string(&method.path).into();
    }
    if !s.client.style.path_template {
        // Literal braces doubled, then each parameter interpolated by its Python name.
        let mut fstring = py_fstring(&method.path);
        for q in &method.path_params {
            fstring = fstring.replacen(&format!("{{{{{}}}}}", q.wire_name), &format!("{{{}}}", q.py_name), 1);
        }
        return fstring.into();
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

/// The statements before the call of a multipart method (ogx `files.create`): the body dict
/// copied with its file paths, the files taken out, and the multipart content type.
fn print_multipart_prelude(s: &FileState<'_>, p: &mut PythonPrinter, method: &SdkMethod, body: &[&SdkParam]) {
    imp(s, p, "_utils", "extract_files");
    p.import_from("typing", "Mapping");
    p.import_from("typing", "cast");
    let paths = || -> Expr {
        let lists = method
            .body_file_paths
            .iter()
            .map(|path| bracket("[", path.iter().map(|k| py_string(k).into()).collect(), false, "]").into())
            .collect();
        bracket("[", lists, false, "]").into()
    };
    let dict = bracket(
        "{",
        body.iter()
            .map(|q| format!("{}: {}", py_string(&q.py_name), q.py_name).into())
            .collect(),
        body.len() > 1,
        "}",
    );
    let copy = if s.client.style.deepcopy_with_paths {
        imp(s, p, "_files", "deepcopy_with_paths");
        call("deepcopy_with_paths(", vec![dict.into(), paths()], body.len() > 1)
    } else {
        // Runtimes before `deepcopy_with_paths` copy the whole body (honcho `messages.upload`).
        imp(s, p, "_utils", "deepcopy_minimal");
        call("deepcopy_minimal(", vec![dict.into()], false)
    };
    print_expr(p, &copy.into(), "body = ", "");
    let extract = call(
        "extract_files(",
        vec![
            "cast(Mapping[str, object], body)".into(),
            Expr::Atom(format!("paths={}", flat(&paths()))),
        ],
        false,
    );
    print_expr(p, &extract.into(), "files = ", "");
    p.line("# It should be noted that the actual Content-Type header that will be");
    p.line("# sent to the server will contain a `boundary` parameter, e.g.");
    p.line("# multipart/form-data; boundary=---abc--");
    p.line("extra_headers = {\"Content-Type\": \"multipart/form-data\", **(extra_headers or {})}");
}

/// `maybe_transform({...}, module.Params)`; awaited in async methods that await.
fn transform_expr(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    key: &str,
    params: &[&SdkParam],
    against: Expr,
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
        vec![dict.into(), against],
        false,
    )
}

/// How a header parameter is sent (perplexity, ogx, stagehand): a string as is; anything else
/// as text, and an array joined with commas, when it is given.
fn header_value(s: &FileState<'_>, p: &mut PythonPrinter, q: &SdkParam) -> String {
    let name = &q.py_name;
    let text = match q.ty {
        SdkType::Primitive(SdkPrimitive::String) => return name.clone(),
        SdkType::Array(_) => format!("\",\".join(str(e) for e in {name})"),
        _ => format!("str({name})"),
    };
    if q.required {
        return text;
    }
    let not_given = s.client.style.not_given();
    imp(s, p, "_utils", "is_given");
    imp(s, p, "_types", not_given);
    format!("{text} if is_given({name}) else {not_given}")
}

/// `extra_headers = {**strip_not_given({<header>: <value>, ...}), **(extra_headers or {})}` for
/// a method with header parameters, by wire name.
fn print_header_merge(s: &FileState<'_>, p: &mut PythonPrinter, method: &SdkMethod) {
    let headers = header_params(method);
    if headers.is_empty() {
        return;
    }
    imp(s, p, "_utils", "strip_not_given");
    let items: Vec<Expr> = headers
        .iter()
        .map(|q| format!("{}: {}", py_string(&q.wire_name), header_value(s, p, q)).into())
        .collect();
    let dict = bracket("{", items, false, "}");
    let merged = bracket(
        "{",
        vec![
            call("**strip_not_given(", vec![dict.into()], false).into(),
            "**(extra_headers or {})".into(),
        ],
        false,
        "}",
    );
    print_expr(p, &merged.into(), "extra_headers = ", "");
}

/// The content type of a raw-bytes body: the body's media type.
fn print_binary_prelude(p: &mut PythonPrinter, method: &SdkMethod) {
    let media = method.body_media_type.as_deref().unwrap_or("application/octet-stream");
    p.line(&format!(
        "extra_headers = {{\"Content-Type\": {}, **(extra_headers or {{}})}}",
        py_string(media)
    ));
}

/// `content=`: a raw-bytes body as it is, or the file a path names, read.
fn binary_content(s: &FileState<'_>, p: &mut PythonPrinter, awaits: bool) -> Expr {
    p.import_module("os");
    let read = if awaits {
        "async_read_file_content"
    } else {
        "read_file_content"
    };
    imp(s, p, "_files", read);
    format!(
        "content={}{read}(body) if isinstance(body, os.PathLike) else body",
        if awaits { "await " } else { "" }
    )
    .into()
}

/// `options=make_request_options(...)` and `cast_to=` (`model=` for a page): an unwrapped envelope
/// field adds a post-parser and casts through its wrapper, as ogx-client-python's
/// `providers.list` does (`DataWrapper[ProviderListResponse]`).
#[allow(clippy::too_many_arguments)]
fn push_options_and_cast(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    method: &SdkMethod,
    items: &mut Vec<Expr>,
    query: Option<Expr>,
    paged: bool,
    no_content: bool,
    model: &str,
) {
    imp(s, p, "_base_client", "make_request_options");
    let mut options: Vec<Expr> = OPTIONS_ARGS.iter().map(|&a| a.into()).collect();
    let multiline = query.is_some() || method.unwrap.is_some();
    options.extend(query);
    let wrapper = method.unwrap.as_deref().map(wrapper_class);
    if let Some(w) = &wrapper {
        options.push(format!("post_parser={w}[{model}]._unwrapper").into());
    }
    items.push(call("options=make_request_options(", options, multiline).into());
    if paged {
        items.push(format!("model={model}").into());
    } else if no_content {
        imp(s, p, "_types", "NoneType");
        items.push("cast_to=NoneType".into());
    } else if let Some(w) = &wrapper {
        p.import_from("typing", "Type");
        p.import_from("typing", "cast");
        imp(s, p, "_wrappers", w);
        items.push(format!("cast_to=cast(Type[{model}], {w}[{model}])").into());
    } else {
        items.push(format!("cast_to={model}").into());
    }
}

/// The wrapper class of an unwrapped envelope field: `data` -> `DataWrapper`.
#[must_use]
pub fn wrapper_class(field: &str) -> String {
    format!("{}Wrapper", pascal_case(&split_words(field)))
}

/// FER201 for a method shape [`emit_method`] does not generate yet.
fn check_supported(method: &SdkMethod) -> Result<(), FerruleError> {
    let switch = stream_switch(method);
    if is_param_switched(method) && switch.is_none() {
        return Err(unsupported(
            method,
            "a param-switched streaming method whose switch is not a body parameter",
        ));
    }
    if switch.is_some() && !method.body_variants.is_empty() {
        return Err(unsupported(
            method,
            "a param-switched streaming method with a union body",
        ));
    }
    if method.body.is_some() && method.body_params.is_empty() && method.body_variants.is_empty() {
        return Err(unsupported(method, "a request body that is not an object"));
    }
    // No oracle sends an object as a header: it would go out as `str(...)`, and a header-only
    // method has no params module to declare its class in.
    if let Some(h) = method.header_params.iter().find(|h| has_inline_object(&h.ty)) {
        return Err(unsupported(
            method,
            &format!("the header parameter \"{}\" with an object schema", h.wire_name),
        ));
    }
    Ok(())
}

/// What the body is checked against: the params class, or for a param-switched method the
/// class of its switch's value.
fn body_against(switch: Option<&SdkParam>, params_name: &str) -> Expr {
    match switch {
        Some(sw) => Cond {
            then: format!("{params_name}Streaming"),
            test: sw.py_name.clone(),
            otherwise: format!("{params_name}NonStreaming"),
        }
        .into(),
        None => params_name.into(),
    }
}

fn emit_method(
    s: &FileState<'_>,
    p: &mut PythonPrinter,
    method: &SdkMethod,
    mode: Mode,
    holder: Option<&SdkResource>,
) -> Result<(), FerruleError> {
    check_supported(method)?;
    let is_async = mode.is_async();
    let page = method.pagination.as_ref();
    let stream = method.streaming.as_ref();
    let switch = stream_switch(method);
    let ret = return_model(holder, method);
    let union = name_union_body(method);
    let params = union.as_ref().map_or_else(|| method_args(method), |u| u.args.clone());
    // Header parameters alone make no params file (ogx `models.retrieve`).
    let params_ref = if params.iter().all(|q| is_header(method, q) || is_binary_body(method, q)) {
        None
    } else {
        Some(method_ref(holder, method, "Params"))
    };
    let leaf_module = params_ref
        .as_ref()
        .map_or("", |r| r.module.rsplit('.').next().unwrap_or(""));
    let plain_named;
    let named = if let Some(u) = &union {
        &u.named
    } else {
        plain_named = name_params(&params, "");
        &plain_named
    };
    let ctx = ParamsContext {
        named,
        qualify: format!("{leaf_module}."),
        here: s.here.to_owned(),
        shadowed: HashSet::new(),
        outputs: s.outputs.clone(),
    };

    let model = ret
        .as_ref()
        .map_or_else(|| "None".to_owned(), |r| ctx.output_ref(p, &r.r#ref));
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
    // A param-switched method returns the response, or a stream of events.
    let streamed = switch.map(|_| {
        let event =
            stream_event_model(holder, method).map_or_else(|| "None".to_owned(), |r| ctx.output_ref(p, &r.r#ref));
        let streamed = format!("{}[{event}]", if is_async { "AsyncStream" } else { "Stream" });
        returns = format!("{model} | {streamed}");
        streamed
    });
    // A paginated async method is a plain def returning an AsyncPaginator.
    let awaits = is_async && page.is_none();

    // Only the last path parameter is positional; the others lead the keyword arguments.
    let path: Vec<Arg> = method
        .path_params
        .last()
        .map(|q| arg_of(p, q, &ctx))
        .into_iter()
        .collect();
    let args = match (&union, switch.zip(streamed.as_deref())) {
        (Some(u), _) => print_overloads(s, p, method, u, (awaits, &path, &returns), &ctx),
        (None, Some((sw, streamed))) => {
            print_stream_overloads(s, p, method, sw, &params, (awaits, &path, &model, streamed), &ctx)
        }
        (None, None) => keyword_path_params(method)
            .iter()
            .chain(params.iter().copied())
            .map(|q| keyword_arg(s, p, method, q, is_async, &ctx))
            .collect(),
    };
    print_signature(s, p, &method.name, awaits, &path, &args, &returns);

    let opened = p.open();
    let has_args = !method.path_params.is_empty() || !params.is_empty();
    // The implementation behind overloads has no docstring.
    let doc = if union.is_some() || switch.is_some() {
        Vec::new()
    } else {
        args_docstring(method, if has_args { Some(&params) } else { None })
    };
    for l in doc {
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
    print_header_merge(s, p, method);
    let params_name = params_ref.as_ref().map_or_else(String::new, |r| {
        format!("{}.{}", r.module.rsplit('.').next().unwrap_or(""), r.name)
    });
    // A method whose one argument is an array body never names its params module.
    let array_body_only = method.body_whole
        && method.query_params.is_empty()
        && named.preorder().is_empty()
        && method.body_params.iter().all(|q| matches!(q.ty, SdkType::Array(_)));
    if let Some(r) = params_ref.as_ref().filter(|_| !array_body_only) {
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
        .filter(|q| !query.iter().any(|x| std::ptr::eq(*x, *q)))
        .filter(|q| !method.path_params.iter().any(|x| std::ptr::eq(x, *q)))
        .filter(|q| !is_header(method, q))
        .collect();
    let in_body =
        page.is_some_and(|pg| pg.param_location == ParamLocation::Body) || method.http_method != HttpVerb::Get;
    let multipart = !method.body_file_paths.is_empty() && !body.is_empty() && in_body;
    if multipart {
        print_multipart_prelude(s, p, method, &body);
    }
    let binary = method.body_binary && !body.is_empty();
    if binary {
        print_binary_prelude(p, method);
    }
    let mut items: Vec<Expr> = vec![path_expr(s, p, method)];
    if let Some(page) = page {
        items.push(format!("page={sync_or_async}{}[{model}]", page.page).into());
    }
    if multipart {
        let f = if awaits {
            "async_maybe_transform"
        } else {
            "maybe_transform"
        };
        imp(s, p, "_utils", f);
        let open = format!("body={}{f}(", if awaits { "await " } else { "" });
        items.push(call(&open, vec!["body".into(), params_name.clone().into()], false).into());
        items.push("files=files".into());
    } else if binary {
        items.push(binary_content(s, p, awaits));
    } else if let ([whole], true) = (body.as_slice(), method.body_whole) {
        // Stainless checks a map against the method's params, an array against its own type.
        let against = if matches!(whole.ty, SdkType::Map(_)) {
            params_name.clone()
        } else {
            to_annotation(&whole.ty, p, &ctx)
        };
        let f = if awaits {
            "async_maybe_transform"
        } else {
            "maybe_transform"
        };
        imp(s, p, "_utils", f);
        let open = format!("body={}{f}(", if awaits { "await " } else { "" });
        items.push(call(&open, vec!["body".into(), against.into()], false).into());
    } else if !body.is_empty() && in_body {
        let against = body_against(switch, &params_name);
        items.push(transform_expr(s, p, "body", &body, against, awaits && page.is_none()).into());
    }
    let query_option = (!query.is_empty()).then(|| {
        Expr::from(transform_expr(
            s,
            p,
            "query",
            &query,
            params_name.clone().into(),
            awaits,
        ))
    });
    push_options_and_cast(
        s,
        p,
        method,
        &mut items,
        query_option,
        page.is_some(),
        ret.is_none() && stream.is_none(),
        &model,
    );
    if page.is_some() && method.http_method != HttpVerb::Get {
        items.push(format!("method=\"{}\"", method.http_method.as_str()).into());
    }
    if let (Some(sw), Some(streamed)) = (switch, &streamed) {
        items.push(format!("stream={} or False", sw.py_name).into());
        items.push(format!("stream_cls={streamed}").into());
    } else if stream.is_some() {
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
    let s = FileState {
        here: &here,
        client,
        outputs: shared_outputs(Some(resource), &resource.methods),
    };
    p.import_from("__future__", "annotations");
    // Only methods take request options (groq-python `chat.py` has subresources alone).
    if !resource.methods.is_empty() {
        p.import_module("httpx");
        for name in ["Body", "Query", "Headers", "NotGiven", client.style.not_given()] {
            imp(&s, &mut p, "_types", name);
        }
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
    let s = FileState {
        here,
        client,
        outputs: shared_outputs(None, &client.client_methods),
    };
    for method in stainless_method_order(&client.client_methods) {
        p.blank();
        emit_method(&s, p, method, mode, None)?;
    }
    Ok(())
}
