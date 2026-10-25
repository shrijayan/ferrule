//! The tests Stainless generates: one `_test.go` per service (each method against the mock
//! server), `usage_test.go` (the README example) and `client_test.go` (retries, timeouts and the
//! User-Agent, against a fake transport).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ClientOpt, Field, Method, OptType, Scalar, Type};

use super::resource::params_decls;
use super::{Gen, Service, method_order, params_arg};
use crate::naming::{exported, local, singular, split_words, words};
use crate::printer::{ALIGN, go_file};
use crate::types::{Ctx, Decl, Pkg};

/// `client_test.go` after its imports; text from Stainless's Go v1 tests (OneBusAway go-sdk,
/// Apache-2.0; see NOTICE) with the SDK's names as `{{...}}` tokens.
const CLIENT_TEST: &str = include_str!("client_test.go.tmpl");

/// `"My API Key"` for the `api_key` setting, as Stainless fills client settings in tests.
fn opt_example(o: &ClientOpt, g: &Gen<'_>) -> String {
    let title: Vec<String> = words(&o.name)
        .iter()
        .map(|w| exported(std::slice::from_ref(w), g.casings))
        .collect();
    format!("\"My {}\"", title.join(" "))
}

/// The `option.With...` lines the tests construct a client with, at `indent` tabs.
fn client_options(g: &Gen<'_>, indent: usize) -> String {
    let tabs = "\t".repeat(indent);
    let mut out = String::new();
    for o in g.client.settings.iter().flat_map(|s| &s.opts) {
        if o.auth.is_some() && o.ty == OptType::String {
            let _ = writeln!(
                out,
                "{tabs}option.With{}({}),",
                exported(&words(&o.name), g.casings),
                opt_example(o, g)
            );
        }
    }
    out
}

/// A test value for a field of type `ty`.
fn example(g: &Gen<'_>, f: &Field, decls: &[Decl], go_ty: &str, ctx: &mut Ctx<'_>) -> String {
    let pkg = &g.target.package_name;
    match &f.ty {
        Type::Nullable { inner } => example(
            g,
            &Field {
                ty: (**inner).clone(),
                ..f.clone()
            },
            decls,
            go_ty,
            ctx,
        ),
        Type::Scalar { scalar } => match scalar {
            Scalar::Boolean => "true".to_owned(),
            s if s.is_integer() => "int64(0)".to_owned(),
            Scalar::Float | Scalar::Double => "0.000000".to_owned(),
            Scalar::Date | Scalar::DateTime => {
                ctx.import("time");
                "time.Now()".to_owned()
            }
            Scalar::Uuid => "\"182bd5e5-6e1a-4fe4-a799-aa6d9a6ab26e\"".to_owned(),
            _ => crate::printer::quote(&f.wire_name),
        },
        Type::Enum { .. } => decls
            .iter()
            .find_map(|d| match d {
                Decl::Enum { name, members, .. } if name == go_ty => members.first().map(|(c, _)| format!("{pkg}.{c}")),
                _ => None,
            })
            .unwrap_or_else(|| crate::printer::quote(&f.wire_name)),
        Type::Array { items } => {
            let item_ty = go_ty.trim_start_matches("[]");
            let inner = example(
                g,
                &Field {
                    ty: (**items).clone(),
                    ..f.clone()
                },
                decls,
                item_ty,
                ctx,
            );
            let qualified = if item_ty.starts_with(|c: char| c.is_ascii_uppercase()) {
                format!("{pkg}.{item_ty}")
            } else {
                item_ty.to_owned()
            };
            format!("[]{qualified}{{{inner}}}")
        }
        _ => crate::printer::quote(&f.wire_name),
    }
}

/// The params literal lines at `indent` tabs (fields one deeper), without the closing brace.
fn params_literal(
    g: &Gen<'_>,
    s: &Service<'_>,
    m: &Method,
    indent: usize,
    ctx: &mut Ctx<'_>,
) -> Option<(String, bool)> {
    params_arg(g.client, m)?;
    let mut scratch = g.ctx(Pkg::Root);
    let decls = params_decls(g, s, m, &mut scratch);
    let Some(Decl::Struct { name, fields, .. }) = decls.first() else {
        return None;
    };
    let pkg = &g.target.package_name;
    let tabs = "\t".repeat(indent + 1);
    let mut out = format!("{pkg}.{name}{{\n");
    let sources: Vec<&Field> = m
        .query_params
        .iter()
        .map(|p| &p.field)
        .chain(super::body_fields(g.client, m))
        .collect();
    let mut optional = false;
    for f in fields {
        let Some(src) = sources.iter().find(|x| x.wire_name == f.wire) else {
            continue;
        };
        optional |= !f.required;
        let value = example(g, src, &decls, &f.ty, ctx);
        let _ = writeln!(out, "{tabs}{}:{ALIGN}{pkg}.F({value}),", f.name);
    }
    Some((out, optional))
}

/// The call of `m` in a test: `client.Agency.Get(context.TODO(), "agencyID")`, with the params
/// literal on its own lines when there are path params too. Returns the text after `:= ` or
/// `= ` and whether the params have optional fields.
fn call(g: &Gen<'_>, s: &Service<'_>, m: &Method, ctx_arg: &str, ctx: &mut Ctx<'_>) -> (String, bool) {
    let target = format!("{}.{}", s.accessor, g.method_name(m));
    let path_args: Vec<String> = m
        .path_params
        .iter()
        .map(|p| crate::printer::quote(&p.field.wire_name))
        .collect();
    match params_literal(g, s, m, if path_args.is_empty() { 1 } else { 2 }, ctx) {
        None => {
            let mut args = vec![ctx_arg.to_owned()];
            args.extend(path_args);
            (format!("{target}({})", args.join(", ")), false)
        }
        Some((literal, optional)) if path_args.is_empty() => (format!("{target}({ctx_arg}, {literal}\t}})"), optional),
        Some((literal, optional)) => {
            let mut out = format!("{target}(\n\t\t{ctx_arg},\n");
            for a in &path_args {
                let _ = writeln!(out, "\t\t{a},");
            }
            let _ = write!(out, "\t\t{literal}\t\t}},\n\t)");
            (out, optional)
        }
    }
}

const PREAMBLE: &str = "\tbaseURL := \"http://localhost:4010\"\n\tif envURL, ok := os.LookupEnv(\"TEST_API_BASE_URL\"); ok {\n\t\tbaseURL = envURL\n\t}\n\tif !testutil.CheckTestServer(t, baseURL) {\n\t\treturn\n\t}\n";

fn test_imports(g: &Gen<'_>, ctx: &mut Ctx<'_>) {
    for p in ["context", "os", "testing"] {
        ctx.import(p);
    }
    ctx.import(&g.target.module_path);
    ctx.import_module("internal/testutil");
    ctx.import_module("option");
}

fn new_client(g: &Gen<'_>) -> String {
    format!(
        "\tclient := {}.NewClient(\n\t\toption.WithBaseURL(baseURL),\n{}\t)\n",
        g.target.package_name,
        client_options(g, 2)
    )
}

/// The `_test.go` file of a service, if it has methods.
#[must_use]
pub fn service_test_file(g: &Gen<'_>, s: &Service<'_>) -> Option<(String, String)> {
    if s.resource.methods.is_empty() {
        return None;
    }
    let mut ctx = g.ctx(Pkg::Root);
    test_imports(g, &mut ctx);
    ctx.import("errors");
    let pkg = &g.target.package_name;
    let mut body = String::new();
    for m in method_order(&s.resource.methods) {
        let (call, optional) = call(g, s, m, "context.TODO()", &mut ctx);
        let suffix = if optional { "WithOptionalParams" } else { "" };
        let _ = write!(
            body,
            "func Test{}{}{suffix}(t *testing.T) {{\n{PREAMBLE}{}\t_, err := {call}\n\tif err != nil {{\n\t\tvar apierr *{pkg}.Error\n\t\tif errors.As(err, &apierr) {{\n\t\t\tt.Log(string(apierr.DumpRequest(true)))\n\t\t}}\n\t\tt.Fatalf(\"err should be nil: %s\", err.Error())\n\t}}\n}}\n\n",
            s.prefix,
            g.method_name(m),
            new_client(g),
        );
    }
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some((
        format!("{}_test.go", s.file),
        go_file(&format!("{pkg}_test"), &imports, &body),
    ))
}

/// `usage_test.go`: the README example against the mock server.
#[must_use]
pub fn usage_test_file(g: &Gen<'_>) -> Option<(String, String)> {
    let (s, m) = g.example()?;
    let mut ctx = g.ctx(Pkg::Root);
    test_imports(g, &mut ctx);
    let var = local(&singular(&split_words(&s.resource.name.written)), g.casings);
    let (call, _) = call(g, s, m, "context.TODO()", &mut ctx);
    let body = format!(
        "func TestUsage(t *testing.T) {{\n{PREAMBLE}{}\t{var}, err := {call}\n\tif err != nil {{\n\t\tt.Error(err)\n\t\treturn\n\t}}\n\tt.Logf(\"%+v\\n\", {var})\n}}\n",
        new_client(g)
    );
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some((
        "usage_test.go".to_owned(),
        go_file(&format!("{}_test", g.target.package_name), &imports, &body),
    ))
}

/// `client_test.go`, calling the README example method.
#[must_use]
pub fn client_test_file(g: &Gen<'_>, client_name: &str) -> Option<(String, String)> {
    let (s, m) = g.example()?;
    if !m.path_params.is_empty() || params_arg(g.client, m).is_some() {
        return None;
    }
    let mut ctx = g.ctx(Pkg::Root);
    for p in ["context", "fmt", "net/http", "reflect", "testing", "time"] {
        ctx.import(p);
    }
    ctx.import(&g.target.module_path);
    ctx.import_module("internal");
    ctx.import_module("option");
    let body = CLIENT_TEST
        .split_once('\n')
        .map_or("", |(_, rest)| rest)
        .replace("{{OPTIONS3}}", &client_options(g, 3))
        .replace("{{OPTIONS2}}", &client_options(g, 2))
        .replace("{{PKG}}", &g.target.package_name)
        .replace("{{CALL}}", &format!("{}.{}", s.accessor, g.method_name(m)))
        .replace("{{USER_AGENT}}", client_name);
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some((
        "client_test.go".to_owned(),
        go_file(&format!("{}_test", g.target.package_name), &imports, &body),
    ))
}
