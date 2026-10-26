//! The tests Stainless generates: `tests/UnsupportedMockTests.php` (`disable_mock_tests`),
//! `tests/ClientTest.php` (the default headers around the README's example request) and one
//! `tests/Services/<Path>Test.php` per resource (each method called against the mock server, with
//! its required parameters and with all of them, and its result checked by type).

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ClientOpt, Literal};

use super::{Gen, Meth, Res};
use crate::layout::{Doc, cat, indent, list, render, text};
use crate::naming::{camel, class_name, split_words, words};
use crate::tree::{Decl, PField};
use crate::types::{ClassPath, Imports, PType, literal};

/// Every test file.
#[must_use]
pub fn test_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let mut out = vec![(
        "tests/UnsupportedMockTests.php".to_owned(),
        format!(
            "<?php\n\nnamespace Tests;\n\nfinal class UnsupportedMockTests\n{{\n    public static bool $skip = {};\n}}\n",
            g.target.disable_mock_tests
        ),
    )];
    if let Some(t) = client_test(g) {
        out.push(t);
    }
    for r in &g.resources {
        if !r.methods.is_empty() {
            out.push(service_test(g, r));
        }
    }
    out
}

/// The options the tests construct the client with: those that authenticate, as `'My <Name>'`.
fn test_opts<'g>(g: &'g Gen<'_>) -> Vec<(String, &'g ClientOpt)> {
    let casings = g.casings();
    g.client
        .settings
        .as_ref()
        .map(|s| {
            s.opts
                .iter()
                .filter(|o| o.auth.is_some())
                .map(|o| (camel(&words(&o.name), casings), o))
                .collect()
        })
        .unwrap_or_default()
}

fn opt_value(o: &ClientOpt) -> String {
    let title: Vec<String> = split_words(&o.name.written)
        .iter()
        .map(|w| class_name(&[w], crate::naming::Casings(&[])))
        .collect();
    format!("'My {}'", title.join(" "))
}

fn find_class<'d>(g: &'d Gen<'_>, c: &ClassPath) -> Option<&'d crate::tree::Class> {
    g.decls.iter().find_map(|d| match d {
        Decl::Class(k) if k.path == *c => Some(k),
        _ => None,
    })
}

fn find_enum_first(g: &Gen<'_>, c: &ClassPath) -> Option<Literal> {
    g.decls.iter().find_map(|d| match d {
        Decl::Enum(e) if e.path == *c => e.members.first().map(|m| m.value.clone()),
        _ => None,
    })
}

/// An example value of a parameter, as the generated tests pass it (the spec's examples are not
/// in the model, Forge gap G18: a string is its wire name, a number 0).
fn example(g: &Gen<'_>, ty: &PType, wire: &str, all: bool, imp: &mut Imports, depth: usize) -> Doc {
    match ty {
        PType::String => text(literal(&Literal::String(wire.to_owned()))),
        PType::Date => text("'2019-12-27'"),
        PType::Int | PType::Float => text("0"),
        PType::Bool => text("true"),
        PType::DateTime => text("new \\DateTimeImmutable('2019-12-27T18:11:19.117Z')"),
        PType::Mixed => text("[]"),
        PType::File => {
            let file = imp.runtime(&["Core", "FileParam"]);
            text(format!(
                "{file}::fromString('Example data', filename: uniqid('file-upload-', true))"
            ))
        }
        PType::Const(v) => text(literal(v)),
        PType::List(t) => list("[", vec![example(g, t, wire, all, imp, depth + 1)], "]"),
        PType::Map(t) => list(
            "[",
            vec![cat(vec![text("'foo' => "), example(g, t, "foo", all, imp, depth + 1)])],
            "]",
        ),
        PType::Nullable(t) | PType::Rec(t) => example(g, t, wire, all, imp, depth),
        PType::Enum(c) => text(find_enum_first(g, c).map_or_else(|| "''".to_owned(), |v| literal(&v))),
        PType::Union(_, variants) => variants
            .first()
            .map_or_else(|| text("[]"), |v| example(g, v, wire, all, imp, depth)),
        PType::Model(c) => {
            let entries = find_class(g, c).map_or_else(Vec::new, |k| {
                k.fields
                    .iter()
                    .filter(|f| if all { true } else { f.required })
                    .filter(|f| depth < 4 || f.required)
                    .map(|f| {
                        cat(vec![
                            text(format!("'{}' => ", f.name)),
                            example(g, &f.ty, &f.wire, all, imp, depth + 1),
                        ])
                    })
                    .collect()
            });
            list("[", entries, "]")
        }
    }
}

fn args(g: &Gen<'_>, m: &Meth<'_>, all: bool, imp: &mut Imports) -> Vec<Doc> {
    let mut out = Vec::new();
    if let Some(p) = &m.args.positional {
        out.push(example(g, &p.ty, &p.wire, all, imp, 0));
    }
    let fields: Vec<&PField> = m
        .args
        .params
        .as_ref()
        .map(|c| c.fields.iter().collect())
        .unwrap_or_default();
    let mut ordered: Vec<&PField> = fields.iter().copied().filter(|f| f.must_pass()).collect();
    if all {
        ordered.extend(fields.iter().copied().filter(|f| !f.must_pass()));
    }
    for f in ordered {
        out.push(cat(vec![
            text(format!("{}: ", f.name)),
            example(g, &f.ty, &f.wire, all, imp, 0),
        ]));
    }
    out
}

fn assertion(m: &Meth<'_>, imp: &mut Imports) -> String {
    let ind = indent(2);
    let ignore = format!("{ind}// @phpstan-ignore-next-line method.alreadyNarrowedType\n");
    match &m.response {
        None => format!("{ignore}{ind}$this->assertNull($result);\n"),
        Some(PType::Model(c)) => format!(
            "{ignore}{ind}$this->assertInstanceOf({}::class, $result);\n",
            imp.class(c)
        ),
        Some(PType::List(_)) => format!("{ignore}{ind}$this->assertIsList($result);\n"),
        Some(_) => format!("{ignore}{ind}$this->assertNotNull($result);\n"),
    }
}

fn test_method(g: &Gen<'_>, accessor: &str, m: &Meth<'_>, all: bool, imp: &mut Imports) -> String {
    let ind = indent(1);
    let ind2 = indent(2);
    let mut name = class_name(&split_words(&m.name), g.casings());
    if all {
        name.push_str("WithOptionalParams");
    }
    let mut out = format!("{ind}#[Test]\n{ind}public function test{name}(): void\n{ind}{{\n");
    if g.target.disable_mock_tests {
        let _ = write!(
            out,
            "{ind2}if (UnsupportedMockTests::$skip) {{\n{ind2}    $this->markTestSkipped('Mock server tests are disabled');\n{ind2}}}\n\n"
        );
    }
    let call = cat(vec![
        text("$result = "),
        list(
            format!("$this->client->{accessor}->{}(", m.name),
            args(g, m, all, imp),
            ")",
        ),
        text(";"),
    ]);
    let _ = write!(out, "{ind2}{}\n\n", render(&call, 2, 4, 0));
    out.push_str(&assertion(m, imp));
    let _ = writeln!(out, "{ind}}}");
    out
}

fn set_up(g: &Gen<'_>) -> String {
    let mut args: Vec<String> = test_opts(g)
        .iter()
        .map(|(n, o)| format!("{n}: {}", opt_value(o)))
        .collect();
    args.push("baseUrl: $testUrl".to_owned());
    // Always one argument per line.
    let lines: String = args.iter().fold(String::new(), |mut acc, a| {
        let _ = writeln!(acc, "            {a},");
        acc
    });
    let call = format!("$client = new Client(\n{lines}        );");
    format!(
        "    protected Client $client;\n\n    protected function setUp(): void\n    {{\n        parent::setUp();\n\n        $testUrl = Util::getenv('TEST_API_BASE_URL') ?: 'http://127.0.0.1:4010';\n        {call}\n\n        $this->client = $client;\n    }}\n"
    )
}

fn service_test(g: &Gen<'_>, r: &Res<'_>) -> (String, String) {
    let mut ns = vec!["Services".to_owned()];
    ns.extend(r.segs[..r.segs.len() - 1].iter().cloned());
    let mut imp = Imports::outside(&g.ns);
    imp.class(&ClassPath::new(vec!["Client".to_owned()]));
    imp.runtime(&["Core", "Util"]);
    let accessor = accessor(g, r);
    let mut tests = Vec::new();
    for m in &r.methods {
        tests.push(test_method(g, &accessor, m, false, &mut imp));
        if m.args
            .params
            .as_ref()
            .is_some_and(|c| c.fields.iter().any(PField::must_pass))
        {
            tests.push(test_method(g, &accessor, m, true, &mut imp));
        }
    }
    let mut uses = imp.use_lines();
    uses.extend([
        "use PHPUnit\\Framework\\Attributes\\CoversNothing;".to_owned(),
        "use PHPUnit\\Framework\\Attributes\\Test;".to_owned(),
        "use PHPUnit\\Framework\\TestCase;".to_owned(),
    ]);
    if g.target.disable_mock_tests {
        uses.push("use Tests\\UnsupportedMockTests;".to_owned());
    }
    uses.sort_by_key(|l| l.replace('\\', " ").to_ascii_lowercase());
    let name = format!("{}Test", r.name());
    let mut out = format!(
        "<?php\n\nnamespace Tests\\{};\n\n{}\n\n",
        ns.join("\\"),
        uses.join("\n")
    );
    out.push_str("/**\n * @internal\n */\n#[CoversNothing]\n");
    let _ = writeln!(out, "final class {name} extends TestCase\n{{");
    out.push_str(&set_up(g));
    for t in tests {
        out.push('\n');
        out.push_str(&t);
    }
    out.push_str("}\n");
    let mut path = vec!["tests".to_owned()];
    path.extend(ns);
    path.push(format!("{name}.php"));
    (path.join("/"), out)
}

/// `files->versions`: the resource's properties from the client down.
fn accessor(g: &Gen<'_>, r: &Res<'_>) -> String {
    let mut out: Vec<String> = Vec::new();
    for i in 1..=r.segs.len() {
        if let Some(res) = g.resources.iter().find(|x| x.segs == r.segs[..i]) {
            out.push(res.attr.clone());
        }
    }
    out.join("->")
}

/// The README's example request: `PhpMeta::example`, else the first method with no required
/// parameter.
fn example_method<'g>(g: &'g Gen<'_>) -> Option<(&'g Res<'g>, &'g Meth<'g>)> {
    if let Some((path, method)) = &g.target.example {
        let found = g.resources.iter().find_map(|r| {
            let keys: Vec<&str> = r.resource.path.iter().map(|n| n.written.as_str()).collect();
            (keys == path.iter().map(String::as_str).collect::<Vec<_>>())
                .then(|| {
                    r.methods
                        .iter()
                        .find(|m| m.method.name.written == *method)
                        .map(|m| (r, m))
                })
                .flatten()
        });
        if found.is_some() {
            return found;
        }
    }
    g.resources.iter().find_map(|r| {
        r.methods
            .iter()
            .find(|m| m.args.positional.is_none() && !m.args.rest().iter().any(|f| f.must_pass()))
            .map(|m| (r, m))
    })
}

fn client_test(g: &Gen<'_>) -> Option<(String, String)> {
    let (r, m) = example_method(g)?;
    let mut imp = Imports::outside(&g.ns);
    imp.external("Http\\Discovery\\Psr17FactoryDiscovery");
    imp.external("Http\\Mock\\Client");
    imp.runtime(&["Core", "Util"]);
    let call_args = args(g, m, false, &mut imp);
    let mut uses = imp.use_lines();
    uses.push("use PHPUnit\\Framework\\TestCase;".to_owned());
    uses.sort_by_key(|l| l.replace('\\', " ").to_ascii_lowercase());
    let mut ctor: Vec<String> = vec!["baseUrl: 'http://localhost'".to_owned()];
    ctor.extend(test_opts(g).iter().map(|(n, o)| format!("{n}: {}", opt_value(o))));
    ctor.push("requestOptions: ['transporter' => $transporter]".to_owned());
    let client = cat(vec![
        text("$client = "),
        list(
            format!("new \\{}\\Client(", g.ns),
            ctor.into_iter().map(text).collect(),
            ")",
        ),
        text(";"),
    ]);
    let call = cat(vec![
        list(format!("$client->{}->{}(", accessor(g, r), m.name), call_args, ")"),
        text(";"),
    ]);
    let mut out = format!("<?php\n\nnamespace Tests;\n\n{}\n\n", uses.join("\n"));
    out.push_str("/**\n * @internal\n *\n * @coversNothing\n */\nclass ClientTest extends TestCase\n{\n");
    out.push_str("    public function testDefaultHeaders(): void\n    {\n        $transporter = new Client;\n");
    out.push_str("        $mockRsp = Psr17FactoryDiscovery::findResponseFactory()\n            ->createResponse()\n            ->withStatus(200)\n            ->withHeader('Content-Type', 'application/json')\n            ->withBody(Psr17FactoryDiscovery::findStreamFactory()->createStream(json_encode([], flags: Util::JSON_ENCODE_FLAGS) ?: ''))\n        ;\n\n");
    out.push_str("        $transporter->setDefaultResponse($mockRsp);\n\n");
    let _ = write!(out, "        {}\n\n", render(&client, 2, 4, 0));
    let _ = write!(out, "        {}\n\n", render(&call, 2, 4, 0));
    out.push_str("        $this->assertNotFalse($requested = $transporter->getRequests()[0] ?? false);\n\n");
    out.push_str("        foreach (['accept', 'content-type'] as $header) {\n            $sent = $requested->getHeaderLine($header);\n            $this->assertNotEmpty($sent);\n        }\n    }\n}\n");
    Some(("tests/ClientTest.php".to_owned(), out))
}
