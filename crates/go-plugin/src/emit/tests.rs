//! The tests Stainless generates: one `_test.go` per service (each method against the mock
//! server), `usage_test.go` (the README example) and `client_test.go` (retries, timeouts and the
//! User-Agent, against a fake transport).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ClientOpt, Field, Method, OptType, Scalar, Type};

use super::pagination::page_of;
use super::resource::params_decls;
use super::{Gen, Service, method_order};
use crate::naming::{exported, local, singular, split_words, words};
use crate::printer::{ALIGN, BREAK, go_file, quote};
use crate::types::{Ctx, Decl, Pkg, opt_scalar};

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

/// A test value for a field of type `go_ty` (the field's neutral type `ty`), for a line at
/// `indent` tabs, and whether an object literal in it sets an optional field; an object is a
/// multi-line literal.
fn example(
    g: &Gen<'_>,
    wire: &str,
    ty: &Type,
    decls: &[Decl],
    go_ty: &str,
    indent: usize,
    ctx: &mut Ctx<'_>,
) -> (String, bool) {
    let pkg = &g.target.package_name;
    let v2 = g.target.v2();
    let plain = |s: String| (s, false);
    match ty {
        Type::Nullable { inner } => example(g, wire, inner, decls, go_ty, indent, ctx),
        Type::Scalar { scalar } => plain(match scalar {
            Scalar::Boolean => "true".to_owned(),
            s if s.is_integer() => if v2 { "0" } else { "int64(0)" }.to_owned(),
            Scalar::Float | Scalar::Double => if v2 { "0" } else { "0.000000" }.to_owned(),
            Scalar::Date | Scalar::DateTime => {
                ctx.import("time");
                "time.Now()".to_owned()
            }
            Scalar::Uuid => "\"182bd5e5-6e1a-4fe4-a799-aa6d9a6ab26e\"".to_owned(),
            _ => crate::printer::quote(wire),
        }),
        // A declared enum's first constant; v2's enums taken as their underlying type (deeper
        // than the params struct) are their first value.
        Type::Enum { members } => plain(
            decls
                .iter()
                .find_map(|d| match d {
                    Decl::Enum { name, members, .. } if name == go_ty => {
                        members.first().map(|(c, _)| format!("{pkg}.{c}"))
                    }
                    _ => None,
                })
                .or_else(|| {
                    members.first().filter(|_| v2).map(|m| match &m.value {
                        ferrule_sdk_model::model::Literal::String(s) => crate::printer::quote(s),
                        ferrule_sdk_model::model::Literal::Boolean(b) => b.to_string(),
                        ferrule_sdk_model::model::Literal::Number(n) => n.to_string(),
                    })
                })
                .unwrap_or_else(|| crate::printer::quote(wire)),
        ),
        Type::Array { items } => {
            let item_ty = go_ty.trim_start_matches("[]");
            let (inner, optional) = example(g, wire, items, decls, item_ty, indent, ctx);
            let qualified = if item_ty.starts_with(|c: char| c.is_ascii_uppercase()) {
                format!("{pkg}.{item_ty}")
            } else {
                item_ty.to_owned()
            };
            // gofmt elides the element type of a composite literal's elements.
            let inner = inner
                .strip_prefix(&format!("{qualified}{{"))
                .map_or(inner.clone(), |rest| format!("{{{rest}"));
            (format!("[]{qualified}{{{inner}}}"), optional)
        }
        _ => match (g.client.object_of(ty), decls.iter().find(|d| d.name() == go_ty)) {
            (Some(obj), Some(Decl::Struct { name, fields, .. })) => {
                let sources: Vec<&Field> = obj.fields.iter().collect();
                struct_literal(g, name, fields, &sources, decls, indent, ctx)
            }
            _ => plain(crate::printer::quote(wire)),
        },
    }
}

/// The order a literal sets fields in: the struct's in v1; in v2 its path fields, then the
/// required fields, then the optional ones, each by wire name (knock-mgmt-go's tests list
/// `Before`, `Include`, `Limit` although the struct declares `Include` last).
fn literal_order(g: &Gen<'_>, fields: &[crate::types::GoField]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..fields.len()).collect();
    if g.target.v2() {
        order.sort_by(|&a, &b| {
            let (a, b) = (&fields[a], &fields[b]);
            (b.key == "path")
                .cmp(&(a.key == "path"))
                .then_with(|| b.required.cmp(&a.required))
                .then_with(|| (b.wire == "id").cmp(&(a.wire == "id")))
                .then_with(|| ferrule_sdk_model::js::cmp(&a.wire.to_lowercase(), &b.wire.to_lowercase()))
        });
    }
    order
}

/// `pkg.Name{` and one `Field: pkg.F(value),` line per field at `indent + 1` tabs, closed at
/// `indent`; also whether any field is optional.
fn struct_literal(
    g: &Gen<'_>,
    name: &str,
    fields: &[crate::types::GoField],
    sources: &[&Field],
    decls: &[Decl],
    indent: usize,
    ctx: &mut Ctx<'_>,
) -> (String, bool) {
    let pkg = &g.target.package_name;
    let tabs = "\t".repeat(indent + 1);
    let mut out = format!("{pkg}.{name}{{\n");
    let mut optional = false;
    let mut after_multiline = false;
    for f in literal_order(g, fields).into_iter().map(|i| &fields[i]) {
        let Some(src) = sources.iter().find(|x| x.wire_name == f.wire) else {
            continue;
        };
        optional |= !f.required;
        let (value, nested) = example(g, &src.wire_name, &src.ty, decls, &f.ty, indent + 1, ctx);
        // An optional field of a nested object names the test too (knock-mgmt-go's
        // `TestChannelGroupUpsertWithOptionalParams`).
        optional |= nested && g.target.v2();
        let multiline = value.contains('\n');
        // gofmt aligns keys only across single-line elements.
        if multiline || after_multiline {
            out.push(BREAK);
        }
        after_multiline = multiline;
        let value = if !g.target.v2() {
            format!("{pkg}.F({value})")
        } else if !f.required && opt_scalar(&f.ty) && f.enum_values.is_empty() {
            // v2 wraps an optional scalar in its `param.Opt` constructor.
            let ctor = match f.ty.as_str() {
                "string" => "String",
                "int64" => "Int",
                "float64" => "Float",
                "bool" => "Bool",
                _ => "Time",
            };
            format!("{pkg}.{ctor}({value})")
        } else {
            value
        };
        let _ = writeln!(out, "{tabs}{}:{ALIGN}{value},", f.name);
    }
    out.push_str(&"\t".repeat(indent));
    out.push('}');
    (out, optional)
}

/// A test value for a path parameter: its wire name for a string (Stainless's UUID for a
/// `uuid`, knock-mgmt-go), else the type's zero.
fn path_example(p: &ferrule_sdk_model::model::Param) -> String {
    if matches!(p.field.ty, Type::Scalar { scalar: Scalar::Uuid }) {
        return "\"182bd5e5-6e1a-4fe4-a799-aa6d9a6ab26e\"".to_owned();
    }
    match super::path_param_type(&p.field.ty) {
        "int64" => "int64(0)".to_owned(),
        "float64" => "0.000000".to_owned(),
        "bool" => "true".to_owned(),
        _ => crate::printer::quote(&p.field.wire_name),
    }
}

/// The params literal at `indent` tabs, and whether it has optional fields.
fn params_literal(
    g: &Gen<'_>,
    s: &Service<'_>,
    m: &Method,
    indent: usize,
    ctx: &mut Ctx<'_>,
) -> Option<(String, bool)> {
    g.params_arg(m)?;
    let mut scratch = g.ctx(Pkg::Root);
    let decls = params_decls(g, s, m, &mut scratch);
    let Some(Decl::Struct { name, fields, .. }) = decls.first() else {
        return None;
    };
    let sources: Vec<&Field> = g
        .path_fields(m)
        .iter()
        .chain(&m.query_params)
        .map(|p| &p.field)
        .chain(super::body_fields(g.client, m))
        .collect();
    Some(struct_literal(g, name, fields, &sources, &decls, indent, ctx))
}

/// The call of `m` in a test: `client.Agency.Get(context.TODO(), "agencyID")`, with the params
/// literal on its own lines when there are path params too. Returns the text after `:= ` or
/// `= ` and whether the params have optional fields.
fn call(g: &Gen<'_>, s: &Service<'_>, m: &Method, ctx_arg: &str, ctx: &mut Ctx<'_>) -> (String, bool) {
    let target = format!("{}.{}", s.accessor, g.method_name(m));
    let path_args: Vec<String> = m.path_params[g.path_fields(m).len()..]
        .iter()
        .map(path_example)
        .collect();
    match params_literal(g, s, m, if path_args.is_empty() { 1 } else { 2 }, ctx) {
        None => {
            let mut args = vec![ctx_arg.to_owned()];
            args.extend(path_args);
            (format!("{target}({})", args.join(", ")), false)
        }
        Some((literal, optional)) if path_args.is_empty() => (format!("{target}({ctx_arg}, {literal})"), optional),
        Some((literal, optional)) => {
            let mut out = format!("{target}(\n\t\t{ctx_arg},\n");
            for a in &path_args {
                let _ = writeln!(out, "\t\t{a},");
            }
            let _ = write!(out, "\t\t{literal},\n\t)");
            (out, optional)
        }
    }
}

const PREAMBLE: &str = "\tbaseURL := \"http://localhost:4010\"\n\tif envURL, ok := os.LookupEnv(\"TEST_API_BASE_URL\"); ok {\n\t\tbaseURL = envURL\n\t}\n\tif !testutil.CheckTestServer(t, baseURL) {\n\t\treturn\n\t}\n";

/// The start of a test against the mock server: skipped first under
/// `settings.disable_mock_tests`.
fn preamble(g: &Gen<'_>) -> String {
    if g.target.disable_mock_tests {
        format!("\tt.Skip(\"Mock server tests are disabled\")\n{PREAMBLE}")
    } else {
        PREAMBLE.to_owned()
    }
}

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
    let preamble = preamble(g);
    let mut body = String::new();
    for m in method_order(&s.resource.methods) {
        let (call, optional) = call(g, s, m, "context.TODO()", &mut ctx);
        let suffix = if optional { "WithOptionalParams" } else { "" };
        let returns_value = !matches!(
            super::resource::returns(g, s, m, &mut g.ctx(Pkg::Root)).0,
            super::resource::Returns::None
        );
        let results = if returns_value { "_, err" } else { "err" };
        let _ = write!(
            body,
            "func Test{}{}{suffix}(t *testing.T) {{\n{preamble}{}\t{results} := {call}\n\tif err != nil {{\n\t\tvar apierr *{pkg}.Error\n\t\tif errors.As(err, &apierr) {{\n\t\t\tt.Log(string(apierr.DumpRequest(true)))\n\t\t}}\n\t\tt.Fatalf(\"err should be nil: %s\", err.Error())\n\t}}\n}}\n\n",
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
    m.response.as_ref()?;
    let mut ctx = g.ctx(Pkg::Root);
    test_imports(g, &mut ctx);
    // A page is logged as `page` (knock-mgmt-go).
    let var = if page_of(g, m).is_some() {
        "page".to_owned()
    } else {
        local(&singular(&split_words(&s.resource.name.written)), g.casings)
    };
    let (call, _) = call(g, s, m, "context.TODO()", &mut ctx);
    let preamble = preamble(g);
    let body = format!(
        "func TestUsage(t *testing.T) {{\n{preamble}{}\t{var}, err := {call}\n\tif err != nil {{\n\t\tt.Error(err)\n\t\treturn\n\t}}\n\tt.Logf(\"%+v\\n\", {var})\n}}\n",
        new_client(g)
    );
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some((
        "usage_test.go".to_owned(),
        go_file(&format!("{}_test", g.target.package_name), &imports, &body),
    ))
}

/// `paginationauto_test.go` and `paginationmanual_test.go`: the README's pagination example
/// (`meta.pagination_example`, else the first paginated method without path params or required
/// params) against the mock server, logging each item's `ID`. Not emitted when the items have
/// no `id` (what the oracles log then is unverified for v1).
#[must_use]
pub fn pagination_test_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let Some((s, m, page)) = g.pagination_example() else {
        return Vec::new();
    };
    let Some(item) = m.pagination.as_ref().and_then(|p| g.client.object_of(&p.item)) else {
        return Vec::new();
    };
    let Some(id) = item.fields.iter().find(|f| f.wire_name == "id") else {
        return Vec::new();
    };
    let field = exported(&words(&id.name), g.casings);
    let pkg = &g.target.package_name;
    let var = local(&singular(&split_words(&s.resource.name.written)), g.casings);
    let mut args = String::from("context.TODO()");
    if g.params_arg(m).is_some() {
        let _ = write!(args, ", {pkg}.{}{}Params{{}}", s.prefix, g.method_name(m));
    }
    let target = format!("{}.{}", s.accessor, g.method_name(m));
    let mock = "// The mock server isn't going to give us real pagination";
    let preamble = preamble(g);
    let fatal = "t.Fatalf(\"err should be nil: %s\", err.Error())";
    let auto = format!(
        "func TestAutoPagination(t *testing.T) {{\n{preamble}{}\titer := {target}AutoPaging({args})\n\t{mock}\n\
         \tfor i := 0; i < 3 && iter.Next(); i++ {{\n\t\t{var} := iter.Current()\n\t\tt.Logf(\"%+v\\n\", {var}.{field})\n\t}}\n\
         \tif err := iter.Err(); err != nil {{\n\t\t{fatal}\n\t}}\n}}\n",
        new_client(g)
    );
    let items = &page.items;
    let manual = format!(
        "func TestManualPagination(t *testing.T) {{\n{preamble}{}\tpage, err := {target}({args})\n\tif err != nil {{\n\t\t{fatal}\n\t}}\n\
         \tfor _, {var} := range page.{items} {{\n\t\tt.Logf(\"%+v\\n\", {var}.{field})\n\t}}\n\t{mock}\n\
         \tpage, err = page.GetNextPage()\n\tif err != nil {{\n\t\t{fatal}\n\t}}\n\tif page != nil {{\n\
         \t\tfor _, {var} := range page.{items} {{\n\t\t\tt.Logf(\"%+v\\n\", {var}.{field})\n\t\t}}\n\t}}\n}}\n",
        new_client(g)
    );
    let mut ctx = g.ctx(Pkg::Root);
    test_imports(g, &mut ctx);
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    let file = |body: &str| go_file(&format!("{pkg}_test"), &imports, body);
    vec![
        ("paginationauto_test.go".to_owned(), file(&auto)),
        ("paginationmanual_test.go".to_owned(), file(&manual)),
    ]
}

/// `client_test.go`, calling the README example method.
#[must_use]
pub fn client_test_file(g: &Gen<'_>, client_name: &str) -> Option<(String, String)> {
    let (s, m) = g.example()?;
    m.response.as_ref()?;
    // Path params as in the method tests; an empty params struct (the client tests only look at
    // the transport).
    let mut args = String::new();
    for p in &m.path_params[g.path_fields(m).len()..] {
        let _ = write!(args, ", {}", path_example(p));
    }
    if g.params_arg(m).is_some() {
        let _ = write!(
            args,
            ", {}.{}{}Params{{}}",
            g.target.package_name,
            s.prefix,
            g.method_name(m)
        );
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
        .replace("{{USER_AGENT}}", client_name)
        .replace("{{ARGS}}", &args);
    // The retry tests count one attempt more than the configured retries (oracles: 2 -> 3).
    let attempts = g.client.settings.as_ref().map_or(2, |s| s.max_retries) + 1;
    let counts = |count: fn(u32) -> String| (0..attempts).map(|i| quote(&count(i))).collect::<Vec<_>>().join(", ");
    let body = body
        .replace("{{ATTEMPTS}}", &attempts.to_string())
        .replace("{{RETRY_COUNTS}}", &counts(|i| i.to_string()))
        .replace("{{EMPTY_COUNTS}}", &counts(|_| String::new()))
        .replace("{{OVERWRITTEN_COUNTS}}", &counts(|_| "42".to_owned()));
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some((
        "client_test.go".to_owned(),
        go_file(&format!("{}_test", g.target.package_name), &imports, &body),
    ))
}
