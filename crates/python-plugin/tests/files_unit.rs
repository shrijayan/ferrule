//! Unit tests of the project-file emitters. The inputs are the TypeScript ones as JSON, so the
//! whole output is also compared with the TypeScript emitter.

mod files_support;

use ferrule_python_plugin::emit::files::api_md::{emit_api_md, import_line};
use ferrule_python_plugin::emit::files::api_tests::{Uses, emit_api_tests, example_value};
use ferrule_python_plugin::emit::files::input::{FilesMethod, FilesResource, ProjectFilesInput};
use ferrule_python_plugin::emit::files::render::{RenderError, TemplateVars, isort_compare, render};
use ferrule_python_plugin::emit::files::{emit_project_files, emit_stats, template_vars};
use ferrule_python_plugin::model::{HttpVerb, SdkType};
use files_support::{input, onebusaway_json, sdk_type};
use serde_json::{Value, json};

fn str_type() -> Value {
    json!({ "kind": "primitive", "name": "string" })
}

fn comments() -> Value {
    json!({
        "name": "comments",
        "title": "Comments",
        "variable": "comment",
        "module": "resources.cases.comments",
        "types": [],
        "typesModule": "types.cases",
        "methods": [{
            "name": "delete",
            "httpMethod": "delete",
            "path": "/cases/{case_token}/comments/{comment_token}",
            "params": [
                { "pyName": "case_token", "wireName": "case_token", "required": true,
                  "location": "path", "type": str_type() },
                { "pyName": "comment_token", "wireName": "comment_token", "required": true,
                  "location": "path", "type": str_type(), "example": "c-1" },
            ],
        }],
        "subresources": [],
    })
}

fn cases() -> Value {
    let mut empty = comments();
    empty["name"] = json!("empty");
    empty["title"] = json!("Empty");
    empty["methods"] = json!([]);
    json!({
        "name": "cases",
        "title": "Cases",
        "variable": "case",
        "module": "resources.cases.cases",
        "types": ["Case", "CaseCreateResponse"],
        "typesModule": "types",
        "methods": [{
            "name": "create",
            "httpMethod": "post",
            "path": "/cases",
            "params": [
                { "pyName": "items", "wireName": "items", "required": true, "location": "body",
                  "type": { "kind": "array", "items": { "kind": "object", "bases": [], "fields": [
                      { "wireName": "at", "pyName": "at", "required": true, "description": "",
                        "type": { "kind": "primitive", "name": "datetime" } },
                      { "wireName": "on", "pyName": "on", "required": true, "description": "",
                        "type": { "kind": "primitive", "name": "date" } },
                  ] } } },
                { "pyName": "count", "wireName": "count", "required": false, "location": "query",
                  "type": { "kind": "primitive", "name": "integer" }, "minimum": 1 },
                { "pyName": "name", "wireName": "name", "required": false, "location": "body",
                  "type": str_type(), "example": "n" },
            ],
            "paramsModule": "types.case_create_params",
            "response": { "name": "CaseCreateResponse", "module": "types.case_create_response" },
        }],
        "subresources": [comments(), empty],
    })
}

fn acme_json() -> Value {
    let probe = json!({
        "accessor": "cases", "method": "create", "args": "name=\"n\"",
        "httpMethod": "post", "path": "/cases",
    });
    json!({
        "edition": "python.2025-10-08",
        "packageName": "acme",
        "distributionName": "acme-sdk",
        "clientClass": "Acme",
        "envPrefix": "ACME",
        "repo": "acme/acme-python",
        "version": "0.1.0",
        "copyrightYear": 2030,
        "organization": {
            "name": "acme", "title": "Acme", "contact": "sec@acme.test",
            "docsUrl": "https://docs.acme.test/",
        },
        "readmeTitle": "Acme",
        "auth": { "kind": "api_key_query", "env": "ACME_API_KEY", "param": "token" },
        "query": { "arrayFormat": "comma", "objectFormat": "brackets" },
        "stats": {
            "openapiSpecUrl": "https://example.test/spec.yml",
            "openapiSpecHash": "abc",
            "configHash": "def",
        },
        "readmeExample": probe,
        "retryProbe": probe,
        "sharedTypes": [],
        "resources": [cases()],
    })
}

fn acme() -> ProjectFilesInput {
    input(&acme_json())
}

fn file<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("no {path}"), |(_, t)| t.as_str())
}

// render

#[test]
fn render_fills_tokens_and_resorts_marked_imports() {
    let vars = TemplateVars::from([("X", "1".to_owned())]);
    assert_eq!(
        render("a %%X%% b\n%%isort%%from p import Longer, A\n", &vars),
        Ok("a 1 b\nfrom p import A, Longer\n".to_owned())
    );
}

#[test]
fn render_rejects_unknown_tokens_and_bad_isort_lines() {
    let none = TemplateVars::new();
    let unknown = render("%%NOPE%%", &none).expect_err("unknown token");
    assert_eq!(unknown, RenderError::UnknownToken("%%NOPE%%".to_owned()));
    assert!(unknown.to_string().contains("no value"));
    let bad = render("%%isort%%import x", &none).expect_err("not an import");
    assert!(bad.to_string().contains("one-line import"));
}

#[test]
fn isort_orders_like_ruff_length_sort() {
    let mut names = vec!["bar", "Foo", "FOO", "ab", "Bb", "Ba"];
    names.sort_by(|a, b| isort_compare(a, b));
    assert_eq!(names, ["Ba", "Bb", "ab", "FOO", "Foo", "bar"]);
}

// project files

#[test]
fn fills_every_template_with_the_input() {
    let files = emit_project_files(&acme());
    assert!(file(&files, "LICENSE").contains("Copyright 2030 Acme"));
    assert!(file(&files, "pyproject.toml").contains("name = \"acme-sdk\""));
    assert!(file(&files, "tests/conftest.py").contains("from acme import Acme, AsyncAcme, DefaultAioHttpClient"));
    assert!(
        file(&files, "tests/test_client.py")
            .contains("client.cases.with_raw_response.create(name=\"n\", extra_headers=")
    );
    assert!(file(&files, "README.md").contains("[docs.acme.test](https://docs.acme.test/)"));
    assert!(file(&files, "scripts/mock").contains("--validator-query-array-format=comma"));
    for (path, text) in &files {
        for banned in ["%%", "Stainless]", "by Stainless"] {
            assert!(!text.contains(banned), "{path} contains {banned}");
        }
    }
    let paths: Vec<&String> = files.iter().map(|(p, _)| p).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);
}

#[test]
fn leaves_the_probe_args_comma_out_when_there_are_no_args() {
    let mut input = acme();
    input.retry_probe.args = Vec::new();
    let vars = template_vars(&input);
    assert_eq!(vars["PROBE_ARGS_COMMA"], "");
    assert_eq!(vars["EX_RESULT"], "cases");
}

#[test]
fn writes_stats_yml() {
    assert_eq!(
        emit_stats(&acme()),
        "configured_endpoints: 2\nopenapi_spec_url: https://example.test/spec.yml\nopenapi_spec_hash: abc\nconfig_hash: def\n"
    );
}

#[test]
fn counts_onebusaway_endpoints() {
    let stats = emit_stats(&input(&onebusaway_json()));
    assert!(stats.starts_with("configured_endpoints: 30\n"), "{stats}");
}

#[test]
fn counts_an_endpoint_bound_to_two_methods_once() {
    // lumaai binds `post /generations/video` to `generations.create` and
    // `generations.video.create` (x-forge-aliases); its .stats.yml says 13 endpoints for 14 methods.
    let mut input = acme();
    let mut alias = input.resources[0].methods[0].clone();
    alias.name = "create_alias".to_owned();
    input.resources[0].methods.push(alias);
    assert!(
        emit_stats(&input).starts_with("configured_endpoints: 2\n"),
        "{}",
        emit_stats(&input)
    );
}

// api.md

#[test]
fn api_md_lists_nested_resources_keyword_only_path_params_and_none_returns() {
    let mut input = acme();
    input.shared_types = vec!["Shared".to_owned()];
    let md = emit_api_md(&input);
    assert!(md.contains("# Shared Types\n\n```python\nfrom acme.types import Shared\n```"));
    assert!(md.contains("## Comments\n\nMethods:"));
    assert!(md.contains("(comment_token, \\*, case_token) -> None</code>"));
    let after = &md[md.find("## Comments\n").expect("a Comments section")..];
    assert!(after.ends_with("## Empty\n"), "{md}");
    assert!(md.contains("(\\*\\*<a href=\"src/acme/types/case_create_params.py\">params</a>)"));
}

#[test]
fn api_md_wraps_long_imports() {
    assert_eq!(import_line("m", &["A", "B"], 10), "from m import (\n    A,\n    B,\n)");
}

// api tests

#[test]
fn writes_a_module_per_resource_with_methods_and_an_init_per_package() {
    let mut paths: Vec<String> = emit_api_tests(&acme()).into_iter().map(|(p, _)| p).collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            "tests/api_resources/cases/__init__.py",
            "tests/api_resources/cases/test_comments.py",
            "tests/api_resources/test_cases.py",
        ]
    );
}

#[test]
fn renders_examples_and_helper_imports() {
    let tests = emit_api_tests(&acme());
    let text = file(&tests, "tests/api_resources/test_cases.py");
    assert!(text.contains("from acme._utils import parse_date, parse_datetime"));
    assert!(text.contains("\"at\": parse_datetime(\"2019-12-27T18:11:19.117Z\"),"));
    assert!(text.contains("count=1,\n            name=\"n\","));
    assert!(text.contains("async with async_client.cases.with_streaming_response.create("));
}

#[test]
fn checks_every_path_parameter_and_none_results() {
    let tests = emit_api_tests(&acme());
    let text = file(&tests, "tests/api_resources/cases/test_comments.py");
    assert!(text.contains("comment_token=\"c-1\",\n            case_token=\"case_token\","));
    assert!(text.contains("`case_token` but received"));
    assert!(text.contains("`comment_token` but received"));
    assert!(text.contains("assert comment is None"));
    assert!(!text.contains("assert_matches_type"));
}

#[test]
fn wraps_a_long_assert_matches_type_call() {
    let long = "X".repeat(100);
    let mut input = acme();
    let r: &mut FilesResource = &mut input.resources[0];
    r.subresources.clear();
    r.methods = vec![FilesMethod {
        name: "get".to_owned(),
        http_method: HttpVerb::Get,
        path: "/x".to_owned(),
        params: Vec::new(),
        params_module: None,
        response: Some(ferrule_python_plugin::emit::files::input::PyName {
            name: long.clone(),
            module: "types.x".to_owned(),
        }),
        stream_switch: None,
    }];
    let tests = emit_api_tests(&input);
    let text = file(&tests, "tests/api_resources/test_cases.py");
    assert!(text.contains(&format!(
        "assert_matches_type(\n            {long}, case, path=[\"response\"]\n        )"
    )));
}

/// A param-switched streaming method (stagehand `sessions.act`, ogx `completions.create`): the
/// tests come twice, `_overload_1` with the switch `False` in the all-params call and
/// `_overload_2` with it `True` after the required arguments, closing the stream.
#[test]
fn tests_a_param_switched_streaming_method_per_overload() {
    let mut json = acme_json();
    json["resources"][0]["subresources"] = json!([]);
    json["resources"][0]["methods"] = json!([{
        "name": "act",
        "httpMethod": "post",
        "path": "/cases/{id}/act",
        "params": [
            { "pyName": "id", "wireName": "id", "required": true, "location": "path", "type": str_type() },
            { "pyName": "input", "wireName": "input", "required": true, "location": "body", "type": str_type() },
            { "pyName": "frame_id", "wireName": "frameId", "required": false, "location": "body",
              "type": str_type() },
            { "pyName": "stream_response", "wireName": "streamResponse", "required": false,
              "location": "body", "type": { "kind": "primitive", "name": "boolean" } },
        ],
        "paramsModule": "types.case_act_params",
        "response": { "name": "CaseActResponse", "module": "types.case_act_response" },
        "streamSwitch": "stream_response",
    }]);
    let tests = emit_api_tests(&input(&json));
    let text = file(&tests, "tests/api_resources/test_cases.py");
    let sync = r#"    @parametrize
    def test_method_act_overload_1(self, client: Acme) -> None:
        case = client.cases.act(
            id="id",
            input="input",
        )
        assert_matches_type(CaseActResponse, case, path=["response"])

    @parametrize
    def test_method_act_with_all_params_overload_1(self, client: Acme) -> None:
        case = client.cases.act(
            id="id",
            input="input",
            frame_id="frameId",
            stream_response=False,
        )
        assert_matches_type(CaseActResponse, case, path=["response"])
"#;
    assert!(text.contains(sync), "{text}");
    let streamed = r#"    @parametrize
    def test_method_act_overload_2(self, client: Acme) -> None:
        case_stream = client.cases.act(
            id="id",
            input="input",
            stream_response=True,
        )
        case_stream.response.close()

    @parametrize
    def test_method_act_with_all_params_overload_2(self, client: Acme) -> None:
        case_stream = client.cases.act(
            id="id",
            input="input",
            stream_response=True,
            frame_id="frameId",
        )
        case_stream.response.close()

    @parametrize
    def test_raw_response_act_overload_2(self, client: Acme) -> None:
        response = client.cases.with_raw_response.act(
            id="id",
            input="input",
            stream_response=True,
        )

        assert response.http_request.headers.get("X-Stainless-Lang") == "python"
        stream = response.parse()
        stream.close()

    @parametrize
    def test_streaming_response_act_overload_2(self, client: Acme) -> None:
        with client.cases.with_streaming_response.act(
            id="id",
            input="input",
            stream_response=True,
        ) as response:
            assert not response.is_closed
            assert response.http_request.headers.get("X-Stainless-Lang") == "python"

            stream = response.parse()
            stream.close()

        assert cast(Any, response.is_closed) is True

    @parametrize
    def test_path_params_act_overload_2(self, client: Acme) -> None:
        with pytest.raises(ValueError, match=r"Expected a non-empty value for `id` but received ''"):
            client.cases.with_raw_response.act(
                id="",
                input="input",
                stream_response=True,
            )
"#;
    assert!(text.contains(streamed), "{text}");
    for line in [
        "        case_stream = await async_client.cases.act(\n",
        "        await case_stream.response.aclose()\n",
        "        stream = await response.parse()\n        await stream.close()\n",
        "            stream = await response.parse()\n            await stream.close()\n",
    ] {
        assert!(text.contains(line), "{line}");
    }
    assert!(text.contains("    def test_path_params_act_overload_1(self, client: Acme) -> None:\n"));
}

/// Header parameters (ogx `models.list`, stagehand `sessions.end`): in the all-params call after
/// every other argument, valued as body parameters are (the wire name for a string, the first
/// value of an enum).
#[test]
fn passes_header_params_last_in_the_all_params_call() {
    let mut json = acme_json();
    json["resources"][0]["subresources"] = json!([]);
    let enum_type = json!({ "kind": "literal", "values": ["true", "false"] });
    json["resources"][0]["methods"] = json!([
        {
            "name": "list",
            "httpMethod": "get",
            "path": "/cases",
            "params": [
                { "pyName": "zone", "wireName": "zone", "required": false, "location": "query", "type": str_type() },
                { "pyName": "anthropic_version", "wireName": "anthropic-version", "required": false,
                  "location": "header", "type": str_type() },
                { "pyName": "x_goog_api_key", "wireName": "x-goog-api-key", "required": false,
                  "location": "header", "type": str_type() },
            ],
            "paramsModule": "types.case_list_params",
            "response": { "name": "CaseListResponse", "module": "types.case_list_response" },
        },
        {
            "name": "end",
            "httpMethod": "post",
            "path": "/cases/{id}/end",
            "params": [
                { "pyName": "id", "wireName": "id", "required": true, "location": "path", "type": str_type() },
                { "pyName": "x_stream_response", "wireName": "x-stream-response", "required": false,
                  "location": "header", "type": enum_type },
            ],
            "response": { "name": "CaseEndResponse", "module": "types.case_end_response" },
        },
    ]);
    let tests = emit_api_tests(&input(&json));
    let text = file(&tests, "tests/api_resources/test_cases.py");
    let list = r#"    def test_method_list_with_all_params(self, client: Acme) -> None:
        case = client.cases.list(
            zone="zone",
            anthropic_version="anthropic-version",
            x_goog_api_key="x-goog-api-key",
        )
"#;
    assert!(text.contains(list), "{text}");
    let end = r#"    def test_method_end_with_all_params(self, client: Acme) -> None:
        case = client.cases.end(
            id="id",
            x_stream_response="true",
        )
"#;
    assert!(text.contains(end), "{text}");
}

// example_value

#[test]
fn example_value_covers_every_type() {
    let ex = |ty: Value, example: Option<Value>| -> String {
        let ty: SdkType = sdk_type(&ty);
        example_value(&ty, &mut Uses::new(), "", example.as_ref(), "w", None)
    };
    assert_eq!(ex(str_type(), None), "\"w\"");
    assert_eq!(ex(str_type(), Some(json!("e"))), "\"e\"");
    assert_eq!(ex(json!({ "kind": "primitive", "name": "number" }), None), "0");
    assert_eq!(ex(json!({ "kind": "primitive", "name": "boolean" }), None), "True");
    assert_eq!(ex(json!({ "kind": "primitive", "name": "null" }), None), "None");
    assert_eq!(ex(json!({ "kind": "literal", "values": ["a"] }), None), "\"a\"");
    assert_eq!(ex(json!({ "kind": "literal", "values": [true] }), None), "True");
    assert_eq!(ex(json!({ "kind": "literal", "values": [false] }), None), "False");
    assert_eq!(ex(json!({ "kind": "literal", "values": [3] }), None), "3");
    assert_eq!(ex(json!({ "kind": "union", "variants": [] }), None), "{}");
    assert_eq!(ex(json!({ "kind": "union", "variants": [str_type()] }), None), "\"w\"");
    assert_eq!(
        ex(json!({ "kind": "array", "items": str_type() }), None),
        "[\"string\"]"
    );
    assert_eq!(
        ex(json!({ "kind": "map", "values": str_type() }), None),
        "{\"foo\": \"string\"}"
    );
    assert_eq!(ex(json!({ "kind": "object", "bases": [], "fields": [] }), None), "{}");
    assert_eq!(ex(json!({ "kind": "unknown" }), None), "{}");
    assert_eq!(ex(json!({ "kind": "ref", "name": "R", "module": "r" }), None), "{}");
}

#[test]
fn example_value_matches_typescript_quirks() {
    let ty = |v: Value| sdk_type(&v);
    let mut uses = Uses::new();
    let number = ty(json!({ "kind": "primitive", "name": "number" }));
    // Numbers print as JavaScript `String(n)`; a spec example is ignored for numbers.
    assert_eq!(
        example_value(&number, &mut uses, "", Some(&json!(5)), "w", Some(-0.5)),
        "-0.5"
    );
    assert_eq!(example_value(&number, &mut uses, "", None, "w", Some(1e21)), "1e+21");
    // The bound is not passed on to a union's first variant.
    let union = ty(json!({ "kind": "union", "variants": [{ "kind": "primitive", "name": "integer" }] }));
    assert_eq!(example_value(&union, &mut uses, "", None, "w", Some(7.0)), "0");
    // An empty literal is `String(undefined)`.
    assert_eq!(
        example_value(
            &ty(json!({ "kind": "literal", "values": [] })),
            &mut uses,
            "",
            None,
            "w",
            None
        ),
        "undefined"
    );
    // Nested multi-line items are indented under the array.
    let nested = ty(
        json!({ "kind": "array", "items": { "kind": "object", "bases": [], "fields": [
        { "wireName": "d", "pyName": "d", "required": true, "description": "",
          "type": { "kind": "primitive", "name": "date" } },
    ] } }),
    );
    assert_eq!(
        example_value(&nested, &mut uses, "  ", None, "w", None),
        "[\n      {\n          \"d\": parse_date(\"2019-12-27\"),\n      }\n  ]"
    );
    assert_eq!(uses.into_iter().collect::<Vec<_>>(), ["parse_date"]);
}
