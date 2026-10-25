//! The tests Stainless generates: `test/<gem>/test_helper.rb`, `resource_namespaces.rb`, one
//! `resources/<resource>_test.rb` per resource (each method called against the mock server and
//! its response matched by pattern) and `client_test.rb` (retries, redirects and headers around
//! the README's example request).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ClientOpt, Resource, SecurityScheme};

use super::{Gen, Meth, Res};
use crate::doc::{Doc, assign, call, concat, group, indent, join, print_rb, text};
use crate::naming::{class_name, snake, split_words, words};
use crate::printer::quote;
use crate::tree::{Class, Decl, RField};
use crate::types::RType;

/// The test helper (`__M__`, `__gem__`, `__client__`, `__SUPER__`).
const TEST_HELPER: &str = include_str!("test_helper.rb.txt");

/// The variable the tests name the client: `onebusaway_sdk` for `onebusaway-sdk`.
#[must_use]
pub fn client_var(g: &Gen<'_>) -> String {
    g.client
        .settings
        .as_ref()
        .map_or_else(|| g.target.lib.clone(), |s| snake(&split_words(&s.name.written)))
}

/// `{a: 1}` as Stainless writes a hash literal: no spaces inside the braces.
fn hash(entries: Vec<Doc>) -> Doc {
    if entries.is_empty() {
        return text("{}");
    }
    group(concat(vec![
        text("{"),
        indent(concat(vec![
            Doc::SoftLine,
            join(&concat(vec![text(","), Doc::Line]), entries),
        ])),
        Doc::SoftLine,
        text("}"),
    ]))
}

/// An example value of a parameter, as the generated tests pass it.
fn example(g: &Gen<'_>, ty: &RType, wire: &str) -> Doc {
    match ty {
        RType::String => text(quote(wire)),
        RType::Integer | RType::Float => text("0"),
        RType::Boolean => text("true"),
        RType::Time => text("\"2019-12-27T18:11:19.117Z\""),
        RType::Date => text("\"2019-12-27\""),
        RType::Unknown | RType::Union(_) => text("{}"),
        RType::File => text("StringIO.new(\"Example data\")"),
        RType::Array(t) => concat(vec![text("["), example(g, t, wire), text("]")]),
        RType::Map(t) => hash(vec![concat(vec![text("foo: "), example(g, t, "foo")])]),
        RType::Nullable(t) => example(g, t, wire),
        RType::Enum(r) => {
            let first = g.models.iter().find_map(|f| find_enum(&f.decl, r));
            text(first.unwrap_or_else(|| ":".to_owned()))
        }
        RType::Model(r) => {
            let class = g.models.iter().find_map(|f| find_class(&f.decl, r));
            let entries = class.map_or_else(Vec::new, |c| {
                c.fields
                    .iter()
                    .filter(|f| f.required)
                    .map(|f| concat(vec![text(format!("{}: ", f.name)), example(g, &f.ty, &f.wire)]))
                    .collect()
            });
            hash(entries)
        }
    }
}

fn find_class<'d>(d: &'d Decl, r: &crate::types::ClassRef) -> Option<&'d Class> {
    match d {
        Decl::Class(c) if c.r == *r => Some(c),
        Decl::Class(c) => c.nested.iter().find_map(|n| find_class(n, r)),
        Decl::Union(u) => u.nested.iter().find_map(|n| find_class(n, r)),
        Decl::Enum(_) => None,
    }
}

fn find_enum(d: &Decl, r: &crate::types::ClassRef) -> Option<String> {
    match d {
        Decl::Enum(e) if e.r == *r => e.members.first().map(|m| crate::printer::literal(&m.value)),
        Decl::Class(c) => c.nested.iter().find_map(|n| find_enum(n, r)),
        Decl::Union(u) => u.nested.iter().find_map(|n| find_enum(n, r)),
        Decl::Enum(_) => None,
    }
}

fn params_of<'g>(g: &'g Gen<'_>, m: &Meth<'_>) -> &'g Class {
    g.models
        .iter()
        .find_map(|f| match &f.decl {
            Decl::Class(c) if c.r == m.params => Some(c),
            _ => None,
        })
        .expect("every method has a params class")
}

/// The arguments of a call with only the required parameters: path parameters positional.
fn required_args(g: &Gen<'_>, m: &Meth<'_>) -> Vec<Doc> {
    let c = params_of(g, m);
    let (path, rest) = c.fields.split_at(m.method.path_params.len());
    let mut args: Vec<Doc> = path.iter().map(|f| example(g, &f.ty, &f.wire)).collect();
    args.extend(
        rest.iter()
            .filter(|f| f.required)
            .map(|f| concat(vec![text(format!("{}: ", f.name)), example(g, &f.ty, &f.wire)])),
    );
    args
}

/// `receiver.method(args)`, or without parentheses when there are no arguments.
fn invoke(receiver: &str, args: Vec<Doc>) -> Doc {
    if args.is_empty() {
        text(receiver)
    } else {
        call(receiver, args)
    }
}

fn shape(g: &Gen<'_>, f: &RField) -> String {
    let m = &g.m;
    let t = match &f.ty {
        RType::Boolean | RType::Unknown => f.ty.rb_inner(m),
        RType::Array(_) | RType::Map(_) => format!("^({})", f.ty.rb_inner(m)),
        other => other.rb_inner(m),
    };
    if !f.required || f.nullable {
        format!("{t} | nil")
    } else {
        t
    }
}

fn method_test(g: &Gen<'_>, out: &mut String, accessor: &str, m: &Meth<'_>) {
    let c = params_of(g, m);
    let required = c.fields[m.method.path_params.len()..].iter().any(|f| f.required);
    let name = if required {
        format!("test_{}_required_params", m.name)
    } else {
        format!("test_{}", m.name)
    };
    let _ = writeln!(out, "  def {name}");
    let call = invoke(
        &format!("@{}.{accessor}.{}", client_var(g), m.name),
        required_args(g, m),
    );
    let _ = writeln!(out, "    {}", print_rb(&assign("response", call), 4));
    out.push('\n');
    out.push_str("    assert_pattern do\n");
    match &m.response {
        None => out.push_str("      response => nil\n"),
        Some(t) => {
            let _ = writeln!(out, "      response => {}", t.rb_inner_pattern(&g.m));
        }
    }
    out.push_str("    end\n");
    if let Some(RType::Model(r)) = &m.response
        && let Some(class) = g.models.iter().find_map(|f| find_class(&f.decl, r))
        && class.base.is_none()
    {
        out.push_str("\n    assert_pattern do\n      response => {\n");
        if class.fields.is_empty() {
            out.push_str("        **_\n");
        } else {
            let lines: Vec<String> = class
                .fields
                .iter()
                .map(|f| format!("        {}: {}", f.name, shape(g, f)))
                .collect();
            out.push_str(&lines.join(",\n"));
            out.push('\n');
        }
        out.push_str("      }\n    end\n");
    }
    out.push_str("  end\n");
}

impl RType {
    /// A response's pattern: the class, or `^(converter)` for a container.
    #[must_use]
    pub fn rb_inner_pattern(&self, m: &str) -> String {
        match self {
            Self::Array(_) | Self::Map(_) => format!("^({})", self.rb_inner(m)),
            other => other.rb_inner(m),
        }
    }
}

/// The resource's class, its path from `resources/` and its accessor from the client.
fn resource_test(g: &Gen<'_>, r: &Resource, parents: &[String]) -> (String, String) {
    let mut class = parents.to_vec();
    class.push(class_name(&words(&r.name), g.casings()));
    let file = r.path.iter().map(|n| snake(&words(n))).collect::<Vec<_>>().join("/");
    let res: Option<&Res<'_>> = g.resources.iter().find(|x| x.class == class);
    let up = "../".repeat(class.len());
    let mut out = format!("# frozen_string_literal: true\n\nrequire_relative \"{up}test_helper\"\n\n");
    let _ = writeln!(
        out,
        "class {m}::Test::Resources::{}Test < {m}::Test::ResourceTest",
        class.join("::"),
        m = g.m
    );
    if let Some(res) = res {
        let accessor = g
            .resources
            .iter()
            .filter(|x| class.starts_with(&x.class))
            .map(|x| x.attr.as_str())
            .collect::<Vec<_>>()
            .join(".");
        for (i, m) in res.methods.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            method_test(g, &mut out, &accessor, m);
        }
    }
    out.push_str("end\n");
    (format!("test/{}/resources/{file}_test.rb", g.target.lib), out)
}

/// Every resource's test file, the resources without methods included (an empty class).
#[must_use]
pub fn resource_tests(g: &Gen<'_>) -> Vec<(String, String)> {
    fn walk(g: &Gen<'_>, r: &Resource, parents: &[String], out: &mut Vec<(String, String)>) {
        out.push(resource_test(g, r, parents));
        let mut class = parents.to_vec();
        class.push(class_name(&words(&r.name), g.casings()));
        for s in &r.subresources {
            walk(g, s, &class, out);
        }
    }
    let mut out = Vec::new();
    for r in &g.client.resources {
        walk(g, r, &[], &mut out);
    }
    // stainless.yml's `shared` resource declares models only; it still gets an (empty) test.
    let shared = g
        .client
        .models
        .iter()
        .any(|d| d.r#ref.path.first().is_some_and(|n| n.written == "shared"));
    if shared && !g.client.resources.iter().any(|r| r.name.written == "shared") {
        out.push((
            format!("test/{}/resources/shared_test.rb", g.target.lib),
            format!(
                "# frozen_string_literal: true\n\nrequire_relative \"../test_helper\"\n\nclass {m}::Test::Resources::SharedTest < {m}::Test::ResourceTest\nend\n",
                m = g.m
            ),
        ));
    }
    out
}

/// The options the tests construct the client with, and their placeholder values.
fn test_opts(g: &Gen<'_>) -> Vec<(String, String)> {
    let Some(s) = g.client.settings.as_ref() else {
        return Vec::new();
    };
    s.opts
        .iter()
        .filter(|o: &&ClientOpt| o.auth.is_some() || !o.nullable)
        .map(|o| {
            let title = class_name(&split_words(&o.name.written), g.casings());
            let spaced = split_words(&title)
                .iter()
                .map(|w| class_name(&[w], g.casings()))
                .collect::<Vec<_>>();
            let _ = spaced;
            let words: Vec<String> = split_words(&o.name.written)
                .iter()
                .map(|w| class_name(&[w], g.casings()))
                .collect();
            (o.name.written.clone(), format!("My {}", words.join(" ")))
        })
        .collect()
}

/// `test_helper.rb` and `resource_namespaces.rb`.
#[must_use]
pub fn helper_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let m = &g.m;
    let mut args = vec![text(format!("base_url: {m}::Test::SingletonClient::TEST_API_BASE_URL"))];
    args.extend(
        test_opts(g)
            .into_iter()
            .map(|(k, v)| text(format!("{k}: {}", quote(&v)))),
    );
    let sup = format!("    {}\n", print_rb(&call("super", args), 4));
    let helper = crate::printer::template(TEST_HELPER)
        .replace("__SUPER__", &sup)
        .replace("__M__", m)
        .replace("__gem__", &g.target.lib)
        .replace("__client__", &client_var(g));
    let namespaces = format!(
        "# frozen_string_literal: true\n\nmodule {m}\n  module Test\n    module Resources\n    end\n  end\nend\n"
    );
    vec![
        (format!("test/{}/test_helper.rb", g.target.lib), helper),
        (format!("test/{}/resource_namespaces.rb", g.target.lib), namespaces),
    ]
}

/// The method `client_test.rb` exercises: the configured example, else the first method
/// without required parameters, else the first.
fn example_method<'g>(g: &'g Gen<'_>) -> Option<(&'g Res<'g>, &'g Meth<'g>)> {
    if let Some((path, method)) = &g.target.example {
        for r in &g.resources {
            let keys: Vec<&str> = g
                .resources
                .iter()
                .filter(|x| r.class.starts_with(&x.class))
                .map(|x| x.resource.name.written.as_str())
                .collect();
            if keys == path.iter().map(String::as_str).collect::<Vec<_>>()
                && let Some(m) = r.methods.iter().find(|m| m.method.name.written == *method)
            {
                return Some((r, m));
            }
        }
    }
    let all = || g.resources.iter().flat_map(|r| r.methods.iter().map(move |m| (r, m)));
    all()
        .find(|(_, m)| required_args(g, m).is_empty())
        .or_else(|| all().next())
}

/// The URL `client_test.rb` stubs: the example's path, with the query authentication.
fn stub_url(g: &Gen<'_>, m: &Meth<'_>) -> String {
    let mut path = m.method.path.clone();
    let c = params_of(g, m);
    for (p, f) in m.method.path_params.iter().zip(&c.fields) {
        path = path.replace(&format!("{{{}}}", p.field.wire_name), &f.wire);
    }
    let mut url = format!("http://localhost{path}");
    if let Some(s) = g.client.settings.as_ref() {
        let opts = test_opts(g);
        let query: Vec<String> = s
            .opts
            .iter()
            .filter_map(|o| {
                let scheme = s
                    .security_schemes
                    .iter()
                    .find(|n| Some(&n.name) == o.auth.as_ref().map(|a| &a.security_scheme))?;
                match &scheme.scheme {
                    SecurityScheme::ApiKey {
                        location: ferrule_sdk_model::model::ApiKeyLocation::Query,
                        name,
                    } => {
                        let v = opts.iter().find(|(k, _)| *k == o.name.written)?.1.replace(' ', "%20");
                        Some(format!("{name}={v}"))
                    }
                    _ => None,
                }
            })
            .collect();
        if !query.is_empty() {
            url.push('?');
            url.push_str(&query.join("&"));
        }
    }
    url
}

/// OneBusAway's `client_test.rb` with whole-line slots for what depends on the SDK.
const CLIENT_TEST: &str = include_str!("client_test.rb.txt");

/// The statements `client_test.rb` fills its slots with.
struct ClientTest<'a> {
    verb: &'a str,
    url: String,
    var: String,
    module: &'a str,
    opts: Vec<(String, String)>,
    receiver: String,
    args: Vec<Doc>,
}

impl ClientTest<'_> {
    fn stub(&self, status: u16, headers: Option<&str>) -> String {
        let mut args = vec![text(format!("status: {status}"))];
        if let Some(h) = headers {
            args.push(text(format!("headers: {h}")));
        }
        args.push(text("body: {}"));
        let d = concat(vec![
            call(
                "stub_request",
                vec![text(format!(":{}", self.verb)), text(quote(&self.url))],
            ),
            text("."),
            call("to_return_json", args),
        ]);
        format!("    {}\n", print_rb(&d, 4))
    }

    fn redirect(verb: &str, url: &str, status: u16, location: &str) -> String {
        let d = concat(vec![
            call("stub_request", vec![text(format!(":{verb}")), text(quote(url))]),
            text("."),
            call(
                "to_return",
                vec![
                    text(format!("status: {status}")),
                    text(format!("headers: {{\"location\" => {}}}", quote(location))),
                ],
            ),
        ]);
        format!("    {}\n", print_rb(&d, 4))
    }

    fn client(&self, extra: Option<&str>) -> String {
        let mut args = vec![text("base_url: \"http://localhost\"")];
        args.extend(self.opts.iter().map(|(k, v)| text(format!("{k}: {}", quote(v)))));
        args.extend(extra.map(text));
        let d = assign(&self.var, call(&format!("{}::Client.new", self.module), args));
        format!("    {}\n", print_rb(&d, 4))
    }

    fn call(&self, indent: usize, options: Option<&str>) -> String {
        let mut args = self.args.clone();
        args.extend(options.map(|o| text(format!("request_options: {o}"))));
        format!(
            "{}{}\n",
            " ".repeat(indent),
            print_rb(&invoke(&self.receiver, args), indent)
        )
    }
}

/// `client_test.rb`; nothing without a method to call.
#[must_use]
pub fn client_test(g: &Gen<'_>) -> Vec<(String, String)> {
    let Some((res, m)) = example_method(g) else {
        return Vec::new();
    };
    let var = client_var(g);
    let accessor = g
        .resources
        .iter()
        .filter(|x| res.class.starts_with(&x.class))
        .map(|x| x.attr.as_str())
        .collect::<Vec<_>>()
        .join(".");
    let t = ClientTest {
        verb: m.method.http_method.as_str(),
        url: stub_url(g, m),
        receiver: format!("{var}.{accessor}.{}", m.name),
        var,
        module: &g.m,
        opts: test_opts(g),
        args: required_args(g, m),
    };
    let slots: [(&str, String); 22] = [
        (
            "__STUB_RETRY_AFTER_SECONDS__",
            t.stub(500, Some("{\"retry-after\" => \"1.3\"}")),
        ),
        (
            "__STUB_RETRY_AFTER_DATE__",
            t.stub(500, Some("{\"retry-after\" => (time_now + 10).httpdate}")),
        ),
        (
            "__STUB_RETRY_AFTER_MS__",
            t.stub(500, Some("{\"retry-after-ms\" => \"1300\"}")),
        ),
        (
            "__STUB_307_CROSS__",
            t.stub(307, Some("{\"location\" => \"https://example.com/redirected\"}")),
        ),
        ("__STUB_307__", t.stub(307, Some("{\"location\" => \"/redirected\"}"))),
        ("__STUB_303__", t.stub(303, Some("{\"location\" => \"/redirected\"}"))),
        ("__STUB_500__", t.stub(500, None)),
        ("__STUB_200__", t.stub(200, None)),
        (
            "__REDIRECT_ANY__",
            ClientTest::redirect("any", "http://localhost/redirected", 307, "/redirected"),
        ),
        (
            "__REDIRECT_GET__",
            ClientTest::redirect("get", "http://localhost/redirected", 303, "/redirected"),
        ),
        (
            "__REDIRECT_CROSS__",
            ClientTest::redirect(
                "any",
                "https://example.com/redirected",
                307,
                "https://example.com/redirected",
            ),
        ),
        ("__CLIENT_RETRIES_3__", t.client(Some("max_retries: 3"))),
        ("__CLIENT_RETRIES_1__", t.client(Some("max_retries: 1"))),
        ("__CLIENT__", t.client(None)),
        ("__CALL_RETRIES_3__", t.call(6, Some("{max_retries: 3}"))),
        ("__CALL_RETRIES_4__", t.call(6, Some("{max_retries: 4}"))),
        (
            "__CALL_OMIT__",
            t.call(6, Some("{extra_headers: {\"x-stainless-retry-count\" => nil}}")),
        ),
        (
            "__CALL_42__",
            t.call(6, Some("{extra_headers: {\"x-stainless-retry-count\" => \"42\"}}")),
        ),
        ("__CALL_EMPTY__", t.call(6, Some("{extra_headers: {}}"))),
        (
            "__CALL_BEARER__",
            t.call(6, Some("{extra_headers: {\"authorization\" => \"Bearer xyz\"}}")),
        ),
        ("__CALL_DEFAULT__", t.call(4, None)),
        ("__CALL__", t.call(6, None)),
    ];
    let mut out = String::new();
    for line in crate::printer::template(CLIENT_TEST).split_inclusive('\n') {
        let key = line.trim();
        match slots.iter().find(|(k, _)| *k == key) {
            Some((_, v)) => out.push_str(v),
            None => out.push_str(&line.replace("__M__", &g.m)),
        }
    }
    vec![(format!("test/{}/client_test.rb", g.target.lib), out)]
}
