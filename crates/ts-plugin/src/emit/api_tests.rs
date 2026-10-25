//! `tests/api-resources/**`: one test file per resource with methods, calling each method with
//! only its required parameters and with all of them, against the mock server. The `node` layout
//! tests a method without required params by calling it bare, then passing request options in
//! the params' place and after the params (`isRequestOptions`).

use super::{File, HEADER};
use crate::ctx::{Ctx, TsResource};
use crate::doc::{concat, group, text};
use crate::emit::resource::{ParamsArg, plan_methods};
use crate::examples::{Examples, Fill};
use crate::naming::{camel_case, words_of};
use crate::pretty::{Expr, WIDTH, call_arguments, const_decl, render};
use crate::types::Mapper;

/// `'My API Key'` for `api_key`: the placeholder the generated tests construct the client with.
#[must_use]
pub fn placeholder(words: &[String]) -> String {
    let title: Vec<String> = words
        .iter()
        .map(|w| {
            let upper = w.to_ascii_uppercase();
            if ["API", "ID", "URL"].contains(&upper.as_str()) {
                upper
            } else {
                let mut c = w.chars();
                c.next()
                    .map_or_else(String::new, |f| f.to_ascii_uppercase().to_string() + c.as_str())
            }
        })
        .collect();
    format!("My {}", title.join(" "))
}

/// `const client = new X({ ... });` with the auth options and the mock server's base URL.
#[must_use]
pub fn client_setup(ctx: &Ctx<'_>) -> Vec<String> {
    let mut out = vec![format!("const client = new {}({{", ctx.settings.client_class)];
    if let Some(settings) = &ctx.client.settings {
        for opt in &settings.opts {
            if opt.auth.is_some() && !opt.nullable {
                let words = words_of(&opt.name);
                out.push(format!(
                    "  {}: {},",
                    camel_case(&words, &ctx.client.casings),
                    crate::js::string_literal(&placeholder(&words))
                ));
            }
        }
    }
    out.push("  baseURL: process.env['TEST_API_BASE_URL'] ?? 'http://127.0.0.1:4010',".to_owned());
    out.push("});".to_owned());
    out
}

const PROMISE_CHECKS: [&str; 7] = [
    "const rawResponse = await responsePromise.asResponse();",
    "expect(rawResponse).toBeInstanceOf(Response);",
    "const response = await responsePromise;",
    "expect(response).not.toBeInstanceOf(Response);",
    "const dataAndResponse = await responsePromise.withResponse();",
    "expect(dataAndResponse.data).toBe(response);",
    "expect(dataAndResponse.response).toBe(rawResponse);",
];

fn test_head(ctx: &Ctx<'_>, name: &str) -> Vec<String> {
    if ctx.settings.disable_mock_tests {
        vec![
            "  // Mock server tests are disabled".to_owned(),
            format!("  test.skip({}, async () => {{", crate::js::string_literal(name)),
        ]
    } else {
        vec![format!("  test({}, async () => {{", crate::js::string_literal(name))]
    }
}

/// A test that a call with request options pointing at an unknown path fails with a 404.
fn rejects_test(ctx: &Ctx<'_>, name: &str, callee: &str, args: Vec<Expr>) -> Vec<String> {
    let mut t = test_head(ctx, name);
    t.push(
        "    // ensure the request options are being passed correctly by passing an invalid HTTP method in order to cause an error"
            .to_owned(),
    );
    let call = Expr::Call(callee.to_owned(), args);
    let not_found = Expr::Raw(format!("{}.NotFoundError", ctx.settings.client_class));
    let doc = concat(vec![
        text("await "),
        group(vec![
            text("expect"),
            call_arguments(&[call]),
            text(".rejects.toThrow"),
            call_arguments(&[not_found]),
        ]),
        text(";"),
    ]);
    t.push(render(&doc, 4, WIDTH));
    t.push("  });".to_owned());
    t
}

/// The tests of a `node`-layout resource's methods.
fn node_tests(ctx: &Ctx<'_>, r: &TsResource<'_>, accessor: &str) -> Vec<Vec<String>> {
    let ex = Examples { ctx };
    let mut mapper = Mapper::new(ctx, &r.module());
    let unknown_path = || Expr::object(vec![("path".to_owned(), Expr::str("/_stainless_unknown_path"))]);
    let mut tests: Vec<Vec<String>> = Vec::new();
    for plan in plan_methods(ctx, r, &mut mapper) {
        let callee = format!("{accessor}.{}", plan.name);
        let required = ex.args(plan.m, Fill::Required);
        let all = ex.args(plan.m, Fill::All);
        let bare = plan.params_arg == ParamsArg::None || !plan.params_required;
        // A required body without required fields: one test, with an empty body.
        let empty = !bare && required.props.is_empty() && required.positional.len() == plan.m.path_params.len();
        let head = if bare || empty {
            plan.name.clone()
        } else {
            format!("{}: only required params", plan.name)
        };
        let mut t = test_head(ctx, &head);
        let mut args = required.exprs();
        if empty {
            args.push(Expr::object(Vec::new()));
        }
        let call = Expr::Call(callee.clone(), args);
        t.push(render(&const_decl("responsePromise", &call), 4, WIDTH));
        t.extend(PROMISE_CHECKS.iter().map(|l| format!("    {l}")));
        t.push("  });".to_owned());
        tests.push(t);
        if empty {
            continue;
        }
        if !bare {
            let mut t = test_head(ctx, &format!("{}: required and optional params", plan.name));
            let call = Expr::Await(Box::new(Expr::Call(callee, all.exprs())));
            t.push(render(&const_decl("response", &call), 4, WIDTH));
            t.push("  });".to_owned());
            tests.push(t);
            continue;
        }
        let mut args = required.positional.clone();
        args.push(unknown_path());
        tests.push(rejects_test(
            ctx,
            &format!("{}: request options instead of params are passed correctly", plan.name),
            &callee,
            args,
        ));
        if plan.params_arg != ParamsArg::None {
            let mut args = all.exprs();
            args.push(unknown_path());
            tests.push(rejects_test(
                ctx,
                &format!("{}: request options and params are passed correctly", plan.name),
                &callee,
                args,
            ));
        }
    }
    tests
}

/// The test file of one resource.
#[must_use]
pub fn emit_tests(ctx: &Ctx<'_>, r: &TsResource<'_>) -> String {
    let chain = ctx.chain(r);
    let accessor = r.accessor(&chain);
    let ex = Examples { ctx };
    let methods = if ctx.node() {
        Vec::new()
    } else {
        super::resource::sorted_methods(r)
    };
    let mut tests: Vec<Vec<String>> = if ctx.node() {
        node_tests(ctx, r, &accessor)
    } else {
        Vec::new()
    };
    for m in methods {
        let name = ctx.member(&words_of(&m.name));
        let callee = format!("{accessor}.{name}");
        let required = ex.args(m, Fill::Required);
        let all = ex.args(m, Fill::All);
        let mut t = test_head(ctx, &format!("{name}: only required params"));
        let call = Expr::Call(callee.clone(), required.exprs());
        t.push(render(&const_decl("responsePromise", &call), 4, WIDTH));
        t.extend(PROMISE_CHECKS.iter().map(|l| format!("    {l}")));
        t.push("  });".to_owned());
        tests.push(t);
        let mut t = test_head(ctx, &format!("{name}: required and optional params"));
        let call = Expr::Await(Box::new(Expr::Call(callee, all.exprs())));
        t.push(render(&const_decl("response", &call), 4, WIDTH));
        t.push("  });".to_owned());
        tests.push(t);
    }
    let mut out = vec![
        HEADER.to_owned(),
        String::new(),
        format!(
            "import {} from '{}';",
            ctx.settings.client_class, ctx.settings.package_name
        ),
    ];
    if ctx.node() {
        out.push("import { Response } from 'node-fetch';".to_owned());
    }
    out.push(String::new());
    out.extend(client_setup(ctx));
    out.push(String::new());
    out.push(format!(
        "describe({}, () => {{",
        crate::js::string_literal(&format!("resource {}", r.prop))
    ));
    for (i, t) in tests.into_iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.extend(t);
    }
    out.push("});".to_owned());
    out.join("\n") + "\n"
}

/// Every resource's test file: `tests/api-resources/<path>.test.ts`, a package's in its directory.
#[must_use]
pub fn test_files(ctx: &Ctx<'_>) -> Vec<File> {
    ctx.all()
        .into_iter()
        .filter(|r| !r.res.methods.is_empty())
        .map(|r| {
            let path = if r.package && !r.children.is_empty() {
                format!("{}/{}", r.files.join("/"), r.files.last().map_or("", String::as_str))
            } else {
                r.files.join("/")
            };
            (format!("tests/api-resources/{path}.test.ts"), emit_tests(ctx, r))
        })
        .collect()
}
