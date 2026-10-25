//! A resource's services: the blocking and async interfaces with their `WithRawResponse` views,
//! and their implementations. One pass writes the four files for both flavors.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Method, Type};

use crate::emit::params::{has_none, param_props};
use crate::emit::{Gen, Service, method_doc};
use crate::kt::{Imports, doc, file, string};

/// How a method's call returns, in one service flavor.
#[derive(Clone, Copy)]
struct Mode {
    async_: bool,
    raw: bool,
}

/// The response type of `m` (a response class or a model) and its import.
#[must_use]
pub fn response_type(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> (String, String) {
    if let Some(Type::Ref(r)) = m.response.as_ref().map(|r| &r.ty) {
        return (g.mapper.model_name(r), g.mapper.model_path(r));
    }
    let name = g.response_name(svc, m);
    let path = format!("{}.{name}", g.models_package(svc));
    (name, path)
}

fn suffix(async_: bool) -> &'static str {
    if async_ { "Async" } else { "" }
}

/// `AgencyService` / `AgencyServiceAsync`.
fn iface(svc: &Service<'_>, async_: bool) -> String {
    format!("{}Service{}", svc.name, suffix(async_))
}

fn modifier_ty(g: &Gen<'_>) -> &'static str {
    if g.java() {
        "Consumer<ClientOptions.Builder>"
    } else {
        "(ClientOptions.Builder) -> Unit"
    }
}

fn wrap(g: &Gen<'_>, mode: Mode, ty: &str) -> String {
    let ty = if mode.raw {
        format!("HttpResponseFor<{ty}>")
    } else {
        ty.to_owned()
    };
    if mode.async_ && g.java() {
        format!("CompletableFuture<{ty}>")
    } else {
        ty
    }
}

/// The overloads of one method in an interface.
// A Stainless file's members in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
fn overloads(g: &Gen<'_>, svc: &Service<'_>, m: &Method, mode: Mode) -> String {
    let java = g.java();
    let name = g.method_name(m);
    let params = g.params_name(svc, m);
    let (resp, _) = response_type(g, svc, m);
    let ret = wrap(g, mode, &resp);
    let fun = if mode.async_ && !java { "suspend fun" } else { "fun" };
    let ann = if mode.raw && !(mode.async_ && java) {
        "@MustBeClosed\n"
    } else {
        ""
    };
    let first_doc = if mode.raw {
        doc(&format!(
            "Returns a raw HTTP response for `{} {}`, but is otherwise the same as [{}.{name}].",
            m.http_method.as_str(),
            m.path,
            iface(svc, mode.async_)
        ))
    } else {
        doc(&method_doc(m))
    };
    let see = format!("/** @see {name} */\n");
    let (props, _) = param_props(g, m);
    let path = props.iter().rfind(|p| p.path).map(|p| (p.name.clone(), p.ty.kt(false)));
    let optional = has_none(g, m);
    let none = format!("{params}.none()");
    let ro = "requestOptions: RequestOptions";
    let ro_def = "requestOptions: RequestOptions = RequestOptions.none()";
    let mut items: Vec<String> = Vec::new();
    let sig = |args: &str| format!("{ann}{fun} {name}({args}): {ret}");
    match (&path, optional, java) {
        (Some((p, t)), true, false) => {
            items.push(format!(
                "{} =\n{name}(params.toBuilder().{p}({p}).build(), requestOptions)",
                sig(&format!("{p}: {t}, params: {params} = {none}, {ro_def},"))
            ));
            items.push(sig(&format!("params: {params}, {ro_def},")));
            items.push(format!(
                "{} =\n{name}({p}, {none}, requestOptions)",
                sig(&format!("{p}: {t}, {ro}"))
            ));
        }
        (Some((p, t)), false, false) => {
            items.push(format!(
                "{} =\n{name}(params.toBuilder().{p}({p}).build(), requestOptions)",
                sig(&format!("{p}: {t}, params: {params}, {ro_def},"))
            ));
            items.push(sig(&format!("params: {params}, {ro_def},")));
        }
        (None, true, false) => {
            items.push(sig(&format!("params: {params} = {none}, {ro_def},")));
            items.push(format!("{} =\n{name}({none}, requestOptions)", sig(ro)));
        }
        (None, false, false) => {
            items.push(sig(&format!("params: {params}, {ro_def},")));
        }
        (Some((p, t)), true, true) => {
            items.push(format!("{} =\n{name}({p}, {none})", sig(&format!("{p}: {t}"))));
            items.push(format!(
                "{} =\n{name}(params.toBuilder().{p}({p}).build(), requestOptions)",
                sig(&format!("{p}: {t}, params: {params} = {none}, {ro_def},"))
            ));
            items.push(format!(
                "{} =\n{name}({p}, params, RequestOptions.none())",
                sig(&format!("{p}: {t}, params: {params} = {none},"))
            ));
            items.push(sig(&format!("params: {params}, {ro_def},")));
            items.push(format!(
                "{} =\n{name}(params, RequestOptions.none())",
                sig(&format!("params: {params}"))
            ));
            items.push(format!(
                "{} =\n{name}({p}, {none}, requestOptions)",
                sig(&format!("{p}: {t}, {ro}"))
            ));
        }
        (Some((p, t)), false, true) => {
            items.push(format!(
                "{} =\n{name}({p}, params, RequestOptions.none())",
                sig(&format!("{p}: {t}, params: {params},"))
            ));
            items.push(format!(
                "{} =\n{name}(params.toBuilder().{p}({p}).build(), requestOptions)",
                sig(&format!("{p}: {t}, params: {params}, {ro_def},"))
            ));
            items.push(format!(
                "{} =\n{name}(params, RequestOptions.none())",
                sig(&format!("params: {params}"))
            ));
            items.push(sig(&format!("params: {params}, {ro_def},")));
        }
        (None, true, true) => {
            items.push(format!("{} =\n{name}({none})", sig("")));
            items.push(sig(&format!("params: {params} = {none}, {ro_def},")));
            items.push(format!(
                "{} =\n{name}(params, RequestOptions.none())",
                sig(&format!("params: {params} = {none}"))
            ));
            items.push(format!("{} =\n{name}({none}, requestOptions)", sig(ro)));
        }
        (None, false, true) => {
            items.push(format!(
                "{} =\n{name}(params, RequestOptions.none())",
                sig(&format!("params: {params}"))
            ));
            items.push(sig(&format!("params: {params}, {ro_def},")));
        }
    }
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        out.push_str(if i == 0 { &first_doc } else { &see });
        out.push_str(item);
        out.push_str("\n\n");
    }
    out
}

fn interface_imports(g: &Gen<'_>, svc: &Service<'_>, async_: bool, imp: &mut Imports) {
    imp.core("ClientOptions");
    imp.core("RequestOptions");
    imp.core("http.HttpResponseFor");
    if !(async_ && g.java()) {
        imp.add("com.google.errorprone.annotations.MustBeClosed");
    }
    if g.java() {
        imp.add("java.util.function.Consumer");
        if async_ {
            imp.add("java.util.concurrent.CompletableFuture");
        }
    }
    for m in svc.methods() {
        imp.add(format!("{}.{}", g.models_package(svc), g.params_name(svc, m)));
        imp.add(response_type(g, svc, m).1);
    }
    for &c in &svc.children {
        let child = &g.services[c];
        imp.add(format!("{}.{}", g.service_package(child, async_), iface(child, async_)));
    }
}

/// The service interface file.
#[must_use]
pub fn interface_file(g: &Gen<'_>, svc: &Service<'_>, async_: bool) -> (String, String) {
    let name = iface(svc, async_);
    let pkg = g.service_package(svc, async_);
    let mut imp = Imports::new(g.root());
    interface_imports(g, svc, async_, &mut imp);
    let modifier = modifier_ty(g);
    let mut out = String::new();
    let _ = writeln!(out, "interface {name} {{\n");
    out.push_str(&doc(
        "Returns a view of this service that provides access to raw HTTP responses for each method.",
    ));
    out.push_str("fun withRawResponse(): WithRawResponse\n\n");
    out.push_str(&doc(
        "Returns a view of this service with the given option modifications applied.\n\nThe original service is not modified.",
    ));
    let _ = writeln!(out, "fun withOptions(modifier: {modifier}): {name}\n");
    for &c in &svc.children {
        let child = &g.services[c];
        let _ = writeln!(out, "fun {}(): {}\n", child.accessor, iface(child, async_));
    }
    for m in svc.methods() {
        out.push_str(&overloads(g, svc, m, Mode { async_, raw: false }));
    }
    out.push_str(&doc(&format!(
        "A view of [{name}] that provides access to raw HTTP responses for each method."
    )));
    out.push_str("interface WithRawResponse {\n\n");
    out.push_str(&doc(
        "Returns a view of this service with the given option modifications applied.\n\nThe original service is not modified.",
    ));
    let _ = writeln!(out, "fun withOptions(modifier: {modifier}): {name}.WithRawResponse\n");
    for &c in &svc.children {
        let child = &g.services[c];
        let _ = writeln!(
            out,
            "fun {}(): {}.WithRawResponse\n",
            child.accessor,
            iface(child, async_)
        );
    }
    for m in svc.methods() {
        out.push_str(&overloads(g, svc, m, Mode { async_, raw: true }));
    }
    out.push_str("}\n}\n");
    let path = format!(
        "{}/services/{}/{}{name}.kt",
        g.core_main(),
        if async_ { "async" } else { "blocking" },
        crate::emit::dir_prefix(&svc.service_pkg)
    );
    (path, file(&pkg, &imp, &out))
}

/// The path segments of `m`'s path, as Kotlin arguments.
fn path_segments(g: &Gen<'_>, m: &Method) -> String {
    let (props, _) = param_props(g, m);
    let index = |wire: &str| props.iter().filter(|p| p.path).position(|p| p.field.wire_name == wire);
    let mut segs = Vec::new();
    for seg in m.path.trim_start_matches('/').split('/') {
        if seg.is_empty() {
            continue;
        }
        let mut text = String::new();
        let mut rest = seg;
        let mut only_param = None;
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else { break };
            let wire = &rest[open + 1..open + close];
            text.push_str(&rest[..open]);
            let i = index(wire).unwrap_or(0);
            if open == 0 && close + 1 == rest.len() && text.is_empty() {
                only_param = Some(i);
            }
            let _ = write!(text, "${{params._pathParam({i})}}");
            rest = &rest[open + close + 1..];
        }
        text.push_str(rest);
        match only_param {
            Some(i) => segs.push(format!("params._pathParam({i})")),
            None => segs.push(string(&text).replace("\\$", "$")),
        }
    }
    segs.join(", ")
}

/// The service implementation file.
// A Stainless file's members in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn impl_file(g: &Gen<'_>, svc: &Service<'_>, async_: bool) -> (String, String) {
    let java = g.java();
    let base = iface(svc, async_);
    let name = format!("{base}Impl");
    let pkg = g.service_package(svc, async_);
    let mut imp = Imports::new(g.root());
    imp.core("ClientOptions");
    imp.core("RequestOptions");
    for n in [
        "handlers.errorBodyHandler",
        "handlers.errorHandler",
        "handlers.jsonHandler",
        "http.HttpMethod",
        "http.HttpRequest",
        "http.HttpResponse",
        "http.HttpResponse.Handler",
        "http.HttpResponseFor",
        "http.parseable",
    ] {
        imp.core(n);
    }
    imp.core(if async_ { "prepareAsync" } else { "prepare" });
    if java {
        imp.add("java.util.function.Consumer");
        if async_ {
            imp.add("java.util.concurrent.CompletableFuture");
        }
    }
    for m in svc.methods() {
        imp.add(format!("{}.{}", g.models_package(svc), g.params_name(svc, m)));
        imp.add(response_type(g, svc, m).1);
        let (props, _) = param_props(g, m);
        if props.iter().any(|p| p.path) {
            imp.core("checkRequired");
            if java {
                imp.add("kotlin.jvm.optionals.getOrNull");
            }
        }
    }
    for &c in &svc.children {
        let child = &g.services[c];
        let cp = g.service_package(child, async_);
        imp.add(format!("{cp}.{}", iface(child, async_)));
        imp.add(format!("{cp}.{}Impl", iface(child, async_)));
    }
    let modifier = modifier_ty(g);
    let apply = if java { "modifier::accept" } else { "modifier" };
    let fun = if async_ && !java {
        "override suspend fun"
    } else {
        "override fun"
    };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "class {name} internal constructor(private val clientOptions: ClientOptions) :\n{base} {{\n"
    );
    let _ = writeln!(
        out,
        "private val withRawResponse: {base}.WithRawResponse by lazy {{\nWithRawResponseImpl(clientOptions)\n}}\n"
    );
    for &c in &svc.children {
        let child = &g.services[c];
        let ci = iface(child, async_);
        let _ = writeln!(
            out,
            "private val {}: {ci} by lazy {{ {ci}Impl(clientOptions) }}\n",
            child.accessor
        );
    }
    let _ = writeln!(
        out,
        "override fun withRawResponse(): {base}.WithRawResponse = withRawResponse\n"
    );
    let _ = writeln!(
        out,
        "override fun withOptions(modifier: {modifier}): {base} =\n{name}(clientOptions.toBuilder().apply({apply}).build())\n"
    );
    for &c in &svc.children {
        let child = &g.services[c];
        let _ = writeln!(
            out,
            "override fun {a}(): {} = {a}\n",
            iface(child, async_),
            a = child.accessor
        );
    }
    for m in svc.methods() {
        let mn = g.method_name(m);
        let params = g.params_name(svc, m);
        let (resp, _) = response_type(g, svc, m);
        let ret = wrap(g, Mode { async_, raw: false }, &resp);
        let parse = if async_ && java {
            ".thenApply { it.parse() }"
        } else {
            ".parse()"
        };
        let _ = writeln!(
            out,
            "{fun} {mn}(params: {params}, requestOptions: RequestOptions,): {ret} =\n// {} {}\nwithRawResponse().{mn}(params, requestOptions){parse}\n",
            m.http_method.as_str(),
            m.path
        );
    }

    let _ = writeln!(
        out,
        "class WithRawResponseImpl internal constructor(private val clientOptions: ClientOptions) :\n{base}.WithRawResponse {{\n"
    );
    out.push_str("private val errorHandler: Handler<HttpResponse> =\nerrorHandler(errorBodyHandler(clientOptions.jsonMapper))\n\n");
    for &c in &svc.children {
        let child = &g.services[c];
        let ci = iface(child, async_);
        let _ = writeln!(
            out,
            "private val {}: {ci}.WithRawResponse by lazy {{\n{ci}Impl.WithRawResponseImpl(clientOptions)\n}}\n",
            child.accessor
        );
    }
    let _ = writeln!(
        out,
        "override fun withOptions(\nmodifier: {modifier}\n): {base}.WithRawResponse =\n{name}.WithRawResponseImpl(clientOptions.toBuilder().apply({apply}).build())\n"
    );
    for &c in &svc.children {
        let child = &g.services[c];
        let _ = writeln!(
            out,
            "override fun {a}(): {}.WithRawResponse = {a}\n",
            iface(child, async_),
            a = child.accessor
        );
    }
    for m in svc.methods() {
        let mn = g.method_name(m);
        let params = g.params_name(svc, m);
        let (resp, _) = response_type(g, svc, m);
        let ret = wrap(g, Mode { async_, raw: true }, &resp);
        let _ = writeln!(
            out,
            "private val {mn}Handler: Handler<{resp}> =\njsonHandler<{resp}>(clientOptions.jsonMapper)\n"
        );
        let _ = writeln!(
            out,
            "{fun} {mn}(params: {params}, requestOptions: RequestOptions,): {ret} {{"
        );
        let (props, _) = param_props(g, m);
        if let Some(p) = props.iter().rfind(|p| p.path) {
            let get = if java { ".getOrNull()" } else { "" };
            out.push_str("// We check here instead of in the params builder because this can be specified\n// positionally or in the params class.\n");
            let _ = writeln!(out, "checkRequired({}, params.{}(){get})", string(&p.name), p.name);
        }
        let method = m.http_method.as_str().to_ascii_uppercase();
        let prepare = if async_ { "prepareAsync" } else { "prepare" };
        let _ = writeln!(
            out,
            "val request = HttpRequest.builder().method(HttpMethod.{method}).baseUrl(clientOptions.baseUrl()).addPathSegments({}).build().{prepare}(clientOptions, params)",
            path_segments(g, m)
        );
        out.push_str("val requestOptions = requestOptions.applyDefaults(RequestOptions.from(clientOptions))\n");
        let handle = format!(
            "errorHandler.handle(response).parseable {{\nresponse.use {{ {mn}Handler.handle(it) }}.also {{\nif (requestOptions.responseValidation!!) {{\nit.validate()\n}}\n}}\n}}"
        );
        if async_ && java {
            let _ = writeln!(
                out,
                "return request.thenComposeAsync {{ clientOptions.httpClient.executeAsync(it, requestOptions) }}.thenApply {{ response ->\n{handle}\n}}\n}}\n"
            );
        } else {
            let exec = if async_ { "executeAsync" } else { "execute" };
            let _ = writeln!(
                out,
                "val response = clientOptions.httpClient.{exec}(request, requestOptions)\nreturn {handle}\n}}\n"
            );
        }
    }
    out.push_str("}\n}\n");
    let path = format!(
        "{}/services/{}/{}{name}.kt",
        g.core_main(),
        if async_ { "async" } else { "blocking" },
        crate::emit::dir_prefix(&svc.service_pkg)
    );
    (path, file(&pkg, &imp, &out))
}
