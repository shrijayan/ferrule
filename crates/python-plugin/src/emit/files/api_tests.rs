//! `tests/api_resources/**`: one test module per resource with methods, sync and async from one
//! pass ([`Mode`]), in the shape of the oracle SDKs' generated tests.

use std::collections::BTreeSet;

use serde_json::Value;

use super::api_md::import_line;
use super::input::{FilesMethod, FilesParam, FilesResource, ParamLocation, ProjectFilesInput};
use super::render::isort_compare;
use crate::emit::common::{HEADER, py_string};
use crate::js;
use crate::model::{SdkLiteral, SdkPrimitive, SdkType};

/// Helpers a test module imports from `<pkg>._utils` because an example value uses them:
/// `parse_date` and `parse_datetime`.
pub type Uses = BTreeSet<&'static str>;

/// The ISO string of every example date, which `parse_date` reads.
pub const EXAMPLE_DATE: &str = "2019-12-27";
/// The ISO string of every example datetime, which `parse_datetime` reads.
pub const EXAMPLE_DATETIME: &str = "2019-12-27T18:11:19.117Z";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Sync,
    Async,
}

fn primitive_example(
    name: SdkPrimitive,
    uses: &mut Uses,
    example: Option<&Value>,
    fallback: &str,
    bound: Option<f64>,
) -> String {
    match name {
        SdkPrimitive::String => match example {
            Some(Value::String(s)) => py_string(s),
            _ => py_string(fallback),
        },
        SdkPrimitive::Integer | SdkPrimitive::Number => js::number_to_string(bound.unwrap_or(0.0)),
        SdkPrimitive::Boolean => "True".to_owned(),
        SdkPrimitive::Null => "None".to_owned(),
        SdkPrimitive::Datetime => {
            uses.insert("parse_datetime");
            format!("parse_datetime({})", py_string(EXAMPLE_DATETIME))
        }
        SdkPrimitive::Date => {
            uses.insert("parse_date");
            format!("parse_date({})", py_string(EXAMPLE_DATE))
        }
    }
}

/// A Python literal for an example of `ty`, laid out the way ruff formats the oracle tests.
#[must_use]
pub fn example_value(
    ty: &SdkType,
    uses: &mut Uses,
    indent: &str,
    example: Option<&Value>,
    fallback: &str,
    bound: Option<f64>,
) -> String {
    match ty {
        SdkType::Primitive(name) => primitive_example(*name, uses, example, fallback, bound),
        SdkType::Literal(values) => match values.first() {
            Some(SdkLiteral::String(s)) => py_string(s),
            Some(SdkLiteral::Boolean(true)) => "True".to_owned(),
            Some(SdkLiteral::Boolean(false)) => "False".to_owned(),
            Some(SdkLiteral::Number(n)) => js::number_to_string(*n),
            // `String(undefined)` in the TypeScript.
            None => "undefined".to_owned(),
        },
        // The bound is not passed on to the first variant, as in the TypeScript.
        SdkType::Union { variants, .. } => variants.first().map_or_else(
            || "{}".to_owned(),
            |v| example_value(v, uses, indent, example, fallback, None),
        ),
        SdkType::Array(items) => {
            let item = example_value(items, uses, &format!("{indent}    "), None, "string", None);
            if item.contains('\n') {
                format!("[\n{indent}    {item}\n{indent}]")
            } else {
                format!("[{item}]")
            }
        }
        SdkType::Map(values) => format!(
            "{{\"foo\": {}}}",
            example_value(values, uses, indent, None, "string", None)
        ),
        SdkType::Object(object) => {
            if object.fields.is_empty() {
                return "{}".to_owned();
            }
            let inner = format!("{indent}    ");
            let lines: Vec<String> = object
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "{inner}{}: {},",
                        py_string(&f.wire_name),
                        example_value(&f.ty, uses, &inner, None, "string", None)
                    )
                })
                .collect();
            format!("{{\n{}\n{indent}}}", lines.join("\n"))
        }
        SdkType::Unknown | SdkType::Ref(_) => "{}".to_owned(),
    }
}

/// The example of a raw-bytes body, Stainless's for file content.
pub const BINARY_EXAMPLE: &str = "b\"raw file contents\"";

/// A parameter's example; query parameters ignore the spec's `example`, as in the oracles.
fn example_of(p: &FilesParam, uses: &mut Uses, indent: &str) -> String {
    if p.binary {
        return BINARY_EXAMPLE.to_owned();
    }
    let example = if p.location == ParamLocation::Query {
        None
    } else {
        p.example.as_ref()
    };
    example_value(&p.ty, uses, indent, example, &p.wire_name, p.minimum.or(p.maximum))
}

/// Call arguments: the last path parameter first (it is positional), then path order, then the
/// rest (required, then optional, each by name).
fn call_order<'a>(params: &[&'a FilesParam]) -> Vec<&'a FilesParam> {
    let path: Vec<&FilesParam> = params
        .iter()
        .copied()
        .filter(|p| p.location == ParamLocation::Path)
        .collect();
    let by_name = |a: &&FilesParam, b: &&FilesParam| js::cmp(&a.py_name, &b.py_name);
    let rest = params
        .iter()
        .copied()
        .filter(|p| !matches!(p.location, ParamLocation::Path | ParamLocation::Header));
    let mut required: Vec<&FilesParam> = rest.clone().filter(|p| p.required).collect();
    let mut optional: Vec<&FilesParam> = rest.filter(|p| !p.required).collect();
    required.sort_by(by_name);
    optional.sort_by(by_name);
    let mut ordered = Vec::with_capacity(params.len());
    if let Some((last, others)) = path.split_last() {
        ordered.push(*last);
        ordered.extend_from_slice(others);
    }
    ordered.extend(required);
    ordered.extend(optional);
    // Header parameters come last, in their own order (ogx `models.list`).
    ordered.extend(params.iter().copied().filter(|p| p.location == ParamLocation::Header));
    ordered
}

/// The call's argument list at `indent`; `empty` is the parameter passed as `""`.
fn args(params: &[&FilesParam], indent: &str, uses: &mut Uses, positional: bool, empty: Option<&FilesParam>) -> String {
    let Some(only) = params.first() else {
        return "()".to_owned();
    };
    let inner = format!("{indent}    ");
    let is_empty = |p: &FilesParam| empty.is_some_and(|e| std::ptr::eq(e, p));
    if params.len() == 1 && only.location == ParamLocation::Path && positional {
        let value = if is_empty(only) {
            "\"\"".to_owned()
        } else {
            example_of(only, uses, &inner)
        };
        return format!("(\n{inner}{value},\n{indent})");
    }
    let lines: Vec<String> = params
        .iter()
        .map(|p| {
            let value = if is_empty(p) {
                "\"\"".to_owned()
            } else {
                example_of(p, uses, &inner)
            };
            format!("{inner}{}={value},", p.py_name)
        })
        .collect();
    format!("(\n{}\n{indent})", lines.join("\n"))
}

struct Ctx<'a> {
    client: &'a str,
    accessor: &'a str,
    variable: &'a str,
    uses: Uses,
    /// `settings.disable_mock_tests`: every test is marked skipped, as in the isaacus oracle.
    skip: bool,
}

/// The result check at `indent`, split the way ruff splits a call past 120 columns.
fn assertion(m: &FilesMethod, variable: &str, indent: &str) -> String {
    let Some(response) = &m.response else {
        return format!("{indent}assert {variable} is None");
    };
    let call_args = format!("{}, {variable}, path=[\"response\"]", response.name);
    let line = format!("{indent}assert_matches_type({call_args})");
    if js::len(&line) <= 120 {
        line
    } else {
        format!("{indent}assert_matches_type(\n{indent}    {call_args}\n{indent})")
    }
}

fn is_required(p: &FilesParam) -> bool {
    p.required || p.location == ParamLocation::Path
}

/// The tests of a method; a param-switched streaming method gets them twice, as Stainless
/// writes them: `_overload_1` with the switch `False` (only in the all-params call), and
/// `_overload_2` with it `True` after the required arguments, closing the stream it returns.
fn method_tests(ctx: &mut Ctx<'_>, m: &FilesMethod, mode: Mode, tests: &mut Vec<String>) {
    let switch = m
        .stream_switch
        .as_deref()
        .and_then(|n| m.params.iter().find(|p| p.py_name == n));
    let Some(switch) = switch else {
        let all: Vec<&FilesParam> = m.params.iter().collect();
        let required: Vec<&FilesParam> = m.params.iter().filter(|p| is_required(p)).collect();
        call_tests(
            ctx,
            m,
            mode,
            tests,
            ("", false),
            &call_order(&required),
            &call_order(&all),
        );
        return;
    };
    let literal = |value: bool| SdkType::Literal(vec![SdkLiteral::Boolean(value)]);
    let off = FilesParam {
        ty: literal(false),
        required: false,
        ..switch.clone()
    };
    let on = FilesParam {
        ty: literal(true),
        required: true,
        ..switch.clone()
    };
    let others: Vec<&FilesParam> = m.params.iter().filter(|p| !std::ptr::eq(*p, switch)).collect();
    let required: Vec<&FilesParam> = call_order(&others.iter().copied().filter(|p| is_required(p)).collect::<Vec<_>>());
    let mut all: Vec<&FilesParam> = others.clone();
    all.push(&off);
    call_tests(
        ctx,
        m,
        mode,
        tests,
        ("_overload_1", false),
        &required,
        &call_order(&all),
    );
    let mut required_on = required.clone();
    required_on.push(&on);
    let mut all_on = call_order(&others);
    let at = all_on.iter().position(|p| !is_required(p)).unwrap_or(all_on.len());
    all_on.insert(at, &on);
    call_tests(ctx, m, mode, tests, ("_overload_2", true), &required_on, &all_on);
}

/// The method, raw, streaming and path-parameter tests of one call shape: `suffix` names them,
/// and `streamed` closes the `Stream` the call returns instead of checking a model.
fn call_tests(
    ctx: &mut Ctx<'_>,
    m: &FilesMethod,
    mode: Mode,
    tests: &mut Vec<String>,
    (suffix, streamed): (&str, bool),
    required: &[&FilesParam],
    all: &[&FilesParam],
) {
    let a = mode == Mode::Async;
    let def = if a { "async def" } else { "def" };
    let aw = if a { "await " } else { "" };
    let cl = if a { "async_client" } else { "client" };
    let ty = if a {
        format!("Async{}", ctx.client)
    } else {
        ctx.client.to_owned()
    };
    let skip = if ctx.skip {
        "    @pytest.mark.skip(reason=\"Mock server tests are disabled\")\n"
    } else {
        ""
    };
    let sig = |name: &str| format!("{skip}    @parametrize\n    {def} {name}{suffix}(self, {cl}: {ty}) -> None:");
    let target = format!("{cl}.{}", ctx.accessor);
    let name = &m.name;
    let v = if streamed {
        format!("{}_stream", ctx.variable)
    } else {
        ctx.variable.to_owned()
    };
    let v = v.as_str();
    // A method whose only parameter is one path parameter is called with it positionally.
    let pos = m.params.len() == 1;
    let i8 = "        ";
    let i12 = "            ";
    // What follows a call: the result checked, or the stream closed.
    let close = if a { "aclose" } else { "close" };
    let check = |indent: &str| {
        if streamed {
            format!("{indent}{aw}{v}.response.{close}()")
        } else {
            assertion(m, v, indent)
        }
    };
    let parsed = |indent: &str| {
        if streamed {
            format!("{indent}stream = {aw}response.parse()\n{indent}{aw}stream.close()")
        } else {
            format!("{indent}{v} = {aw}response.parse()\n{}", assertion(m, v, indent))
        }
    };

    let call = args(required, i8, &mut ctx.uses, pos, None);
    tests.push(format!(
        "{}\n{i8}{v} = {aw}{target}.{name}{call}\n{}",
        sig(&format!("test_method_{name}")),
        check(i8)
    ));
    if all.len() > required.len() {
        let call = args(all, i8, &mut ctx.uses, pos, None);
        tests.push(format!(
            "{}\n{i8}{v} = {aw}{target}.{name}{call}\n{}",
            sig(&format!("test_method_{name}_with_all_params")),
            check(i8)
        ));
    }
    let call = args(required, i8, &mut ctx.uses, pos, None);
    // A streamed response is not read, so it is still open.
    let closed = if streamed {
        String::new()
    } else {
        format!("{i8}assert response.is_closed is True\n")
    };
    tests.push(format!(
        "{}\n{i8}response = {aw}{target}.with_raw_response.{name}{call}\n\n\
         {closed}\
         {i8}assert response.http_request.headers.get(\"X-Stainless-Lang\") == \"python\"\n\
         {}",
        sig(&format!("test_raw_response_{name}")),
        parsed(i8)
    ));
    let call = args(required, i8, &mut ctx.uses, pos, None);
    tests.push(format!(
        "{}\n{i8}{} {target}.with_streaming_response.{name}{call} as response:\n\
         {i12}assert not response.is_closed\n\
         {i12}assert response.http_request.headers.get(\"X-Stainless-Lang\") == \"python\"\n\n\
         {}\n\n\
         {i8}assert cast(Any, response.is_closed) is True",
        sig(&format!("test_streaming_response_{name}")),
        if a { "async with" } else { "with" },
        parsed(i12)
    ));
    let path: Vec<&FilesParam> = m.params.iter().filter(|p| p.location == ParamLocation::Path).collect();
    if !path.is_empty() {
        let blocks: Vec<String> = path
            .iter()
            .map(|p| {
                let call = args(required, i12, &mut ctx.uses, pos, Some(p));
                format!(
                    "{i8}with pytest.raises(ValueError, match=r\"Expected a non-empty value for `{}` but received ''\"):\n\
                     {i12}{aw}{target}.with_raw_response.{name}{call}",
                    p.py_name
                )
            })
            .collect();
        tests.push(format!(
            "{}\n{}",
            sig(&format!("test_path_params_{name}")),
            blocks.join("\n\n")
        ));
    }
}

fn test_class(ctx: &mut Ctx<'_>, r: &FilesResource, mode: Mode) -> String {
    let head = match mode {
        Mode::Sync => format!(
            "class Test{}:\n    parametrize = pytest.mark.parametrize(\"client\", [False, True], indirect=True, ids=[\"loose\", \"strict\"])",
            r.title
        ),
        Mode::Async => format!(
            "class TestAsync{}:\n    parametrize = pytest.mark.parametrize(\n        \"async_client\", [False, True, {{\"http_client\": \"aiohttp\"}}], indirect=True, ids=[\"loose\", \"strict\", \"aiohttp\"]\n    )",
            r.title
        ),
    };
    let mut parts = vec![head];
    for m in &r.methods {
        method_tests(ctx, m, mode, &mut parts);
    }
    parts.join("\n\n")
}

fn parent_module(module: &str) -> &str {
    module.rsplit_once('.').map_or("", |(parent, _)| parent)
}

/// One test module, for resource `r` reached as `client.<accessor>`.
#[must_use]
pub fn emit_test_module(input: &ProjectFilesInput, r: &FilesResource, accessor: &str) -> String {
    let pkg = &input.package_name;
    let mut ctx = Ctx {
        client: &input.client_class,
        accessor,
        variable: &r.variable,
        uses: Uses::new(),
        skip: !input.mock_tests,
    };
    let sync = test_class(&mut ctx, r, Mode::Sync);
    let body = format!("{sync}\n\n\n{}\n", test_class(&mut ctx, r, Mode::Async));

    // Module -> names, each without duplicates; both are sorted below, so order doesn't matter.
    let mut imports: Vec<(String, Vec<String>)> = Vec::new();
    let mut add = |module: String, name: &str| {
        let at = imports.iter().position(|(m, _)| *m == module).unwrap_or_else(|| {
            imports.push((module, Vec::new()));
            imports.len() - 1
        });
        let names = &mut imports[at].1;
        if !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    };
    add(pkg.clone(), &input.client_class);
    add(pkg.clone(), &format!("Async{}", input.client_class));
    for m in &r.methods {
        let Some(response) = &m.response else {
            continue;
        };
        add("tests.utils".to_owned(), "assert_matches_type");
        add(format!("{pkg}.{}", parent_module(&response.module)), &response.name);
    }
    for u in &ctx.uses {
        add(format!("{pkg}._utils"), u);
    }
    imports.sort_by(|(a, _), (b, _)| isort_compare(a, b));
    let lines = imports.iter_mut().map(|(module, names)| {
        names.sort_by(|a, b| isort_compare(a, b));
        import_line(module, names, 88)
    });

    let mut out: Vec<String> = [
        HEADER,
        "",
        "from __future__ import annotations",
        "",
        "import os",
        "from typing import Any, cast",
        "",
        "import pytest",
        "",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    out.extend(lines);
    out.extend(
        [
            "",
            "base_url = os.environ.get(\"TEST_API_BASE_URL\", \"http://127.0.0.1:4010\")",
            "",
            "",
        ]
        .iter()
        .map(|s| (*s).to_owned()),
    );
    out.push(body);
    out.join("\n")
}

fn has_methods(r: &FilesResource) -> bool {
    !r.methods.is_empty() || r.subresources.iter().any(has_methods)
}

/// Every test module and package `__init__.py` under `tests/api_resources/`, in the order the
/// TypeScript inserts them (not sorted).
#[must_use]
pub fn emit_api_tests(input: &ProjectFilesInput) -> Vec<(String, String)> {
    fn walk(
        input: &ProjectFilesInput,
        resources: &[FilesResource],
        dir: &mut Vec<String>,
        parent: &str,
        files: &mut Vec<(String, String)>,
    ) {
        for r in resources {
            let accessor = if parent.is_empty() {
                r.name.clone()
            } else {
                format!("{parent}.{}", r.name)
            };
            let join = |last: &[&str]| {
                let mut parts = vec!["tests/api_resources"];
                parts.extend(dir.iter().map(String::as_str));
                parts.extend_from_slice(last);
                parts.join("/")
            };
            if !r.methods.is_empty() {
                let path = join(&[&format!("test_{}.py", r.name)]);
                files.push((path, emit_test_module(input, r, &accessor)));
            }
            // A package as in `src/`: a standalone API, or one whose subresources have tests.
            if r.standalone_link.is_some() || r.subresources.iter().any(has_methods) {
                let path = join(&[&r.name, "__init__.py"]);
                files.push((path, format!("{HEADER}\n")));
            }
            if !r.subresources.is_empty() {
                dir.push(r.name.clone());
                walk(input, &r.subresources, dir, &accessor, files);
                dir.pop();
            }
        }
    }
    let mut files = Vec::new();
    walk(input, &input.resources, &mut Vec::new(), "", &mut files);
    files
}
