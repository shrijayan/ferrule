use ferrule_converter::extensions::client::{build_client, timeout_seconds, upper_snake};
use ferrule_converter::extensions::groups::{CLIENT_GROUP, collides_with_client_group, forge_group_path, forge_words};
use ferrule_converter::extensions::json::{
    deref, is_array_schema, items_schema, property_names, property_schema, success_response,
};
use ferrule_converter::extensions::pagination::{
    PAGINATION_SCHEMES, Scheme, best_match, matches, operation_shape, read_schemes,
};
use ferrule_converter::extensions::streaming::{StreamingInput, build_streaming, read_on_event, resolve_event_model};
use ferrule_converter::{Diagnostic, Map, Value};
use proptest::prelude::*;
use serde_json::json;

// ---------------------------------------------------------------------------------------------
// Shared helpers

fn v(j: serde_json::Value) -> Value {
    Value::from(j)
}

fn obj(value: &Value) -> &Map {
    value.as_object().expect("an object")
}

/// `[code, severity, pointer]` of each diagnostic.
fn csp(ds: &[Diagnostic]) -> Vec<[String; 3]> {
    ds.iter()
        .map(|d| [d.code.clone(), d.severity.to_string(), d.pointer.clone()])
        .collect()
}

/// `[code, pointer]` of each diagnostic.
fn cp(ds: &[Diagnostic]) -> Vec<[String; 2]> {
    ds.iter().map(|d| [d.code.clone(), d.pointer.clone()]).collect()
}

/// `toMatchObject`: objects match on the expected keys only, recursively; arrays element-wise.
fn match_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => e.iter().all(|(k, ev)| a.get(k).is_some_and(|av| match_object(av, ev))),
        (Value::Array(a), Value::Array(e)) => a.len() == e.len() && a.iter().zip(e).all(|(x, y)| match_object(x, y)),
        _ => actual == expected,
    }
}

fn assert_match(actual: &Value, expected: serde_json::Value) {
    let expected = v(expected);
    assert!(
        match_object(actual, &expected),
        "expected {actual:?} to match {expected:?}"
    );
}

fn client_spec() -> Value {
    v(json!({
        "servers": [{"url": "https://spec.example.com"}],
        "security": [{"SpecKey": []}],
        "components": {
            "securitySchemes": {
                "SpecKey": {"type": "apiKey", "in": "query", "name": "key"},
                "Bearer": {"type": "http", "scheme": "Bearer"},
                "Broken": {"type": "mutualTLS"}
            }
        }
    }))
}

fn full_client_config() -> Value {
    v(json!({
        "organization": {"name": "acme-corp"},
        "environments": {"production": "https://api.acme.com", "sandbox": "https://sandbox.acme.com"},
        "security": [{"Bearer": []}, {"Basic": ["read"]}],
        "security_schemes": {
            "Basic": {"type": "http", "scheme": "basic", "x-stainless-auth": {"disable_base64": true}},
            "Bearer": {"type": "http", "scheme": "bearer", "x-stainless-auth": {"scheme_keyword": "Token"}},
            "OAuth": {"type": "oauth2", "flows": {"clientCredentials": {"tokenUrl": "/t", "scopes": {}}}},
            "Oidc": {"type": "openIdConnect", "openIdConnectUrl": "https://id"},
            "Digest": {"type": "http", "scheme": "digest"},
            "Cookie": {"type": "apiKey", "in": "cookie", "name": "sid"}
        },
        "client_settings": {
            "default_client_name": "Acme",
            "default_env_prefix": "ACME",
            "opts": {
                "api_key": {
                    "type": "string",
                    "read_env": "ACME_API_KEY",
                    "auth": {"security_scheme": "Bearer"},
                    "description": "The key"
                },
                "region": {"type": "string", "nullable": true, "default": "us", "server_variable": "region"},
                "org_id": {"type": "integer", "send_in_header": "X-Org", "send_as_query_param": "org"},
                "flag": {"type": "boolean", "auth": {"security_scheme": "Basic", "role": "username"}}
            },
            "idempotency": {"header": "Idempotency-Key"},
            "default_headers": {"X-Version": "2", "bad": 3},
            "omit_stainless_headers": true,
            "default_timeout": "PT1M30S",
            "default_retries": {"max_retries": 5, "initial_delay_seconds": 1, "max_delay_seconds": 30},
            "python": {"default_http_limits": {"max_connections": 10, "keepalive_expiry": 5}}
        },
        "query_settings": {"nested_format": "dots", "array_format": "indices"}
    }))
}

fn full_client_expected() -> Value {
    v(json!({
        "name": "Acme",
        "envPrefix": "ACME",
        "opts": [
            {
                "name": "api_key",
                "type": "string",
                "nullable": false,
                "readEnv": "ACME_API_KEY",
                "description": "The key",
                "auth": {"securityScheme": "Bearer", "role": "value"}
            },
            {"name": "region", "type": "string", "nullable": true, "default": "us", "serverVariable": "region"},
            {"name": "org_id", "type": "integer", "nullable": false, "send": {"in": "header", "name": "X-Org"}},
            {
                "name": "flag",
                "type": "boolean",
                "nullable": false,
                "auth": {"securityScheme": "Basic", "role": "username"}
            }
        ],
        "environments": [
            {"name": "production", "url": "https://api.acme.com"},
            {"name": "sandbox", "url": "https://sandbox.acme.com"}
        ],
        "defaultEnvironment": "production",
        "securitySchemes": {
            "SpecKey": {"type": "apiKey", "in": "query", "name": "key"},
            "Bearer": {"type": "http", "scheme": "bearer", "keyword": "Token"},
            "Basic": {"type": "http", "scheme": "basic", "disableBase64": true},
            "OAuth": {"type": "oauth2", "flows": {"clientCredentials": {"tokenUrl": "/t", "scopes": {}}}},
            "Oidc": {"type": "openIdConnect", "url": "https://id"},
            "Digest": {"type": "http", "scheme": "digest"},
            "Cookie": {"type": "apiKey", "in": "cookie", "name": "sid"}
        },
        "security": [{"Bearer": []}, {"Basic": ["read"]}],
        "timeoutSeconds": 90,
        "retries": {"maxRetries": 5, "initialDelaySeconds": 1, "maxDelaySeconds": 30},
        "idempotencyHeader": "Idempotency-Key",
        "defaultHeaders": {"X-Version": "2"},
        "omitPlatformHeaders": true,
        "query": {"nestedFormat": "dots", "arrayFormat": "indices"},
        "httpLimits": {"maxConnections": 10, "keepaliveExpiry": 5}
    }))
}

#[test]
fn reads_every_client_settings_field() {
    let (client, diagnostics) = build_client(&client_spec(), &full_client_config());
    assert_eq!(client, full_client_expected());
    assert_eq!(
        csp(&diagnostics),
        [
            ["FER1302", "warning", "/client_settings/opts/org_id"],
            ["FER1303", "warning", "/components/securitySchemes/Broken"],
        ]
    );
}

#[test]
fn defaults_to_stainless_defaults_the_spec_server_and_the_spec_security() {
    let (client, diagnostics) = build_client(&client_spec(), &v(json!({"organization": {"name": "OneBusAway SDK"}})));
    assert_eq!(diagnostics.len(), 1);
    assert_match(
        &client,
        json!({
            "name": "OneBusAway SDK",
            "envPrefix": "ONE_BUS_AWAY_SDK",
            "opts": [],
            "environments": [{"name": "production", "url": "https://spec.example.com"}],
            "defaultEnvironment": "production",
            "security": [{"SpecKey": []}],
            "timeoutSeconds": 60,
            "retries": {"maxRetries": 2, "initialDelaySeconds": 0.5, "maxDelaySeconds": 8},
            "defaultHeaders": {},
            "omitPlatformHeaders": false,
            "query": {"nestedFormat": "brackets", "arrayFormat": "comma"}
        }),
    );
    assert_eq!(client.get("idempotencyHeader"), None);
    assert_eq!(client.get("httpLimits"), None);
}

#[test]
fn falls_back_to_the_package_name_and_leaves_environments_empty_without_servers() {
    let (client, _) = build_client(
        &v(json!({"security": "nope", "servers": ["x"]})),
        &v(json!({
            "targets": {"python": {"package_name": "acme"}},
            "security": [{"A": "x"}, 3]
        })),
    );
    assert_match(
        &client,
        json!({"name": "acme", "envPrefix": "ACME", "environments": []}),
    );
    assert_eq!(client.get("security"), Some(&v(json!([{"A": []}]))));
    assert_eq!(client.get("defaultEnvironment"), None);
    let (empty, _) = build_client(&Value::object(), &Value::object());
    assert_eq!(empty.get("name"), Some(&Value::from("")));
}

#[test]
fn reports_bad_opts_undefined_auth_schemes_and_bad_timeouts() {
    let (client, diagnostics) = build_client(
        &Value::object(),
        &v(json!({
            "environments": {"a": 3},
            "client_settings": {
                "opts": {
                    "x": {"type": "date"},
                    "y": "nope",
                    "k": {"type": "string", "auth": {"security_scheme": "S"}}
                },
                "default_timeout": "soon",
                "python": {"default_http_limits": {}}
            },
            "query_settings": {"nested_format": "weird", "array_format": 7}
        })),
    );
    assert_eq!(
        csp(&diagnostics),
        [
            ["FER1301", "error", "/client_settings/opts/x/type"],
            ["FER1301", "error", "/client_settings/opts/y/type"],
            ["FER1304", "error", "/client_settings/opts/k/auth/security_scheme"],
            ["FER1305", "error", "/client_settings/default_timeout"],
        ]
    );
    assert!(diagnostics[2].suggestion.contains("(none defined)"));
    assert_match(
        &client,
        json!({
            "timeoutSeconds": 60,
            "environments": [],
            "httpLimits": {},
            "query": {"nestedFormat": "brackets", "arrayFormat": "comma"}
        }),
    );
}

fn secs(input: &Value) -> Option<Value> {
    timeout_seconds(input).map(Value::Number)
}

#[test]
fn timeout_seconds_converts_milliseconds_and_iso_durations() {
    let rows = [
        (json!(60000), 60.0),
        (json!(1500), 1.5),
        (json!("PT60S"), 60.0),
        (json!("PT0.5S"), 0.5),
        (json!("PT2M"), 120.0),
        (json!("PT1H"), 3600.0),
        (json!("P1D"), 86400.0),
    ];
    for (input, seconds) in rows {
        let input = v(input);
        assert_eq!(secs(&input), Some(Value::Number(seconds)), "{input:?}");
    }
}

#[test]
fn timeout_seconds_rejects() {
    let rows = [
        v(json!(0)),
        v(json!(-1)),
        Value::Number(f64::INFINITY),
        v(json!("P")),
        v(json!("PT")),
        v(json!("PT0S")),
        v(json!("60")),
        Value::Null,
        Value::Bool(true),
    ];
    for input in &rows {
        assert_eq!(secs(input), None, "{input:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn round_trips_whole_seconds_written_as_pt_n_s(n in 1u32..=10_000) {
        let n = f64::from(n);
        prop_assert_eq!(secs(&Value::from(format!("PT{n}S"))), Some(Value::Number(n)));
        prop_assert_eq!(secs(&Value::Number(n * 1000.0)), Some(Value::Number(n)));
    }

    #[test]
    fn always_yields_an_upper_case_identifier_or_nothing(s in any::<String>()) {
        let out = upper_snake(&s);
        let ok = out.is_empty()
            || out.split('_').all(|w| {
                !w.is_empty() && w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            });
        prop_assert!(ok, "{:?} -> {:?}", s, out);
    }
}

#[test]
fn upper_snake_examples() {
    let rows = [
        ("onebusaway-sdk", "ONEBUSAWAY_SDK"),
        ("AcmeCorp", "ACME_CORP"),
        ("  my  api ", "MY_API"),
    ];
    for (input, out) in rows {
        assert_eq!(upper_snake(input), out, "{input:?}");
    }
}

const STREAMING_POINTER: &str = "/resources/chat/methods/create";

fn streaming_spec() -> Value {
    v(json!({
        "components": {
            "requestBodies": {
                "Chat": {
                    "content": {
                        "application/json": {
                            "schema": {"type": "object", "properties": {"stream": {"type": "boolean"}}}
                        }
                    }
                }
            }
        }
    }))
}

fn streaming_config() -> Value {
    v(json!({
        "resources": {
            "chat": {
                "subresources": {
                    "completions": {"models": {"chunk": "#/components/schemas/Chunk", "other": {"x": 1}}}
                }
            },
            "events": {"models": {"event": "Event"}}
        }
    }))
}

fn sse() -> serde_json::Value {
    json!({"200": {"content": {"text/event-stream": {}, "application/json": {}}}})
}

fn jsonl() -> serde_json::Value {
    json!({"200": {"content": {"application/x-ndjson": {}}}})
}

fn stream_with_op(streaming: Option<&Value>, op: &Value) -> (Option<Value>, Vec<Diagnostic>) {
    let spec = streaming_spec();
    let config = streaming_config();
    build_streaming(&StreamingInput {
        spec: &spec,
        config: &config,
        op: obj(op),
        streaming,
        pointer: STREAMING_POINTER,
        on_event: &[],
    })
}

/// `buildStreaming(input(streaming, responses))`.
fn stream(streaming: Option<serde_json::Value>, responses: serde_json::Value) -> (Option<Value>, Vec<Diagnostic>) {
    let mut op = json!({"requestBody": {"$ref": "#/components/requestBodies/Chat"}});
    op["responses"] = responses;
    let op = v(op);
    let streaming = streaming.map(Value::from);
    stream_with_op(streaming.as_ref(), &op)
}

fn format_of(result: &(Option<Value>, Vec<Diagnostic>)) -> Option<&str> {
    result.0.as_ref().and_then(|s| s.get("format")).and_then(Value::as_str)
}

#[test]
fn reads_a_method_streaming_block() {
    let (streaming, diagnostics) = stream(
        Some(json!({
            "param_discriminator": "stream",
            "stream_event_model": "chat.completions.chunk",
            "params_type_name": "completion_create_params",
            "method_name": "stream",
            "synthesize_event_and_data_properties": true
        })),
        json!({}),
    );
    assert_eq!(diagnostics, Vec::<Diagnostic>::new());
    assert_eq!(
        streaming,
        Some(v(json!({
            "format": "sse",
            "paramDiscriminator": "stream",
            "eventModel": "chat.completions.chunk",
            "eventSchema": "Chunk",
            "paramsTypeName": "completion_create_params",
            "methodName": "stream",
            "synthesizeEventAndData": true,
            "onEvent": []
        })))
    );
}

#[test]
fn infers_the_format_from_the_response_content_type_unless_written() {
    let always = json!({"param_discriminator": null});
    assert_eq!(format_of(&stream(Some(always.clone()), sse())), Some("sse"));
    assert_eq!(format_of(&stream(Some(always.clone()), jsonl())), Some("jsonl"));
    assert_eq!(
        format_of(&stream(
            Some(json!({"param_discriminator": null, "type": "jsonl"})),
            sse()
        )),
        Some("jsonl")
    );
    assert_eq!(
        format_of(&stream(
            Some(json!({"param_discriminator": null, "type": "sse"})),
            jsonl()
        )),
        Some("sse")
    );
    let (streaming, diagnostics) = stream(Some(always), json!({}));
    assert_match(
        &streaming.expect("streaming"),
        json!({"format": "sse", "paramDiscriminator": null}),
    );
    assert_eq!(diagnostics, Vec::<Diagnostic>::new());
}

#[test]
fn is_absent_when_not_configured_or_skipped_for_python() {
    assert_eq!(stream(None, json!({})).0, None);
    assert_eq!(stream(Some(json!(true)), json!({})).0, None);
    assert_eq!(
        stream(
            Some(json!({"param_discriminator": null, "skip": ["python"]})),
            json!({})
        )
        .0,
        None
    );
    assert!(
        stream(Some(json!({"param_discriminator": null, "skip": ["go"]})), json!({}))
            .0
            .is_some()
    );
}

#[test]
fn warns_about_a_missing_or_unknown_discriminator_and_an_unresolved_event_model() {
    let missing = stream(Some(json!({"stream_event_model": "chat.nope"})), json!({}));
    assert_eq!(
        missing.0.as_ref().and_then(|s| s.get("paramDiscriminator")),
        Some(&Value::Null)
    );
    assert_eq!(
        cp(&missing.1),
        [
            ["FER1401", "/resources/chat/methods/create/streaming"],
            ["FER1403", "/resources/chat/methods/create/streaming/stream_event_model"],
        ]
    );
    let streaming = v(json!({"param_discriminator": "go"}));
    let unknown = stream_with_op(Some(&streaming), &v(json!({"requestBody": "bad"})));
    assert_eq!(
        csp(&unknown.1),
        [[
            "FER1402",
            "warning",
            "/resources/chat/methods/create/streaming/param_discriminator"
        ]]
    );
}

#[test]
fn resolve_event_model_rows() {
    let config = streaming_config();
    let rows = [
        ("chat.completions.chunk", Some("Chunk")),
        ("events.event", Some("Event")),
        ("chat.completions.other", None),
        ("chat.missing.chunk", None),
        ("nope.event", None),
        ("event", None),
    ];
    for (written, schema) in rows {
        assert_eq!(
            resolve_event_model(&config, written),
            schema.map(str::to_owned),
            "{written}"
        );
    }
}

#[test]
fn normalizes_kinds_and_keeps_config_order() {
    let config = v(json!({
        "streaming": {
            "on_event": [
                {"data_starts_with": "[DONE]", "handle": "done"},
                {"event_type": null, "handle": "yield"},
                {"kind": "event", "event_type": ["error", 3], "handle": "error", "error_property": "msg"},
                {"event_type": "ping", "handle": "continue"},
                {"kind": "fallthrough", "handle": "break"},
                {"kind": "data", "data_starts_with": "x", "handle": "nope"},
                "junk"
            ]
        }
    }));
    let expected = v(json!([
        {"kind": "data", "handle": "done", "dataStartsWith": "[DONE]"},
        {"kind": "event", "handle": "yield", "eventType": null},
        {"kind": "event", "handle": "error", "eventType": ["error"], "errorProperty": "msg"},
        {"kind": "event", "handle": "continue", "eventType": "ping"},
        {"kind": "fallthrough", "handle": "break"}
    ]));
    assert_eq!(Value::Array(read_on_event(&config)), expected);
    assert_eq!(read_on_event(&Value::object()), Vec::<Value>::new());
}

fn pagination_spec() -> Value {
    v(json!({
        "components": {
            "schemas": {
                "Item": {"type": "object", "properties": {"id": {"type": "string"}}},
                "Page": {
                    "allOf": [
                        {
                            "type": "object",
                            "properties": {
                                "data": {"type": "array", "items": {"$ref": "#/components/schemas/Item"}}
                            }
                        },
                        {
                            "properties": {
                                "meta": {"type": "object", "properties": {"next_cursor": {"type": "string"}}},
                                "has_more": {"type": "boolean"},
                                "total": {"type": "integer"}
                            }
                        }
                    ]
                }
            }
        }
    }))
}

/// `op(extra)`: the cursor/limit operation, with `extra` keys overriding.
fn page_op(extra: serde_json::Value) -> Value {
    let mut base = json!({
        "parameters": [{"name": "cursor", "in": "query"}, {"name": "limit", "in": "query"}],
        "responses": {
            "201": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Page"}}}}
        }
    });
    if let (Some(b), serde_json::Value::Object(e)) = (base.as_object_mut(), extra) {
        b.extend(e);
    }
    v(base)
}

fn nested_cursor() -> serde_json::Value {
    json!({
        "name": "nested",
        "type": "cursor",
        "request": {
            "cursor": {"x-stainless-pagination-property": {"purpose": "next_cursor_param"}},
            "limit": {"type": "integer"}
        },
        "response": {
            "data": {"type": "array", "x-stainless-pagination-property": {"purpose": "items"}},
            "meta": {
                "type": "object",
                "properties": {
                    "next_cursor": {
                        "type": "string",
                        "x-stainless-pagination-property": {"purpose": "next_cursor_field"}
                    }
                }
            }
        }
    })
}

fn nested_scheme() -> Scheme {
    let (schemes, _) = read_schemes(&v(json!({"pagination": [nested_cursor()]})));
    schemes.into_iter().next().expect("fixture")
}

/// `matches(spec, scheme, operationShape(spec, pathItem, op))`.
fn matches_op(scheme: &Scheme, path_item: &Value, op: &Value) -> bool {
    let spec = pagination_spec();
    matches(&spec, scheme, &operation_shape(&spec, obj(path_item), obj(op)))
}

#[test]
fn flattens_nested_response_fields_item_fields_and_header_fields() {
    let (schemes, diagnostics) = read_schemes(&v(json!({
        "pagination": [
            nested_cursor(),
            {
                "name": "ids",
                "type": "cursor_id",
                "param_location": "body",
                "continue_on_empty_items": true,
                "request": {"starting_after": {"x-stainless-pagination-property": {"purpose": "cursor_id_param"}}},
                "response": {
                    "data": {
                        "type": "array",
                        "x-stainless-pagination-property": {"purpose": "items"},
                        "items": {
                            "type": "object",
                            "properties": {"id": {"x-stainless-pagination-property": {"purpose": "cursor_item_id"}}}
                        }
                    },
                    "meta": {"type": "object", "properties": {"plain": {"type": "string"}}},
                    "next": {"x-stainless-pagination-property": {"purpose": "next_cursor_url", "from_header": "Link"}},
                    "raw": {"x-stainless-pagination-property": {"from_header": "X-Total"}}
                }
            }
        ]
    })));
    assert_eq!(diagnostics, Vec::<Diagnostic>::new());
    let extensions: Vec<Value> = schemes.iter().map(|s| Value::Object(s.extension.clone())).collect();
    assert_eq!(
        Value::Array(extensions),
        v(json!([
            {
                "scheme": "cursor",
                "page": "nested",
                "paramLocation": "query",
                "continueOnEmptyItems": false,
                "request": {"cursor": "next_cursor_param", "limit": "limit"},
                "response": {"data": "items", "meta.next_cursor": "next_cursor_field"}
            },
            {
                "scheme": "cursor_id",
                "page": "ids",
                "paramLocation": "body",
                "continueOnEmptyItems": true,
                "request": {"starting_after": "cursor_id_param"},
                "response": {"data.[].id": "cursor_item_id", "data": "items", "meta": "meta"},
                "headers": {"Link": "next_cursor_url", "X-Total": "raw"}
            }
        ]))
    );
}

#[test]
fn accepts_the_single_object_form_and_rejects_unknown_types() {
    let (single, _) = read_schemes(&v(
        json!({"pagination": {"type": "offset", "request": {}, "response": {}}}),
    ));
    assert_eq!(single.first().map(Scheme::page), Some("page"));
    let (bad, bad_diagnostics) = read_schemes(&v(json!({"pagination": [{"name": "a", "type": "token"}, "junk"]})));
    assert_eq!(bad.len(), 0);
    assert_eq!(
        cp(&bad_diagnostics),
        [["FER1501", "/pagination/0/type"], ["FER1501", "/pagination/1/type"]]
    );
    let (_, one) = read_schemes(&v(json!({"pagination": {"type": 1}})));
    assert_eq!(one.first().map(|d| d.pointer.as_str()), Some("/pagination/type"));
    assert_eq!(read_schemes(&Value::object()).0.len(), 0);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn accepts_every_scheme_the_public_schema_allows(
        ty in proptest::sample::select(PAGINATION_SCHEMES.to_vec()),
        name in any::<String>(),
    ) {
        let (schemes, _) = read_schemes(&v(
            json!({"pagination": [{"name": name, "type": ty, "request": {}, "response": {}}]}),
        ));
        let ext = schemes.first().map(|s| Value::Object(s.extension.clone()));
        let expected = v(json!({"scheme": ty, "page": name}));
        prop_assert!(
            ext.as_ref().is_some_and(|e| match_object(e, &expected)),
            "{:?} does not match {:?}",
            ext,
            expected
        );
    }
}

#[test]
fn matches_nested_response_paths_through_ref_and_all_of() {
    assert!(matches_op(&nested_scheme(), &Value::object(), &page_op(json!({}))));
}

#[test]
fn needs_every_request_param_and_every_response_path() {
    let nested = nested_scheme();
    let no_limit = page_op(json!({"parameters": [{"name": "cursor", "in": "query"}]}));
    assert!(!matches_op(&nested, &Value::object(), &no_limit));
    let flat = page_op(json!({
        "responses": {
            "200": {
                "content": {
                    "application/json": {
                        "schema": {"properties": {"data": {"type": "array"}, "next_cursor": {}}}
                    }
                }
            }
        }
    }));
    assert!(!matches_op(&nested, &Value::object(), &flat));
}

fn data_meta_op(data: serde_json::Value) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert("data".to_owned(), data);
    properties.insert("meta".to_owned(), json!({"properties": {"next_cursor": {}}}));
    page_op(json!({
        "responses": {
            "200": {
                "content": {
                    "application/json": {
                        "schema": {"properties": properties}
                    }
                }
            }
        }
    }))
}

#[test]
fn needs_the_items_field_to_be_an_array() {
    let nested = nested_scheme();
    let not_array = data_meta_op(json!({"type": "object"}));
    assert!(!matches_op(&nested, &Value::object(), &not_array));
    let type_list = data_meta_op(json!({"type": ["array", "null"]}));
    assert!(matches_op(&nested, &Value::object(), &type_list));
}

#[test]
fn reads_body_params_for_param_location_body() {
    let (schemes, _) = read_schemes(&v(json!({
        "pagination": [
            {"name": "b", "type": "offset", "param_location": "body", "request": {"offset": {}}, "response": {}}
        ]
    })));
    let body = schemes.first().expect("fixture");
    let with_body = v(json!({
        "requestBody": {"content": {"application/json": {"schema": {"properties": {"offset": {}}}}}},
        "responses": {}
    }));
    assert!(matches_op(body, &Value::object(), &with_body));
    assert!(!matches_op(
        body,
        &Value::object(),
        &v(json!({"parameters": [{"name": "offset", "in": "query"}]}))
    ));
}

#[test]
fn picks_the_most_specific_match_earliest_on_ties() {
    let mut twin = nested_cursor();
    twin["name"] = json!("twin");
    let (schemes, _) = read_schemes(&v(json!({
        "pagination": [
            {
                "name": "loose",
                "type": "cursor",
                "request": {"cursor": {}},
                "response": {"data": {"x-stainless-pagination-property": {"purpose": "items"}}}
            },
            nested_cursor(),
            twin,
            {"name": "miss", "type": "offset", "request": {"offset": {}}, "response": {}}
        ]
    })));
    let spec = pagination_spec();
    let path_item = v(json!({"parameters": [{"name": "limit", "in": "query"}]}));
    let op = page_op(json!({}));
    let shape = operation_shape(&spec, obj(&path_item), obj(&op));
    assert_eq!(best_match(&spec, &schemes, &shape).map(Scheme::page), Some("nested"));
    assert!(best_match(&spec, &schemes[3..], &shape).is_none());
}

#[test]
fn collects_path_level_and_operation_level_query_params_only() {
    let spec = pagination_spec();
    let path_item = v(json!({
        "parameters": [{"name": "a", "in": "query"}, {"name": "h", "in": "header"}]
    }));
    let op = v(json!({
        "parameters": [{"$ref": "#/nope"}, {"name": "b", "in": "query"}],
        "responses": {"default": {}}
    }));
    let shape = operation_shape(&spec, obj(&path_item), obj(&op));
    assert_eq!(shape.query, ["a", "b"]);
    assert_eq!(shape.response_schema, None);
    assert_eq!(shape.body, Vec::<String>::new());
}

fn json_spec() -> Value {
    v(json!({
        "components": {
            "schemas": {
                "Loop": {"$ref": "#/components/schemas/Loop"},
                "List": {"allOf": [{"description": "x"}, {"items": {"type": "string"}}]},
                "Deep": {"allOf": [{"allOf": [{"properties": {"a": {"type": "string"}}}]}]},
                "a/b~c": {"type": "integer"}
            }
        }
    }))
}

#[test]
fn derefs_escaped_cyclic_and_external_refs() {
    let spec = json_spec();
    let escaped = v(json!({"$ref": "#/components/schemas/a~1b~0c"}));
    assert_eq!(deref(&spec, Some(&escaped)), Some(&v(json!({"type": "integer"}))));
    let looped = v(json!({"$ref": "#/components/schemas/Loop"}));
    assert_eq!(deref(&spec, Some(&looped)), None);
    let external = v(json!({"$ref": "other.yml#/x"}));
    assert_eq!(deref(&spec, Some(&external)), None);
    let three = v(json!(3));
    assert_eq!(deref(&spec, Some(&three)), Some(&three));
}

#[test]
fn follows_all_of_for_properties_and_items() {
    let spec = json_spec();
    let deep = v(json!({"$ref": "#/components/schemas/Deep"}));
    assert_eq!(property_names(&spec, Some(&deep), 0), ["a"]);
    assert_eq!(
        property_schema(&spec, Some(&deep), "a", 0),
        Some(&v(json!({"type": "string"})))
    );
    assert_eq!(property_schema(&spec, Some(&deep), "b", 0), None);
    let list = v(json!({"$ref": "#/components/schemas/List"}));
    assert_eq!(items_schema(&spec, Some(&list), 0), Some(&v(json!({"type": "string"}))));
    assert!(is_array_schema(&spec, Some(&list), 0));
    let empty_all_of = v(json!({"allOf": [{}]}));
    assert_eq!(items_schema(&spec, Some(&empty_all_of), 0), None);
    let object = v(json!({"type": "object"}));
    assert!(!is_array_schema(&spec, Some(&object), 0));
    assert!(!is_array_schema(&spec, Some(&Value::from("x")), 0));
    assert_eq!(items_schema(&spec, Some(&Value::Null), 0), None);
}

#[test]
fn stops_at_a_depth_limit_on_self_nesting_all_of() {
    let spec = json_spec();
    let mut s = json!({"properties": {"z": {}}, "items": {}});
    for _ in 0..40 {
        s = json!({"allOf": [s]});
    }
    let s = v(s);
    assert_eq!(property_names(&spec, Some(&s), 0).len(), 0);
    assert_eq!(property_schema(&spec, Some(&s), "z", 0), None);
    assert!(!is_array_schema(&spec, Some(&s), 0));
    assert_eq!(items_schema(&spec, Some(&s), 0), None);
}

#[test]
fn picks_the_lowest_2xx_response() {
    let spec = json_spec();
    let op = v(json!({
        "responses": {"404": {}, "201": {"description": "b"}, "200": {"description": "a"}}
    }));
    let a = v(json!({"description": "a"}));
    assert_eq!(success_response(&spec, obj(&op)), a.as_object());
    let string_response = v(json!({"responses": {"200": "x"}}));
    assert_eq!(success_response(&spec, obj(&string_response)), None);
    assert_eq!(success_response(&spec, &Map::new()), None);
}

fn path(segments: &[&str]) -> Vec<String> {
    segments.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn splits_names_into_words_as_forge_does() {
    assert_eq!(forge_words("$ferrule_client"), ["ferrule", "client"]);
    assert_eq!(forge_words("XMLDocument"), ["xml", "document"]);
    assert_eq!(forge_words("ipv4Cidr"), ["ipv4", "cidr"]);
}

#[test]
fn maps_a_leading_client_to_the_reserved_group_only() {
    assert_eq!(forge_group_path(&path(&["$client"])), [CLIENT_GROUP]);
    assert_eq!(forge_group_path(&path(&["$client", "x"])), [CLIENT_GROUP, "x"]);
    assert_eq!(forge_group_path(&path(&["a", "$client"])), ["a", "$client"]);
}

#[test]
fn flags_resources_that_share_the_reserved_group() {
    assert!(collides_with_client_group("ferrule_client"));
    assert!(collides_with_client_group("FerruleClient"));
    assert!(!collides_with_client_group("$client"));
    assert!(!collides_with_client_group("client"));
}
