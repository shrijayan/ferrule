use ferrule_converter::schema::config::parse_config;
use ferrule_converter::value::{parse_json, to_json};
use ferrule_converter::{Diagnostic, Severity, Value, convert, convert_text, parse_spec};
use proptest::prelude::*;
use serde_json::json;

fn v(j: &serde_json::Value) -> Value {
    Value::from(j)
}

fn cursor_page() -> serde_json::Value {
    json!({
        "name": "cursor_page",
        "type": "cursor",
        "request": {
            "after": {"type": "string", "x-stainless-pagination-property": {"purpose": "next_cursor_param"}},
            "limit": {"type": "integer"}
        },
        "response": {
            "data": {"type": "array", "x-stainless-pagination-property": {"purpose": "items"}},
            "next_cursor": {"type": "string", "x-stainless-pagination-property": {"purpose": "next_cursor_field"}}
        }
    })
}

fn expected_pagination() -> Value {
    v(&json!({
        "scheme": "cursor",
        "page": "cursor_page",
        "paramLocation": "query",
        "continueOnEmptyItems": false,
        "request": {"after": "next_cursor_param", "limit": "limit"},
        "response": {"data": "items", "next_cursor": "next_cursor_field"}
    }))
}

fn make_spec_json() -> serde_json::Value {
    json!({
        "openapi": "3.1.0",
        "paths": {
            "/users": {
                "parameters": [{"$ref": "#/components/parameters/Limit"}],
                "get": {
                    "parameters": [
                        {"name": "after", "in": "query", "schema": {"type": "string"}},
                        {"name": "X-Trace", "in": "header"}
                    ],
                    "responses": {"200": {"$ref": "#/components/responses/UserPage"}}
                },
                "post": {"responses": {"201": {"description": "ok"}}}
            },
            "/users/{id}": {"get": {"responses": {}}},
            "/users/{id}/posts": {"get": {"responses": {"200": {"content": {"application/json": {"schema": {
                "allOf": [{"$ref": "#/components/schemas/Page"}, {"properties": {"extra": {"type": "string"}}}]
            }}}}}}},
            "/aliased": {"$ref": "#/paths/~1users~1{id}"},
            "/unconfigured": {"get": {"responses": {}}}
        },
        "components": {
            "parameters": {"Limit": {"name": "limit", "in": "query", "schema": {"type": "integer"}}},
            "responses": {"UserPage": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Page"}}}}},
            "schemas": {
                "Page": {"type": "object", "properties": {"data": {"type": "array"}, "next_cursor": {"type": "string"}}},
                "Loop": {"$ref": "#/components/schemas/Loop"}
            }
        }
    })
}

fn make_spec() -> Value {
    v(&make_spec_json())
}

fn op<'a>(spec: &'a Value, path: &str, verb: &str) -> &'a Value {
    spec.get("paths")
        .and_then(|p| p.get(path))
        .and_then(|i| i.get(verb))
        .expect("operation")
}

/// `toMatchObject`: every expected key matches, recursively.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => e
            .iter()
            .all(|(k, ev)| a.get(k).is_some_and(|av| matches_object(av, ev))),
        (Value::Array(a), Value::Array(e)) => a.len() == e.len() && a.iter().zip(e).all(|(x, y)| matches_object(x, y)),
        _ => actual == expected,
    }
}

fn codes(ds: &[Diagnostic]) -> Vec<(&str, String, &str)> {
    ds.iter()
        .map(|d| (d.code.as_str(), d.severity.to_string(), d.pointer.as_str()))
        .collect()
}

fn none() -> Vec<Diagnostic> {
    Vec::new()
}

mod resources_and_methods {
    use super::*;

    fn config() -> Value {
        v(&json!({"resources": {
            "users": {
                "methods": {"retrieve": "get /users/{id}", "create": {"endpoint": "post /users"}},
                "subresources": {"posts": {"methods": {"list_posts": "get /users/{id}/posts"}}}
            },
            "$client": {"methods": {"ping": "GET /users/{id}"}}
        }}))
    }

    #[test]
    fn writes_fern_and_x_ferrule_names_from_the_resource_tree() {
        let r = convert(&make_spec(), &config());
        assert_eq!(r.diagnostics, none());
        assert!(matches_object(
            op(&r.spec, "/users", "post"),
            &v(&json!({
                "x-fern-sdk-group-name": ["users"],
                "x-fern-sdk-method-name": "create",
                "x-ferrule-resource": {"path": ["users"], "order": [0]},
                "x-ferrule-method-name": "create",
                "x-ferrule-method-order": 1
            }))
        ));
        assert!(matches_object(
            op(&r.spec, "/users/{id}/posts", "get"),
            &v(&json!({
                "x-fern-sdk-group-name": ["users", "posts"],
                "x-ferrule-resource": {"path": ["users", "posts"], "order": [0, 0]},
                "x-ferrule-method-name": "list_posts",
                "x-ferrule-method-order": 0
            }))
        ));
        // `users.retrieve` and `$client.ping` share `get /users/{id}`: one alias each.
        let aliases = op(&r.spec, "/users/{id}", "get")
            .get("x-forge-aliases")
            .and_then(Value::as_array)
            .expect("x-forge-aliases");
        assert_eq!(aliases.len(), 2);
        assert!(matches_object(
            &aliases[0],
            &v(&json!({
                "x-fern-sdk-group-name": ["users"],
                "x-ferrule-resource": {"path": ["users"], "order": [0]},
                "x-ferrule-method-name": "retrieve",
                "x-ferrule-method-order": 0
            }))
        ));
        assert!(matches_object(
            &aliases[1],
            &v(&json!({
                "x-fern-sdk-group-name": ["$ferrule_client"],
                "x-ferrule-resource": {"path": ["$client"], "order": [1]},
                "x-ferrule-method-name": "ping",
                "x-ferrule-method-order": 0
            }))
        ));
    }

    #[test]
    fn leaves_operations_not_in_the_config_untouched() {
        let input = make_spec();
        let r = convert(&input, &config());
        assert_eq!(op(&r.spec, "/unconfigured", "get"), op(&input, "/unconfigured", "get"));
    }

    #[test]
    fn reports_fer1001_with_a_pointer_for_missing_operations() {
        let r = convert(
            &make_spec(),
            &v(&json!({"resources": {"a": {
                "methods": {"bad": "get /nope", "verb": "put /users", "junk": "no-space"},
                "subresources": {"b/c": {"methods": {"x": {"endpoint": "get /missing"}}}}
            }}})),
        );
        assert_eq!(
            codes(&r.diagnostics),
            [
                ("FER1001", "error".to_owned(), "/resources/a/methods/bad"),
                ("FER1001", "error".to_owned(), "/resources/a/methods/verb"),
                ("FER1001", "error".to_owned(), "/resources/a/methods/junk"),
                (
                    "FER1001",
                    "error".to_owned(),
                    "/resources/a/subresources/b~1c/methods/x"
                ),
            ]
        );
        assert!(r.diagnostics[0].message.contains("get /nope"));
        assert_ne!(r.diagnostics[0].suggestion, "");
    }

    #[test]
    fn follows_ref_path_items() {
        let r = convert(
            &make_spec(),
            &v(&json!({"resources": {"a": {"methods": {"r": "get /aliased"}}}})),
        );
        assert_eq!(
            op(&r.spec, "/users/{id}", "get").get("x-ferrule-method-name"),
            Some(&Value::from("r"))
        );
    }

    #[test]
    fn handles_a_spec_without_paths() {
        let r = convert(
            &Value::object(),
            &v(&json!({"resources": {"a": {"methods": {"r": "get /x"}}}})),
        );
        assert_eq!(r.diagnostics.len(), 1);
    }
}

mod pagination {
    use super::*;

    fn with_pagination(methods: &serde_json::Value) -> Value {
        v(&json!({"pagination": [cursor_page()], "resources": {"users": {"methods": methods}}}))
    }

    #[test]
    fn matches_list_methods_against_configured_schemes() {
        let r = convert(&make_spec(), &with_pagination(&json!({"list": "get /users"})));
        assert_eq!(r.diagnostics, none());
        assert_eq!(
            op(&r.spec, "/users", "get").get("x-ferrule-pagination"),
            Some(&expected_pagination())
        );
    }

    /// The configured schemes go on the root too, for the runtime to render `pagination.py` from
    /// when it is not copied from an adopted SDK: the class name spelled with the casings, and the
    /// fields as lists, in config order (the annotated spec's objects are written sorted).
    #[test]
    fn writes_the_configured_schemes_on_the_root() {
        let expected = v(&json!([{
            "name": "cursor_page",
            "className": "CursorPage",
            "kind": "cursor",
            "request": [
                {"name": "after", "type": "string", "purpose": "next_cursor_param"},
                {"name": "limit", "type": "integer"}
            ],
            "response": [
                {"name": "data", "type": "array", "purpose": "items"},
                {"name": "next_cursor", "type": "string", "purpose": "next_cursor_field"}
            ]
        }]));
        let r = convert(&make_spec(), &with_pagination(&json!({"list": "get /users"})));
        assert_eq!(r.spec.get("x-ferrule-page-schemes"), Some(&expected));
        let single = convert(
            &make_spec(),
            &v(&json!({"pagination": cursor_page(), "resources": {"users": {"methods": {"list": "get /users"}}}})),
        );
        assert_eq!(single.spec.get("x-ferrule-page-schemes"), Some(&expected));
        let none = convert(
            &make_spec(),
            &v(&json!({"resources": {"users": {"methods": {"list": "get /users"}}}})),
        );
        assert!(none.spec.get("x-ferrule-page-schemes").is_none());
    }

    /// The knock and ogx shapes: an implicit items array, nested fields, a field written before
    /// the items, and a custom casing in the name.
    #[test]
    fn writes_nested_fields_implicit_items_and_cased_names() {
        let config = v(&json!({
            "custom_casings": {"openai": {"pascal": "OpenAI"}},
            "pagination": [
                {
                    "name": "openai_cursor_page",
                    "type": "cursor",
                    "request": {"after": {"type": "string", "x-stainless-pagination-property": {"purpose": "next_cursor_param"}}},
                    "response": {
                        "skip_token": {"type": "string"},
                        "entries": {"type": "array", "items": {"type": "object"}},
                        "page_info": {"type": "object", "properties": {
                            "after": {"type": "string", "x-stainless-pagination-property": {"purpose": "next_cursor_field"}}
                        }}
                    }
                }
            ],
            "resources": {"users": {"methods": {"list": "get /users"}}}
        }));
        let r = convert(&make_spec(), &config);
        assert_eq!(
            r.spec.get("x-ferrule-page-schemes"),
            Some(&v(&json!([{
                "name": "openai_cursor_page",
                "className": "OpenAICursorPage",
                "kind": "cursor",
                "request": [{"name": "after", "type": "string", "purpose": "next_cursor_param"}],
                "response": [
                    {"name": "skip_token", "type": "string"},
                    {"name": "entries", "type": "array", "purpose": "items"},
                    {"name": "page_info", "type": "object", "fields": [
                        {"name": "after", "type": "string", "purpose": "next_cursor_field"}
                    ]}
                ]
            }])))
        );
    }

    #[test]
    fn matches_list_methods_and_all_of_responses() {
        let r = convert(
            &make_spec(),
            &v(&json!({
                "pagination": [{"name": "p", "type": "cursor", "response": {"data": null}}],
                "resources": {"users": {"methods": {"list_posts": "get /users/{id}/posts"}}}
            })),
        );
        assert_eq!(
            op(&r.spec, "/users/{id}/posts", "get").get("x-ferrule-pagination"),
            Some(&v(&json!({
                "scheme": "cursor",
                "page": "p",
                "paramLocation": "query",
                "continueOnEmptyItems": false,
                "request": {},
                "response": {"data": "data"}
            })))
        );
    }

    #[test]
    fn paginated_false_suppresses_matching() {
        let r = convert(
            &make_spec(),
            &with_pagination(&json!({"list": {"endpoint": "get /users", "paginated": false}})),
        );
        assert!(op(&r.spec, "/users", "get").get("x-ferrule-pagination").is_none());
    }

    #[test]
    fn does_not_match_non_list_methods_or_non_matching_operations() {
        let r = convert(
            &make_spec(),
            &with_pagination(&json!({"retrieve": "get /users", "list": "get /users/{id}", "list_x": "post /users"})),
        );
        assert_eq!(r.diagnostics, none());
        assert!(op(&r.spec, "/users/{id}", "get").get("x-ferrule-pagination").is_none());
        assert!(op(&r.spec, "/users", "post").get("x-ferrule-pagination").is_none());
    }

    #[test]
    fn paginated_true_uses_the_matching_scheme_or_reports_fer1003() {
        let ok = convert(
            &make_spec(),
            &with_pagination(&json!({"fetch": {"endpoint": "get /users", "paginated": true}})),
        );
        assert_eq!(
            op(&ok.spec, "/users", "get").get("x-ferrule-pagination"),
            Some(&expected_pagination())
        );
        let bad = convert(
            &make_spec(),
            &with_pagination(&json!({"get": {"endpoint": "get /users/{id}", "paginated": true}})),
        );
        let got: Vec<(&str, &str)> = bad
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str(), d.pointer.as_str()))
            .collect();
        assert_eq!(got, [("FER1003", "/resources/users/methods/get/paginated")]);
    }

    #[test]
    fn paginated_name_picks_a_scheme_or_reports_fer1002() {
        let ok = convert(
            &make_spec(),
            &with_pagination(&json!({"get": {"endpoint": "get /users/{id}", "paginated": "cursor_page"}})),
        );
        assert_eq!(
            op(&ok.spec, "/users/{id}", "get").get("x-ferrule-pagination"),
            Some(&expected_pagination())
        );
        let bad = convert(
            &make_spec(),
            &v(
                &json!({"resources": {"u": {"methods": {"get": {"endpoint": "get /users/{id}", "paginated": "nope"}}}}}),
            ),
        );
        let got: Vec<(&str, &str)> = bad
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str(), d.pointer.as_str()))
            .collect();
        assert_eq!(got, [("FER1002", "/resources/u/methods/get/paginated")]);
        assert!(bad.diagnostics[0].suggestion.contains("(none defined)"));
    }

    #[test]
    fn survives_cyclic_and_external_refs() {
        let mut spec = make_spec_json();
        spec["paths"]["/users"]["get"] = json!({
            "parameters": [{"$ref": "other.yaml#/p"}],
            "responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Loop"}}}}}
        });
        let r = convert(&v(&spec), &with_pagination(&json!({"list": "get /users"})));
        assert!(op(&r.spec, "/users", "get").get("x-ferrule-pagination").is_none());
    }
}

mod ferrule_config {
    use super::*;

    #[test]
    fn is_written_from_targets_python() {
        let r = convert(
            &Value::object(),
            &v(&json!({"targets": {"python": {"package_name": "acme"}}})),
        );
        assert_eq!(
            r.spec.get("x-ferrule-config"),
            Some(&v(&json!({
                "version": 1,
                "edition": "2025-10-08",
                "pythonEdition": "python.2025-10-08",
                "packageName": "acme",
                "projectName": "acme"
            })))
        );
        let named = convert(
            &Value::object(),
            &v(&json!({
                "edition": "2026-05-06",
                "targets": {"python": {"package_name": "acme", "project_name": "acme-sdk", "edition": "python.2025-11-20"}}
            })),
        );
        assert!(matches_object(
            named.spec.get("x-ferrule-config").expect("config"),
            &v(&json!({"edition": "2026-05-06", "pythonEdition": "python.2025-11-20", "projectName": "acme-sdk"}))
        ));
    }

    #[test]
    fn is_omitted_without_a_python_package_name() {
        let r = convert(&Value::object(), &v(&json!({"organization": {"name": "acme"}})));
        assert!(r.spec.get("x-ferrule-config").is_none());
    }
}

mod parse_config_forms {
    use super::*;

    #[test]
    fn accepts_the_method_forms_models_shared_and_unknown_keys() {
        let config = parse_config(
            "
resources:
  a:
    methods:
      list: get /a
      create: { endpoint: post /a, paginated: false, type: http }
    models:
      thing: Thing
    subresources:
      b:
        methods:
          list: { endpoint: get /a/b, paginated: page }
  $shared:
    models: { ref: Reference }
pagination:
  - name: page
    type: offset
    request: { offset: { type: integer } }
    response: { items: { type: array } }
client_settings: { anything: true }
",
        )
        .expect("config");
        assert!(matches_object(
            &config,
            &v(
                &json!({"resources": {"a": {"subresources": {"b": {"methods": {"list": {"endpoint": "get /a/b", "paginated": "page"}}}}}}})
            )
        ));
        assert!(config.get("client_settings").is_some());
    }

    #[test]
    fn rejects_malformed_methods_and_pagination() {
        assert!(parse_config("resources: { a: { methods: { x: 3 } } }").is_err());
        assert!(parse_config("pagination: [{ name: p }]").is_err());
    }
}

/// The smallest config the public schema accepts.
const VALID_CONFIG: &str = "organization: { name: acme }
environments: { production: https://api.acme.test }
resources: {}
readme:
  example_requests:
    default: { type: request, endpoint: get /x, params: {} }
    headline: { type: request, endpoint: get /x, params: {} }
";

mod parse_spec_and_convert_text {
    use super::*;

    #[test]
    fn returns_only_diagnostics_for_a_config_that_fails_validation() {
        let r = convert_text("{}", "resources: {}").expect("convert");
        assert_eq!(r.output, "");
        assert_ne!(r.diagnostics, Vec::new());
        assert!(r.diagnostics.iter().all(|d| d.code.starts_with("FER11")));
    }

    #[test]
    fn applies_openapi_transforms_before_annotating_and_reports_edition_errors() {
        let config = format!(
            "{VALID_CONFIG}edition: 2099-01-01
openapi:
  transforms:
    - command: update
      reason: rename
      args: {{ target: '$.paths[\"/x\"].get', value: {{ summary: hi }} }}
"
        );
        let r = convert_text("{\"paths\":{\"/x\":{\"get\":{\"responses\":{}}}}}", &config).expect("convert");
        assert!(r.diagnostics.iter().any(|d| d.code == "FER1101"));
        let out = parse_json(&r.output).expect("output");
        assert_eq!(op(&out, "/x", "get").get("summary"), Some(&Value::from("hi")));
    }

    #[test]
    fn parses_json_and_yaml_objects_and_rejects_others() {
        assert_eq!(
            parse_spec("{\"openapi\":\"3.1.0\"}"),
            Ok(v(&json!({"openapi": "3.1.0"})))
        );
        assert_eq!(parse_spec("openapi: 3.1.0"), Ok(v(&json!({"openapi": "3.1.0"}))));
        assert!(parse_spec("- 1").is_err());
        assert!(parse_spec("").is_err());
    }

    #[test]
    fn emits_sorted_indented_json_with_a_trailing_newline() {
        let r = convert_text("{\"b\":1,\"a\":{\"d\":[{\"z\":1,\"y\":2}],\"c\":null}}", VALID_CONFIG).expect("convert");
        assert_eq!(
            r.output,
            "{\n  \"a\": {\n    \"c\": null,\n    \"d\": [\n      {\n        \"y\": 2,\n        \"z\": 1\n      }\n    ]\n  },\n  \"b\": 1\n}\n"
        );
    }

    /// Reduced from knock's `ms_teams_pagination`: Stainless declares a page's fields in the
    /// order stainless.yml lists them (knock-go `MsTeamsPagination` has `SkipToken`, then
    /// `MsTeamsTeams`), so the field maps keep config order in the written output.
    #[test]
    fn writes_pagination_field_maps_in_config_order() {
        let spec = r#"{"paths": {"/teams": {"get": {
            "parameters": [
                {"name": "top", "in": "query", "schema": {"type": "integer"}},
                {"name": "skiptoken", "in": "query", "schema": {"type": "string"}}
            ],
            "responses": {"200": {"content": {"application/json": {"schema": {"properties": {
                "ms_teams_teams": {"type": "array", "items": {}},
                "skip_token": {"type": "string"}
            }}}}}}
        }}}}"#;
        let config = VALID_CONFIG.replace(
            "resources: {}",
            "resources:
  teams:
    methods:
      list: get /teams
pagination:
  - name: ms_teams_pagination
    type: cursor
    request:
      top: { type: integer }
      skiptoken: { type: string, x-stainless-pagination-property: { purpose: next_cursor_param } }
    response:
      skip_token: { type: string, x-stainless-pagination-property: { purpose: next_cursor_field } }
      ms_teams_teams: { type: array, items: { type: object } }
      total: { type: integer, x-stainless-pagination-property: { purpose: total_count, from_header: X-Total } }
      next: { type: string, x-stainless-pagination-property: { purpose: next_cursor_url, from_header: Link } }",
        );
        let r = convert_text(spec, &config).expect("convert");
        assert_eq!(r.diagnostics, none());
        let out = parse_json(&r.output).expect("output");
        let page = op(&out, "/teams", "get")
            .get("x-ferrule-pagination")
            .expect("paginated");
        let keys = |field: &str| -> Vec<String> {
            page.get(field)
                .and_then(Value::as_object)
                .expect("field map")
                .keys()
                .cloned()
                .collect()
        };
        assert_eq!(keys("request"), ["top", "skiptoken"]);
        assert_eq!(keys("response"), ["skip_token", "ms_teams_teams"]);
        assert_eq!(keys("headers"), ["X-Total", "Link"]);
    }
}

fn config_strategy() -> impl Strategy<Value = serde_json::Value> {
    let method = prop_oneof![
        prop::sample::select(vec!["get /users", "post /users", "get /users/{id}", "get /nope"]).prop_map(|s| json!(s)),
        (
            prop::sample::select(vec!["get /users", "get /users/{id}/posts"]),
            prop_oneof![any::<bool>().prop_map(|b| json!(b)), Just(json!("cursor_page"))]
        )
            .prop_map(|(e, p)| json!({"endpoint": e, "paginated": p})),
    ];
    let methods = prop::collection::btree_map(".{0,6}", method, 0..4);
    let resources = prop::collection::btree_map(".{0,6}", methods.prop_map(|m| json!({"methods": m})), 0..4);
    resources.prop_map(|r| json!({"resources": r, "pagination": [cursor_page()]}))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn never_mutates_its_inputs(config in config_strategy()) {
        let spec = make_spec();
        let config = v(&config);
        let (spec_before, config_before) = (spec.clone(), config.clone());
        let _ = convert(&spec, &config);
        prop_assert_eq!(spec, spec_before);
        prop_assert_eq!(config, config_before);
    }

    #[test]
    fn is_deterministic(config in config_strategy()) {
        let text = to_json(&make_spec());
        let yaml = config.to_string();
        prop_assert_eq!(convert_text(&text, &yaml), convert_text(&text, &yaml));
    }
}

mod client_order_streaming {
    use super::*;

    #[test]
    fn records_stainless_yml_order_of_resources_and_methods() {
        let r = convert(
            &make_spec(),
            &v(&json!({"resources": {
                "z": {"methods": {"second": "get /users/{id}/posts", "first": "post /users"}},
                "a": {"subresources": {"x": {}, "y": {"methods": {"only": "get /users"}}}}
            }})),
        );
        assert!(matches_object(
            op(&r.spec, "/users", "post"),
            &v(&json!({"x-ferrule-resource": {"path": ["z"], "order": [0]}, "x-ferrule-method-order": 1}))
        ));
        assert!(matches_object(
            op(&r.spec, "/users/{id}/posts", "get"),
            &v(&json!({"x-ferrule-method-order": 0}))
        ));
        assert!(matches_object(
            op(&r.spec, "/users", "get"),
            &v(&json!({
                "x-fern-sdk-group-name": ["a", "y"],
                "x-ferrule-resource": {"path": ["a", "y"], "order": [1, 1]},
                "x-ferrule-method-order": 0
            }))
        ));
    }

    #[test]
    fn writes_x_ferrule_client_at_the_root_and_on_every_operation_for_a_python_target() {
        let r = convert(
            &make_spec(),
            &v(&json!({
                "organization": {"name": "acme"},
                "targets": {"python": {"package_name": "acme"}},
                "client_settings": {"idempotency": {"header": "Idempotency-Key"}},
                "resources": {"users": {"methods": {"create": "post /users", "retrieve": "get /users/{id}"}}}
            })),
        );
        assert_eq!(r.diagnostics, none());
        let root = r.spec.get("x-ferrule-client").expect("client");
        assert!(matches_object(
            root,
            &v(&json!({"name": "acme", "envPrefix": "ACME", "idempotencyHeader": "Idempotency-Key"}))
        ));
        assert_eq!(op(&r.spec, "/users", "post").get("x-ferrule-client"), Some(root));
        assert_eq!(op(&r.spec, "/users/{id}", "get").get("x-ferrule-client"), Some(root));
        let none_spec = convert(
            &make_spec(),
            &v(&json!({"resources": {"users": {"methods": {"create": "post /users"}}}})),
        );
        assert!(none_spec.spec.get("x-ferrule-client").is_none());
        assert!(op(&none_spec.spec, "/users", "post").get("x-ferrule-client").is_none());
    }

    #[test]
    fn writes_x_ferrule_client_for_a_config_without_a_python_target() {
        // imagekit's csharp-branch config has only `targets.csharp`: the neutral model still
        // needs the client settings and the custom casings, but there is no Python package.
        let r = convert(
            &make_spec(),
            &v(&json!({
                "organization": {"name": "acme"},
                "targets": {"csharp": {"package_name": "Acme", "production_repo": "acme/acme-csharp"}},
                "client_settings": {"idempotency": {"header": "Idempotency-Key"}},
                "custom_casings": {"acme": {"pascal": "ACME"}},
                "resources": {"users": {"methods": {"create": "post /users", "retrieve": "get /users/{id}"}}}
            })),
        );
        assert_eq!(r.diagnostics, none());
        let root = r.spec.get("x-ferrule-client").expect("client");
        assert!(matches_object(
            root,
            &v(&json!({"name": "acme", "envPrefix": "ACME", "idempotencyHeader": "Idempotency-Key"}))
        ));
        let post = op(&r.spec, "/users", "post");
        assert_eq!(post.get("x-ferrule-client"), Some(root));
        assert!(post.get("x-ferrule-casings").is_some());
        assert!(post.get("x-ferrule-targets").is_some());
        // Python's settings stay Python's.
        assert!(post.get("x-ferrule-package").is_none());
        assert!(r.spec.get("x-ferrule-config").is_none());
    }

    #[test]
    fn passes_client_diagnostics_through() {
        let r = convert(
            &make_spec(),
            &v(
                &json!({"targets": {"python": {"package_name": "acme"}}, "client_settings": {"default_timeout": "never"}}),
            ),
        );
        let got: Vec<&str> = r.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(got, ["FER1305"]);
    }

    #[test]
    fn writes_x_ferrule_streaming_with_the_top_level_on_event_handlers() {
        let r = convert(
            &make_spec(),
            &v(&json!({
                "streaming": {"on_event": [{"data_starts_with": "[DONE]", "handle": "done"}]},
                "resources": {"users": {"methods": {
                    "create": {"endpoint": "post /users", "streaming": {"param_discriminator": null}},
                    "retrieve": {"endpoint": "get /users/{id}", "streaming": {"type": "jsonl"}}
                }}}
            })),
        );
        assert_eq!(
            op(&r.spec, "/users", "post").get("x-ferrule-streaming"),
            Some(&v(&json!({
                "format": "sse",
                "paramDiscriminator": null,
                "synthesizeEventAndData": false,
                "onEvent": [{"kind": "data", "handle": "done", "dataStartsWith": "[DONE]"}]
            })))
        );
        assert!(matches_object(
            op(&r.spec, "/users/{id}", "get")
                .get("x-ferrule-streaming")
                .expect("streaming"),
            &v(&json!({"format": "jsonl"}))
        ));
        let got: Vec<(&str, &str)> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str(), d.pointer.as_str()))
            .collect();
        assert_eq!(got, [("FER1401", "/resources/users/methods/retrieve/streaming")]);
    }

    #[test]
    fn reports_fer1006_for_a_resource_sharing_the_client_forge_group() {
        let r = convert(
            &make_spec(),
            &v(&json!({"resources": {"ferrule_client": {}, "$client": {"methods": {"ping": "get /users"}}}})),
        );
        assert_eq!(
            codes(&r.diagnostics),
            [("FER1006", "error".to_owned(), "/resources/ferrule_client")]
        );
    }

    #[test]
    fn reports_pagination_config_errors_and_names_the_defined_schemes() {
        let r = convert(
            &make_spec(),
            &v(&json!({
                "pagination": [cursor_page(), {"name": "bad", "type": "token", "request": {}, "response": {}}],
                "resources": {"u": {"methods": {"get": {"endpoint": "get /users/{id}", "paginated": "nope"}}}}
            })),
        );
        let got: Vec<(&str, &str)> = r
            .diagnostics
            .iter()
            .map(|d| (d.code.as_str(), d.pointer.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("FER1501", "/pagination/1/type"),
                ("FER1002", "/resources/u/methods/get/paginated")
            ]
        );
        assert!(r.diagnostics[1].suggestion.contains("cursor_page"));
    }
}

fn models_spec() -> Value {
    v(&json!({
        "openapi": "3.1.0",
        "paths": {"/a": {"get": {"responses": {}}}, "/b": {"get": {"responses": {}}}},
        "components": {"schemas": {"Ref": {"type": "object"}, "Thing": {"type": "object"}}}
    }))
}

fn ext<'a>(out: &'a Value, path: &str, key: &str) -> Option<&'a Value> {
    out.get("paths")
        .and_then(|p| p.get(path))
        .and_then(|i| i.get("get"))
        .and_then(|o| o.get(key))
}

#[test]
fn mirrors_models_and_package_onto_every_annotated_operation() {
    let config = parse_config(
        "
targets: { python: { package_name: acme, production_repo: acme/acme-python } }
resources:
  $shared: { models: { references: Ref } }
  things:
    models: { thing: \"#/components/schemas/Thing\" }
    methods: { list: get /a }
    subresources:
      parts: { models: { part_ref: Ref }, methods: { retrieve: get /b } }
",
    )
    .expect("config");
    let r = convert(&models_spec(), &config);
    assert_eq!(r.diagnostics, none());
    let models = v(&json!({
        "Ref": {"name": "PartRef", "module": "things.part_ref"},
        "Thing": {"name": "Thing", "module": "thing"}
    }));
    for path in ["/a", "/b"] {
        assert_eq!(
            ext(&r.spec, path, "x-ferrule-package"),
            Some(&v(&json!({"name": "acme", "repo": "acme/acme-python"})))
        );
        assert_eq!(ext(&r.spec, path, "x-ferrule-models"), Some(&models));
    }
}

#[test]
fn puts_shared_models_in_shared_and_omits_the_package_without_a_python_target() {
    let config =
        parse_config("resources: { $shared: { models: { references: Ref } }, a: { methods: { list: get /a } } }")
            .expect("config");
    let r = convert(&models_spec(), &config);
    assert_eq!(
        ext(&r.spec, "/a", "x-ferrule-models"),
        Some(&v(
            &json!({"Ref": {"name": "References", "module": "shared.references"}})
        ))
    );
    assert!(ext(&r.spec, "/a", "x-ferrule-package").is_none());
}

#[test]
fn defaults_repo_to_empty_and_warns_on_the_object_model_form() {
    let config = parse_config(
        "
targets: { python: { package_name: acme } }
resources: { a: { models: { thing: { type: { $ref: \"#/components/schemas/Thing\" } } }, methods: { list: get /a } } }
",
    )
    .expect("config");
    let r = convert(&models_spec(), &config);
    assert_eq!(
        ext(&r.spec, "/a", "x-ferrule-package"),
        Some(&v(&json!({"name": "acme", "repo": ""})))
    );
    assert_eq!(
        codes(&r.diagnostics),
        [("FER1004", "warning".to_owned(), "/resources/a/models/thing")]
    );
}

/// imagekit `$shared.models`: `Transformation: { openapi_uri: '#/components/schemas/Transformation' }`
/// is a model (Stainless writes `types/shared/transformation.py`); `only:` or `skip:` lists give
/// the targets that generate it (`x-ferrule-targets-only`). One no target generates is left out,
/// and one leaving Python out never replaces a Python model of the same schema.
#[test]
fn reads_the_object_model_form_and_honours_only_and_skip() {
    let config = parse_config(
        "
resources:
  $shared:
    models:
      transformation: { openapi_uri: \"#/components/schemas/Thing\" }
      references: { type: model, openapi_uri: Ref, only: [python, go] }
      ai_tag: { openapi_uri: \"#/components/schemas/Thing\", only: [go, ruby, java] }
      skipped: { openapi_uri: Ref, skip: [python] }
      skipped_all: { openapi_uri: Ref, skip: true }
  a: { methods: { list: get /a } }
",
    )
    .expect("config");
    let r = convert(&models_spec(), &config);
    assert_eq!(r.diagnostics, none());
    assert_eq!(
        ext(&r.spec, "/a", "x-ferrule-models"),
        Some(&v(&json!({
            "Thing": {"name": "Transformation", "module": "shared.transformation"},
            "Ref": {"name": "References", "module": "shared.references", "x-ferrule-targets-only": ["python", "go"]}
        })))
    );
}

/// `openapi_uri` into a sub-schema (`#/components/schemas/Extensions/items`) or a query, and
/// `type: alias`, are not read yet: FER1004 at the entry, when some target generates it.
#[test]
fn warns_on_object_models_it_cannot_read() {
    let config = parse_config(
        "
resources:
  a:
    models:
      item: { openapi_uri: \"#/components/schemas/Thing/items\" }
      old: { type: alias, to: thing }
      java_only: { openapi_uri: \"#/components/schemas/Thing/items\", only: [java] }
    methods: { list: get /a }
",
    )
    .expect("config");
    let r = convert(&models_spec(), &config);
    assert_eq!(
        codes(&r.diagnostics),
        [
            ("FER1004", "warning".to_owned(), "/resources/a/models/item"),
            ("FER1004", "warning".to_owned(), "/resources/a/models/old"),
            ("FER1004", "warning".to_owned(), "/resources/a/models/java_only"),
        ]
    );
    assert_eq!(ext(&r.spec, "/a", "x-ferrule-models"), Some(&v(&json!({}))));
}

#[test]
fn names_every_schema_models_points_at_with_the_last_name_winning() {
    let config = parse_config(
        "
resources:
  $shared: { models: { references: Ref } }
  a: { models: { thing: Thing, other_ref: Ref }, methods: { list: get /a } }
",
    )
    .expect("config");
    let spec = models_spec();
    let r = convert(&spec, &config);
    assert_eq!(r.diagnostics, none());
    let schemas = r
        .spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .expect("schemas");
    assert!(matches_object(
        schemas,
        &v(&json!({"Thing": {"x-fern-type-name": "Thing"}, "Ref": {"x-fern-type-name": "OtherRef"}}))
    ));
    let original = spec
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.get("Ref"))
        .expect("Ref");
    assert!(original.get("x-fern-type-name").is_none());
}

/// lumaai binds `post /generations/video` to `generations.create` and
/// `generations.video.create`, and Stainless emits both. Forge makes one method per operation
/// unless the operation lists `x-forge-aliases`, one definition per method, each with its own
/// extensions (an alias does not inherit the operation's).
#[test]
fn writes_an_endpoint_bound_to_two_methods_as_forge_aliases() {
    let config = parse_config(
        "
targets: { python: { package_name: acme, production_repo: acme/acme-python } }
resources:
  things:
    models: { thing: Thing }
    methods: { list: get /a }
    subresources:
      parts: { methods: { retrieve: get /b, list: get /a } }
",
    )
    .expect("config");
    let r = convert(&models_spec(), &config);
    let op = r
        .spec
        .get("paths")
        .and_then(|p| p.get("/a"))
        .and_then(|i| i.get("get"))
        .expect("get /a");
    for own in [
        "x-fern-sdk-group-name",
        "x-fern-sdk-method-name",
        "x-ferrule-resource",
        "x-ferrule-method-name",
        "x-ferrule-models",
    ] {
        assert!(op.get(own).is_none(), "{own} on the operation");
    }
    let Some(Value::Array(aliases)) = op.get("x-forge-aliases") else {
        panic!("no x-forge-aliases on get /a");
    };
    let models = v(&json!({"Thing": {"name": "Thing", "module": "thing"}}));
    let package = v(&json!({"name": "acme", "repo": "acme/acme-python"}));
    let expected = [
        (json!(["things"]), json!({"path": ["things"], "order": [0]}), 0.0),
        (
            json!(["things", "parts"]),
            json!({"path": ["things", "parts"], "order": [0, 0]}),
            1.0,
        ),
    ];
    assert_eq!(aliases.len(), expected.len());
    for (alias, (group, resource, order)) in aliases.iter().zip(expected) {
        assert_eq!(alias.get("x-fern-sdk-group-name"), Some(&v(&group)));
        assert_eq!(alias.get("x-fern-sdk-method-name"), Some(&v(&json!("list"))));
        assert_eq!(alias.get("x-ferrule-resource"), Some(&v(&resource)));
        assert_eq!(alias.get("x-ferrule-method-name"), Some(&v(&json!("list"))));
        assert_eq!(alias.get("x-ferrule-method-order"), Some(&Value::Number(order)));
        assert_eq!(alias.get("x-ferrule-models"), Some(&models));
        assert_eq!(alias.get("x-ferrule-package"), Some(&package));
        assert!(alias.get("x-ferrule-client").is_some());
    }
    // An operation bound once keeps its extensions on the operation itself.
    assert!(ext(&r.spec, "/b", "x-forge-aliases").is_none());
    assert_eq!(
        ext(&r.spec, "/b", "x-ferrule-method-name"),
        Some(&v(&json!("retrieve")))
    );
}

#[test]
fn reports_fer1005_for_a_model_whose_schema_is_not_in_the_spec() {
    let config = parse_config("resources: { a: { models: { ghost: Ghost } } }").expect("config");
    let spec = models_spec();
    let r = convert(&spec, &config);
    assert_eq!(
        codes(&r.diagnostics),
        [("FER1005", "warning".to_owned(), "/resources/a/models/ghost")]
    );
    assert_eq!(r.spec, spec);
    assert!(r.diagnostics.iter().all(|d| d.severity == Severity::Warning));
}
