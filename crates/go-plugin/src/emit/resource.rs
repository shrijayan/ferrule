//! One root `.go` file per resource: the service, its methods, the models stainless.yml declares
//! under it, and the types its methods introduce (responses first, then params).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Field, HttpMethod, Method, Type};

use super::decls::print_decls;
use super::pagination::page_of;
use super::{Gen, Service, body_fields, method_order};
use crate::naming::exported;
use crate::printer::{ALIGN, comment, go_file};
use crate::types::{Ctx, Decl, Mode, Pkg, Position, map_type, model_ref, object_decls, params_decls as params_struct};

/// What a method returns.
pub enum Returns {
    None,
    /// `*T`.
    Pointer(String),
    /// `*pagination.<page>[<item>]`, with a `<Method>AutoPaging` twin.
    Page {
        page: String,
        item: String,
    },
}

/// The Go return type of `m` and the declarations it introduces. A page's item type takes the
/// response's name (`UserFeedListItemsResponse` in knock-go).
pub fn returns(g: &Gen<'_>, s: &Service<'_>, m: &Method, ctx: &mut Ctx<'_>) -> (Returns, Vec<Decl>) {
    let Some(resp) = &m.response else {
        return (Returns::None, Vec::new());
    };
    let name = format!("{}{}Response", s.prefix, g.method_name(m));
    // An inline response object is documented by its schema's description (knock-mgmt-go's
    // `APIKeyExchangeResponse`).
    let doc = |ty: &Type| match ty {
        Type::Object(o) => o.description.clone().unwrap_or_default(),
        _ => String::new(),
    };
    if let (Some(page), Some(p)) = (page_of(g, m), &m.pagination) {
        let mapped = map_type(&p.item, &Position::exact(&name), &doc(&p.item), Mode::Response, ctx);
        let returns = Returns::Page {
            page: page.name,
            item: mapped.ty,
        };
        return (returns, mapped.decls);
    }
    // v2 declares a response that is any object as an empty struct (imagekit-go's
    // `FolderNewResponse`).
    if g.target.v2() && is_any_object(&resp.ty) {
        let empty = ferrule_sdk_model::model::Object::default();
        let decls = object_decls(&empty, &Position::exact(&name), &doc(&resp.ty), Mode::Response, ctx);
        return (Returns::Pointer(name), decls);
    }
    let mapped = map_type(&resp.ty, &Position::exact(&name), &doc(&resp.ty), Mode::Response, ctx);
    (Returns::Pointer(mapped.ty), mapped.decls)
}

/// Whether `ty` is an object with no fields, whose other keys may be anything.
fn is_any_object(ty: &Type) -> bool {
    match ty {
        Type::Object(o) => o.fields.is_empty() && o.bases.is_empty(),
        Type::Map { values } => matches!(
            **values,
            Type::Scalar {
                scalar: ferrule_sdk_model::model::Scalar::Unknown
            }
        ),
        _ => false,
    }
}

/// The params struct of `m`, if it takes params.
pub fn params_decls(g: &Gen<'_>, s: &Service<'_>, m: &Method, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    if g.params_arg(m).is_none() {
        return Vec::new();
    }
    let name = format!("{}{}Params", s.prefix, g.method_name(m));
    let path: Vec<Field> = g.path_fields(m).iter().map(|p| p.field.clone()).collect();
    let query: Vec<Field> = m.query_params.iter().map(|p| p.field.clone()).collect();
    let body: Vec<Field> = body_fields(g.client, m).into_iter().cloned().collect();
    params_struct(&path, &query, &body, &name, ctx)
}

/// Whether the params struct of `m` has fields sent as query or body (not only v2's path
/// fields), so that the request takes it.
fn sends_params(g: &Gen<'_>, m: &Method) -> bool {
    !m.query_params.is_empty() || !body_fields(g.client, m).is_empty() || !m.header_params.is_empty()
}

fn http_method(m: HttpMethod) -> &'static str {
    match m {
        HttpMethod::Get => "http.MethodGet",
        HttpMethod::Put => "http.MethodPut",
        HttpMethod::Post => "http.MethodPost",
        HttpMethod::Delete => "http.MethodDelete",
        HttpMethod::Options => "http.MethodOptions",
        HttpMethod::Head => "http.MethodHead",
        HttpMethod::Patch => "http.MethodPatch",
        HttpMethod::Trace => "http.MethodTrace",
        HttpMethod::Connect => "http.MethodConnect",
        HttpMethod::Query => "\"QUERY\"",
    }
}

/// The method's doc comment text: its description, else its summary.
#[must_use]
pub fn method_doc(m: &Method) -> &str {
    if m.description.trim().is_empty() {
        &m.summary
    } else {
        &m.description
    }
}

/// `path := ...`: the path as a literal, or `fmt.Sprintf` over the path params.
fn path_line(m: &Method, path_params: &[(String, &'static str)], ctx: &mut Ctx<'_>) -> String {
    let template = m.path.trim_start_matches('/');
    if path_params.is_empty() {
        return format!("\tpath := {}\n", crate::printer::quote(template));
    }
    ctx.import("fmt");
    let mut fmt_path = String::new();
    let mut tail = template;
    let mut names = Vec::new();
    while let Some(start) = tail.find('{') {
        let Some(end) = tail[start..].find('}') else { break };
        fmt_path.push_str(&tail[..start]);
        let wire = &tail[start + 1..start + end];
        let idx = m.path_params.iter().position(|p| p.field.wire_name == wire);
        let verb = idx.map_or("%s", |i| if path_params[i].1 == "string" { "%s" } else { "%v" });
        fmt_path.push_str(verb);
        names.push(idx.map_or_else(|| wire.to_owned(), |i| path_params[i].0.clone()));
        tail = &tail[start + end + 1..];
    }
    fmt_path.push_str(tail);
    format!(
        "\tpath := fmt.Sprintf({}, {})\n",
        crate::printer::quote(&fmt_path),
        names.join(", ")
    )
}

fn method(out: &mut String, g: &Gen<'_>, s: &Service<'_>, m: &Method, ret: &Returns, ctx: &mut Ctx<'_>) {
    ctx.import("context");
    ctx.import("net/http");
    ctx.import("slices");
    ctx.import_module("internal/requestconfig");
    ctx.import_module("option");
    let name = g.method_name(m);
    let path_params = g.path_params(m);
    let mut args = vec!["ctx context.Context".to_owned()];
    args.extend(path_params.iter().map(|(n, t)| format!("{n} {t}")));
    let arg = g.params_arg(m);
    if let Some(a) = arg {
        args.push(format!("{a} {}{name}Params", s.prefix));
    }
    args.push("opts ...option.RequestOption".to_owned());
    let (results, ok_return, err_return) = match ret {
        Returns::Pointer(t) => (format!("(res *{t}, err error)"), "return res, err", "return nil, err"),
        Returns::Page { page, item } => {
            ctx.import_module("packages/pagination");
            (
                format!("(res *pagination.{page}[{item}], err error)"),
                "return res, nil",
                "return nil, err",
            )
        }
        Returns::None => ("(err error)".to_owned(), "return err", "return err"),
    };
    comment(out, 0, method_doc(m));
    let _ = writeln!(out, "func (r *{}) {name}({}) {results} {{", s.service, args.join(", "));
    if matches!(ret, Returns::Page { .. }) {
        out.push_str("\tvar raw *http.Response\n\topts = slices.Concat(r.Options, opts)\n");
        out.push_str("\topts = append([]option.RequestOption{option.WithResponseInto(&raw)}, opts...)\n");
    } else {
        out.push_str("\topts = slices.Concat(r.Options, opts)\n");
    }
    if matches!(ret, Returns::None) {
        // v2: imagekit-go's `Delete`; v1's empty value is unverified (no oracle method).
        let accept = if g.target.v2() { "*/*" } else { "" };
        let _ = writeln!(
            out,
            "\topts = append([]option.RequestOption{{option.WithHeader(\"Accept\", \"{accept}\")}}, opts...)"
        );
    }
    // Every path param by its Go expression: the params struct's field, else the argument.
    let fields = g.path_fields(m);
    let values: Vec<(String, &'static str)> = m
        .path_params
        .iter()
        .enumerate()
        .map(|(i, p)| match (i < fields.len(), arg) {
            (true, Some(a)) => (
                format!(
                    "{a}.{}",
                    crate::naming::exported(&crate::naming::words(&p.field.name), g.casings)
                ),
                super::path_param_type(&p.field.ty),
            ),
            _ => path_params[i - fields.len()].clone(),
        })
        .collect();
    // The message names the parameter as sent (knock-go: `user_id`).
    for ((n, t), p) in values.iter().zip(&m.path_params) {
        if *t == "string" {
            ctx.import("errors");
            let _ = write!(
                out,
                "\tif {n} == \"\" {{\n\t\terr = errors.New(\"missing required {} parameter\")\n\t\t{err_return}\n\t}}\n",
                p.field.wire_name
            );
        }
    }
    out.push_str(&path_line(m, &values, ctx));
    let res = if matches!(ret, Returns::None) { "nil" } else { "&res" };
    let verb = http_method(m.http_method);
    // A params struct holding only path fields is not sent (imagekit-go's `Restore`).
    let params = arg.filter(|_| sends_params(g, m)).unwrap_or("nil");
    let Returns::Page { page, item } = ret else {
        let _ = writeln!(
            out,
            "\terr = requestconfig.ExecuteNewRequest(ctx, {verb}, path, {params}, {res}, opts...)"
        );
        let _ = write!(out, "\t{ok_return}\n}}\n\n");
        return;
    };
    let _ = write!(
        out,
        "\tcfg, err := requestconfig.NewRequestConfig(ctx, {verb}, path, {params}, &res, opts...)\n\
         \tif err != nil {{\n\t\t{err_return}\n\t}}\n\terr = cfg.Execute()\n\tif err != nil {{\n\t\t{err_return}\n\t}}\n\
         \tres.SetPageConfig(cfg, raw)\n\t{ok_return}\n}}\n\n"
    );
    // The auto-pager: the same arguments, passed on.
    let mut call = vec!["ctx".to_owned()];
    call.extend(path_params.iter().map(|(n, _)| n.clone()));
    call.extend(arg.map(str::to_owned));
    call.push("opts...".to_owned());
    comment(out, 0, method_doc(m));
    let _ = write!(
        out,
        "func (r *{}) {name}AutoPaging({}) *pagination.{page}AutoPager[{item}] {{\n\
         \treturn pagination.New{page}AutoPager(r.{name}({}))\n}}\n\n",
        s.service,
        args.join(", "),
        call.join(", ")
    );
}

/// The service struct and its constructor.
fn service_decl(out: &mut String, g: &Gen<'_>, s: &Service<'_>, ctx: &mut Ctx<'_>) {
    ctx.import_module("option");
    comment(
        out,
        0,
        &format!(
            "{} contains methods and other services that help with interacting with the {} API.\n\n\
             Note, unlike clients, this service does not read variables from the environment automatically. \
             You should not instantiate this service directly, and instead use the [New{}] method instead.",
            s.service, g.org, s.service
        ),
    );
    // v2 holds services by value.
    let ptr = if g.target.v2() { "" } else { "*" };
    let _ = writeln!(out, "type {} struct {{", s.service);
    let _ = writeln!(out, "\tOptions{ALIGN}[]option.RequestOption");
    let subs: Vec<&Service<'_>> = s.resource.subresources.iter().filter_map(|r| g.service_of(r)).collect();
    for sub in &subs {
        let _ = writeln!(out, "\t{}{ALIGN}{ptr}{}", sub.field, sub.service);
    }
    out.push_str("}\n\n");
    comment(
        out,
        0,
        &format!(
            "New{} generates a new service that applies the given options to each request. These options are \
             applied after the parent client's options (if there is one), and before any request-specific options.",
            s.service
        ),
    );
    let amp = if g.target.v2() { "" } else { "&" };
    let _ = writeln!(
        out,
        "func New{}(opts ...option.RequestOption) (r {ptr}{}) {{\n\tr = {amp}{}{{}}\n\tr.Options = opts",
        s.service, s.service, s.service
    );
    for sub in &subs {
        let _ = writeln!(out, "\tr.{} = New{}(opts...)", sub.field, sub.service);
    }
    out.push_str("\treturn\n}\n\n");
}

/// The models stainless.yml declares under this resource (not `$shared`).
fn resource_models(g: &Gen<'_>, s: &Service<'_>, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    g.models_of(s)
        .into_iter()
        .flat_map(|d| all_model_decls(g, d, ctx))
        .collect()
}

/// A model's declarations as the SDK has them: its response types unless only params use it,
/// then in v2 its param variant when params use it (knock-mgmt-go's `Duration`, then
/// `DurationParam`), the enums shared between them declared once.
pub fn all_model_decls(g: &Gen<'_>, d: &ferrule_sdk_model::model::ModelDecl, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let schema = &d.r#ref.schema;
    let in_params = g.param_models.contains(schema);
    let mut out = Vec::new();
    if !g.target.v2() || g.response_models.contains(schema) || !in_params {
        out.extend(model_decls(d, Mode::Response, ctx));
    }
    if g.target.v2() && in_params && crate::types::has_param_variant(g.client, &d.r#ref) {
        for decl in model_decls(d, Mode::Body, ctx) {
            if !out.iter().any(|o: &Decl| o.name() == decl.name()) {
                out.push(decl);
            }
        }
    }
    out
}

/// The declarations of one named model in `mode`: in v2 params, its param variant.
pub fn model_decls(d: &ferrule_sdk_model::model::ModelDecl, mode: Mode, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let name = crate::types::model_name(&d.r#ref, ctx.casings);
    let pos = if ctx.v2 && mode != Mode::Response {
        Position::param_variant(&name)
    } else {
        Position::exact(&name)
    };
    match &d.ty {
        Type::Object(o) if !(o.fields.is_empty() && o.bases.is_empty()) => {
            let doc = if d.description.is_empty() {
                o.description.as_deref().unwrap_or_default()
            } else {
                &d.description
            };
            object_decls(o, &pos, doc, mode, ctx)
        }
        other => {
            let mapped = map_type(other, &pos, &d.description, mode, ctx);
            if mapped.decls.is_empty() {
                // An alias of another type, as Stainless declares one.
                let _ = model_ref(&d.r#ref, ctx);
                Vec::new()
            } else {
                mapped.decls
            }
        }
    }
}

/// The `.go` file of one service.
#[must_use]
pub fn resource_file(g: &Gen<'_>, s: &Service<'_>) -> (String, String) {
    let mut ctx = g.ctx(Pkg::Root);
    let mut body = String::new();
    service_decl(&mut body, g, s, &mut ctx);
    let methods = method_order(&s.resource.methods);
    let mut response_decls = Vec::new();
    for m in &methods {
        let (ret, decls) = returns(g, s, m, &mut ctx);
        method(&mut body, g, s, m, &ret, &mut ctx);
        response_decls.extend(decls);
    }
    let models = resource_models(g, s, &mut ctx);
    print_decls(&mut body, &models, &mut ctx);
    print_decls(&mut body, &response_decls, &mut ctx);
    for m in &methods {
        let decls = params_decls(g, s, m, &mut ctx);
        print_decls(&mut body, &decls, &mut ctx);
    }
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    (
        format!("{}.go", s.file),
        go_file(&g.target.package_name, &imports, &body),
    )
}

/// A Go field name for words, for callers outside the type mapper.
#[must_use]
pub fn field_name(g: &Gen<'_>, words: &[String]) -> String {
    exported(words, g.casings)
}
