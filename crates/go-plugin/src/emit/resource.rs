//! One root `.go` file per resource: the service, its methods, the models stainless.yml declares
//! under it, and the types its methods introduce (responses first, then params).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Field, HttpMethod, Method, Object, Type};

use super::decls::print_decls;
use super::{Gen, Service, body_fields, method_order, params_arg};
use crate::naming::exported;
use crate::printer::{ALIGN, comment, go_file};
use crate::types::{Ctx, Decl, Mode, Pkg, Position, map_type, model_ref, object_decls};

/// What a method returns.
pub enum Returns {
    None,
    /// `*T`.
    Pointer(String),
}

/// The Go return type of `m` and the declarations it introduces.
pub fn returns(g: &Gen<'_>, s: &Service<'_>, m: &Method, ctx: &mut Ctx<'_>) -> (Returns, Vec<Decl>) {
    let Some(resp) = &m.response else {
        return (Returns::None, Vec::new());
    };
    let name = format!("{}{}Response", s.prefix, g.method_name(m));
    let mapped = map_type(&resp.ty, &Position::exact(&name), "", Mode::Response, ctx);
    (Returns::Pointer(mapped.ty), mapped.decls)
}

/// The params struct of `m`, if it takes params.
pub fn params_decls(g: &Gen<'_>, s: &Service<'_>, m: &Method, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let Some(arg) = params_arg(g.client, m) else {
        return Vec::new();
    };
    let name = format!("{}{}Params", s.prefix, g.method_name(m));
    let mode = if arg == "query" { Mode::Query } else { Mode::Body };
    let fields: Vec<Field> = if mode == Mode::Query {
        m.query_params.iter().map(|p| p.field.clone()).collect()
    } else {
        body_fields(g.client, m).into_iter().cloned().collect()
    };
    let obj = Object {
        fields,
        ..Object::default()
    };
    object_decls(&obj, &Position::exact(&name), "", mode, ctx)
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
    let arg = params_arg(g.client, m);
    if let Some(a) = arg {
        args.push(format!("{a} {}{name}Params", s.prefix));
    }
    args.push("opts ...option.RequestOption".to_owned());
    let (results, ok_return, err_return) = match ret {
        Returns::Pointer(t) => (format!("(res *{t}, err error)"), "return res, err", "return nil, err"),
        Returns::None => ("(err error)".to_owned(), "return err", "return err"),
    };
    comment(out, 0, method_doc(m));
    let _ = writeln!(out, "func (r *{}) {name}({}) {results} {{", s.service, args.join(", "));
    out.push_str("\topts = slices.Concat(r.Options, opts)\n");
    if matches!(ret, Returns::None) {
        out.push_str("\topts = append([]option.RequestOption{option.WithHeader(\"Accept\", \"\")}, opts...)\n");
    }
    // The message names the parameter as sent (knock-go: `user_id`).
    for ((n, t), p) in path_params.iter().zip(&m.path_params) {
        if *t == "string" {
            ctx.import("errors");
            let _ = write!(
                out,
                "\tif {n} == \"\" {{\n\t\terr = errors.New(\"missing required {} parameter\")\n\t\t{err_return}\n\t}}\n",
                p.field.wire_name
            );
        }
    }
    let template = m.path.trim_start_matches('/');
    if path_params.is_empty() {
        let _ = writeln!(out, "\tpath := {}", crate::printer::quote(template));
    } else {
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
        let _ = writeln!(
            out,
            "\tpath := fmt.Sprintf({}, {})",
            crate::printer::quote(&fmt_path),
            names.join(", ")
        );
    }
    let res = if matches!(ret, Returns::None) { "nil" } else { "&res" };
    let _ = writeln!(
        out,
        "\terr = requestconfig.ExecuteNewRequest(ctx, {}, path, {}, {res}, opts...)",
        http_method(m.http_method),
        arg.unwrap_or("nil")
    );
    let _ = write!(out, "\t{ok_return}\n}}\n\n");
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
    let _ = writeln!(out, "type {} struct {{", s.service);
    let _ = writeln!(out, "\tOptions{ALIGN}[]option.RequestOption");
    let subs: Vec<&Service<'_>> = s.resource.subresources.iter().filter_map(|r| g.service_of(r)).collect();
    for sub in &subs {
        let _ = writeln!(out, "\t{}{ALIGN}*{}", sub.field, sub.service);
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
    let _ = writeln!(
        out,
        "func New{}(opts ...option.RequestOption) (r *{}) {{\n\tr = &{}{{}}\n\tr.Options = opts",
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
        .flat_map(|d| model_decls(d, Mode::Response, ctx))
        .collect()
}

/// The declarations of one named model.
pub fn model_decls(d: &ferrule_sdk_model::model::ModelDecl, mode: Mode, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let name = d.r#ref.name.written.clone();
    let pos = Position::exact(&name);
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
