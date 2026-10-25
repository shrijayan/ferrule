//! `tests/api-resources/**`: one test file per resource with methods, calling each method with
//! only its required parameters and with all of them, against the mock server.

use super::{File, HEADER};
use crate::ctx::{Ctx, TsResource};
use crate::examples::{Examples, Fill};
use crate::naming::{camel_case, words_of};
use crate::pretty::{Expr, WIDTH, const_decl, render};

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

/// The test file of one resource.
#[must_use]
pub fn emit_tests(ctx: &Ctx<'_>, r: &TsResource<'_>) -> String {
    let chain = ctx.chain(r);
    let accessor = r.accessor(&chain);
    let ex = Examples { ctx };
    let mut methods: Vec<_> = r.res.methods.iter().collect();
    methods.sort_by_key(|m| m.order);
    let mut tests: Vec<Vec<String>> = Vec::new();
    for m in methods {
        let name = camel_case(&words_of(&m.name), &ctx.client.casings);
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
        String::new(),
    ];
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
