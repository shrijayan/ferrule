//! Spec and config text is untrusted: values that become identifiers in any language are
//! validated, so nothing can inject code into an SDK. The cases are the Python plugin's, read into
//! the neutral shapes.

use serde_json::{Value, json};

use super::*;

fn to_json(v: &Value) -> Json {
    serde_json::from_value(v.clone()).expect("a JSON value")
}

/// `x-ferrule-*` pairs from a JSON object, in written order.
fn pairs(v: &Value) -> Vec<(String, Json)> {
    v.as_object()
        .expect("an object")
        .iter()
        .map(|(k, v)| (k.clone(), to_json(v)))
        .collect()
}

fn read(ext: &[(String, Json)]) -> Result<MethodExtensions, FerruleError> {
    let refs: Vec<(&str, &Json)> = ext.iter().map(|(k, v)| (k.as_str(), v)).collect();
    read_method_extensions(&refs, "#/p")
}

fn base() -> Value {
    json!({
        "x-ferrule-resource": { "path": ["things"] },
        "x-ferrule-method-name": "list",
        "x-ferrule-package": { "name": "acme", "repo": "acme/acme-python" },
    })
}

/// `base` with `key` set to `value`, or left out for `None`.
fn with(key: &str, value: Option<Value>) -> Vec<(String, Json)> {
    let mut v = base();
    let obj = v.as_object_mut().expect("an object");
    match value {
        Some(value) => {
            obj.insert(key.to_owned(), value);
        }
        None => {
            obj.shift_remove(key);
        }
    }
    pairs(&v)
}

fn code(r: Result<MethodExtensions, FerruleError>) -> String {
    match r {
        Ok(_) => "none".to_owned(),
        Err(e) => format!("{} {}", e.code, e.pointer),
    }
}

fn written(names: &[Name]) -> Vec<&str> {
    names.iter().map(|n| n.written.as_str()).collect()
}

#[test]
fn reads_every_extension() {
    let mut v = base();
    let obj = v.as_object_mut().expect("an object");
    obj.insert("x-other".into(), json!(1));
    obj.insert(
        "x-ferrule-models".into(),
        json!({ "Thing": { "name": "Thing", "module": "shared.thing" }, "B": { "name": "B" } }),
    );
    obj.insert(
        "x-ferrule-pagination".into(),
        json!({ "scheme": "cursor", "page": "cursor_page", "request": { "after": "next_cursor_param" }, "response": { "data": "items" } }),
    );
    let e = read(&pairs(&v)).expect("valid");
    assert_eq!(e.resource, ["things"]);
    assert_eq!(e.method_name, name("list"));
    assert_eq!(
        e.package,
        Package {
            name: "acme".into(),
            repo: "acme/acme-python".into()
        }
    );
    let thing = &e.models["Thing"];
    assert_eq!(written(&thing.path), ["shared", "thing"]);
    assert_eq!(thing.name.written, "Thing");
    assert_eq!(thing.name.words, ["thing"]);
    assert_eq!(written(&e.models["B"].path), ["B"]);
    let page = e.pagination.expect("pagination");
    assert_eq!(page.scheme, PaginationScheme::Cursor);
    assert_eq!(page.page.words, ["cursor", "page"]);
    assert_eq!(
        page.request,
        [Purpose {
            field: "after".into(),
            purpose: "next_cursor_param".into()
        }]
    );
}

#[test]
fn rejects_malformed_extensions() {
    let cases: Vec<(&str, Option<Value>, &str)> = vec![
        ("x-ferrule-bogus", Some(json!(1)), "FER102 #/p/x-ferrule-bogus"),
        ("x-ferrule-resource", None, "FER103 #/p/x-ferrule-resource"),
        (
            "x-ferrule-resource",
            Some(json!({ "path": [] })),
            "FER101 #/p/x-ferrule-resource",
        ),
        (
            "x-ferrule-resource",
            Some(json!({ "path": ["a b"] })),
            "FER101 #/p/x-ferrule-resource",
        ),
        (
            "x-ferrule-method-name",
            Some(json!(3)),
            "FER101 #/p/x-ferrule-method-name",
        ),
        (
            "x-ferrule-method-name",
            Some(json!("a-b")),
            "FER101 #/p/x-ferrule-method-name",
        ),
        (
            "x-ferrule-package",
            Some(json!({ "name": "a" })),
            "FER101 #/p/x-ferrule-package",
        ),
        ("x-ferrule-package", Some(json!("acme")), "FER101 #/p/x-ferrule-package"),
        (
            "x-ferrule-package",
            Some(json!({ "name": "a", "repo": "a/b\"\"\" x" })),
            "FER101 #/p/x-ferrule-package",
        ),
        ("x-ferrule-models", Some(json!([])), "FER101 #/p/x-ferrule-models"),
        (
            "x-ferrule-models",
            Some(json!({ "A": { "name": "A", "module": "Bad Mod" } })),
            "FER101 #/p/x-ferrule-models",
        ),
        (
            "x-ferrule-models",
            Some(json!({ "A": { "name": 1 } })),
            "FER101 #/p/x-ferrule-models",
        ),
        (
            "x-ferrule-models",
            Some(json!({ "A": "A" })),
            "FER101 #/p/x-ferrule-models",
        ),
        (
            "x-ferrule-pagination",
            Some(json!({ "scheme": "nope" })),
            "FER101 #/p/x-ferrule-pagination",
        ),
        (
            "x-ferrule-pagination",
            Some(json!({ "scheme": "cursor", "page": "p", "request": { "a": 1 }, "response": {} })),
            "FER101 #/p/x-ferrule-pagination",
        ),
        (
            "x-ferrule-streaming",
            Some(json!({ "format": "xml" })),
            "FER101 #/p/x-ferrule-streaming",
        ),
        (
            "x-ferrule-resource",
            Some(json!({ "path": ["a"], "order": [1, 2] })),
            "FER101 #/p/x-ferrule-resource",
        ),
        (
            "x-ferrule-method-order",
            Some(json!(-1)),
            "FER101 #/p/x-ferrule-method-order",
        ),
        (
            "x-ferrule-streaming",
            Some(json!({ "format": "sse", "paramDiscriminator": 3 })),
            "FER101 #/p/x-ferrule-streaming",
        ),
        (
            "x-ferrule-client",
            Some(json!({ "name": "x" })),
            "FER101 #/p/x-ferrule-client",
        ),
    ];
    for (key, value, expected) in cases {
        assert_eq!(code(read(&with(key, value.clone()))), expected, "{key} = {value:?}");
    }
}

#[test]
fn rejects_malformed_casings_and_standalone_resources() {
    let cases = [
        ("x-ferrule-casings", json!({ "QA": { "initialism": true } })),
        ("x-ferrule-casings", json!({ "qa": { "pascal": "Q A" } })),
        (
            "x-ferrule-casings",
            json!({ "qa": { "initialism": true, "pascal": "QA" } }),
        ),
        ("x-ferrule-standalone", json!({ "path": ["a"] })),
        ("x-ferrule-standalone", json!([{ "path": ["a b"] }])),
    ];
    for (key, value) in cases {
        assert_eq!(
            code(read(&with(key, Some(value.clone())))),
            format!("FER101 #/p/{key}"),
            "{key} = {value}"
        );
    }
}

#[test]
fn error_messages_carry_code_pointer_and_fix() {
    let e = read(&[]).expect_err("missing extensions");
    let text = e.to_string();
    assert!(text.starts_with("FER103 at #/p/x-ferrule-resource: "), "{text}");
    assert!(text.contains("Fix: "));
}

#[test]
fn treats_null_like_the_typescript_nullish_defaults() {
    let e = read(&with("x-ferrule-method-order", Some(Value::Null))).expect("null order is 0");
    assert_eq!(e.method_order, 0);
    let e = read(&with("x-ferrule-models", Some(Value::Null))).expect("null models are none");
    assert!(e.models.is_empty());
    assert_eq!(
        code(read(&with("x-ferrule-pagination", Some(Value::Null)))),
        "FER101 #/p/x-ferrule-pagination"
    );
}

#[test]
fn strips_a_branch_from_the_production_repo() {
    let e = read(&with(
        "x-ferrule-package",
        Some(json!({ "name": "fx", "repo": "acme/fx-python#master" })),
    ))
    .expect("a repo with a branch");
    assert_eq!(e.package.repo, "acme/fx-python");
    for bad in ["acme/fx#", "acme/fx#a b", "acme/fx#a#b", "#master"] {
        assert_eq!(
            code(read(&with(
                "x-ferrule-package",
                Some(json!({ "name": "fx", "repo": bad }))
            ))),
            "FER101 #/p/x-ferrule-package",
            "{bad}"
        );
    }
}

/// `module` segments are stainless.yml keys as written; the model keeps them as written, with
/// their words, so each language spells its own file path.
#[test]
fn keeps_model_modules_as_written_config_keys() {
    let e = read(&with(
        "x-ferrule-models",
        Some(json!({
            "AssistantMessage": { "name": "AssistantMessage", "module": "assistantMessage" },
            "UrlEndpointResponse": { "name": "UrlEndpointResponse", "module": "accounts.UrlEndpointResponse" },
            "Stop": { "name": "Stop", "module": "trip-details.stop" },
            "RouteIds": { "name": "RouteIDs", "module": "route_ids" },
        })),
    ))
    .expect("config keys as modules");
    let path = |schema: &str| written(&e.models[schema].path).join(".");
    assert_eq!(path("AssistantMessage"), "assistantMessage");
    assert_eq!(path("UrlEndpointResponse"), "accounts.UrlEndpointResponse");
    assert_eq!(path("Stop"), "trip-details.stop");
    assert_eq!(e.models["Stop"].path[0].words, ["trip", "details"]);
    assert_eq!(e.models["AssistantMessage"].name.words, ["assistant", "message"]);
    let ids = &e.models["RouteIds"].name;
    assert_eq!(
        (ids.words.as_slice(), ids.written.as_str()),
        (&["route".to_owned(), "ids".to_owned()][..], "RouteIDs")
    );
    for bad in ["a..b", "a b", ".a", "a;b", "1a", "$client"] {
        assert_eq!(
            code(read(&with(
                "x-ferrule-models",
                Some(json!({ "A": { "name": "A", "module": bad } }))
            ))),
            "FER101 #/p/x-ferrule-models",
            "{bad}"
        );
    }
}

#[test]
fn reads_casings_and_standalone_resources() {
    let mut v = base();
    let obj = v.as_object_mut().expect("an object");
    obj.insert(
        "x-ferrule-casings".into(),
        json!({ "qa": { "initialism": true }, "lumaai": { "pascal": "LumaAI" } }),
    );
    obj.insert(
        "x-ferrule-standalone".into(),
        json!([{ "path": ["extractions", "qa"], "order": [1, 0] }]),
    );
    let e = read(&pairs(&v)).expect("valid");
    assert_eq!(
        e.casings,
        [
            CustomCasing {
                word: "qa".into(),
                casing: Casing::Initialism(true)
            },
            CustomCasing {
                word: "lumaai".into(),
                casing: Casing::Pascal("LumaAI".into())
            },
        ]
    );
    assert_eq!(
        e.standalone,
        [(vec!["extractions".to_owned(), "qa".to_owned()], vec![1, 0])]
    );
}

fn client() -> Value {
    json!({
        "name": "acme-sdk",
        "envPrefix": "ACME_SDK",
        "opts": [{
            "name": "api_key",
            "type": "string",
            "nullable": false,
            "readEnv": "ACME_API_KEY",
            "auth": { "securityScheme": "K", "role": "value" },
            "send": { "in": "query", "name": "key" },
        }],
        "environments": [{ "name": "production", "url": "https://x.test" }],
        "defaultEnvironment": "production",
        "securitySchemes": {
            "K": { "type": "apiKey", "in": "query", "name": "key" },
            "B": { "type": "http", "scheme": "bearer", "keyword": "Token" },
            "D": { "type": "http", "scheme": "digest" },
            "O": { "type": "oauth2", "flows": {} },
        },
        "security": [{ "K": [] }],
        "timeoutSeconds": 60,
        "retries": { "maxRetries": 3, "initialDelaySeconds": 0.5, "maxDelaySeconds": 8 },
        "query": { "arrayFormat": "comma", "nestedFormat": "dots" },
        "defaultHeaders": { "X-A": "b" },
    })
}

fn client_with(key: &str, value: Option<Value>) -> Result<RawClient, Malformed> {
    let mut c = client();
    let obj = c.as_object_mut().expect("an object");
    match value {
        Some(v) => {
            obj.insert(key.to_owned(), v);
        }
        None => {
            obj.shift_remove(key);
        }
    }
    read_client(&to_json(&c))
}

#[test]
fn reads_every_member_of_the_client() {
    let c = read_client(&to_json(&client())).expect("valid");
    assert_eq!(c.name.words, ["acme", "sdk"]);
    assert_eq!(c.max_retries, 3.0);
    assert_eq!(
        (c.initial_retry_delay_seconds, c.max_retry_delay_seconds),
        (Some(0.5), Some(8.0))
    );
    assert_eq!(c.default_environment.as_deref(), Some("production"));
    assert_eq!(c.array_format, ArrayFormat::Comma);
    assert_eq!(c.nested_format, NestedFormat::Dots);
    assert_eq!(
        c.security_schemes[1].scheme,
        SecurityScheme::Http {
            scheme: HttpAuthScheme::Bearer,
            keyword: Some("Token".into())
        }
    );
    assert_eq!(c.security, [["K"]]);
    assert_eq!(c.opts[0].name, name("api_key"));
    assert_eq!(
        c.opts[0].send,
        Some(Send {
            location: SendLocation::Query,
            name: "key".into()
        })
    );
    assert_eq!(
        c.default_headers,
        [Header {
            name: "X-A".into(),
            value: "b".into()
        }]
    );
    let mut d = client();
    let obj = d.as_object_mut().expect("an object");
    obj.insert("retries".into(), json!({}));
    obj.insert("query".into(), json!({}));
    obj.shift_remove("defaultHeaders");
    let d = read_client(&to_json(&d)).expect("valid");
    assert_eq!(
        (d.max_retries, d.array_format, d.nested_format),
        (2.0, ArrayFormat::Repeat, NestedFormat::Brackets)
    );
}

#[test]
fn rejects_a_bad_client_member() {
    let cases = [
        ("name", json!(1)),
        ("environments", json!([{ "name": 1 }])),
        ("security", json!(["K"])),
        ("retries", json!({ "maxRetries": "x" })),
        ("retries", json!({ "initialDelaySeconds": "x" })),
        ("query", json!({ "arrayFormat": "pipes" })),
        ("query", json!({ "nestedFormat": "pipes" })),
        ("opts", json!([{ "name": "a-b", "type": "string", "nullable": false }])),
        (
            "opts",
            json!([{ "name": "a", "type": "string", "nullable": false, "auth": { "role": "x" } }]),
        ),
        (
            "opts",
            json!([{ "name": "a", "type": "string", "nullable": false, "send": { "in": "cookie" } }]),
        ),
        (
            "securitySchemes",
            json!({ "X": { "type": "http", "scheme": "negotiate" } }),
        ),
        (
            "securitySchemes",
            json!({ "X": { "type": "http", "scheme": "bearer", "keyword": 1 } }),
        ),
        (
            "securitySchemes",
            json!({ "X": { "type": "apiKey", "in": "body", "name": "k" } }),
        ),
        ("securitySchemes", json!({ "X": { "type": "mutualTLS" } })),
    ];
    for (key, value) in cases {
        assert_eq!(client_with(key, Some(value.clone())), Err(Malformed), "{key} = {value}");
    }
}

#[test]
fn reads_order_streaming_and_pagination_from_operations() {
    let ext = pairs(&json!({
        "x-ferrule-resource": { "path": ["trip-details", "sub"], "order": [3, 1] },
        "x-ferrule-method-name": "retrieve",
        "x-ferrule-method-order": 2,
        "x-ferrule-package": { "name": "acme", "repo": "" },
        "x-ferrule-client": client(),
        "x-ferrule-streaming": { "format": "jsonl", "paramDiscriminator": null },
        "x-ferrule-pagination": {
            "scheme": "offset",
            "page": "offset_page",
            "paramLocation": "body",
            "request": { "offset": "offset_count_param" },
            "response": { "data": "items" },
            "headers": { "X-Total": "total" },
        },
    }));
    let e = read(&ext).expect("valid");
    assert_eq!(e.resource, ["trip-details", "sub"]);
    assert_eq!(e.resource_order, [3, 1]);
    assert_eq!(e.method_order, 2);
    assert_eq!(
        e.streaming,
        Some(RawStreaming {
            format: StreamFormat::Jsonl,
            param_discriminator: None,
            event_schema: None
        })
    );
    assert_eq!(e.pagination.map(|p| p.param_location), Some(ParamLocation::Body));
    assert_eq!(e.client.map(|c| c.name.written), Some("acme-sdk".to_owned()));
}

#[test]
fn rejects_untrusted_client_values() {
    for url in ["javascript:x", "https://a\"+__import__(\"os\")+\"", "https://{x}"] {
        assert_eq!(
            client_with("environments", Some(json!([{ "name": "p", "url": url }]))),
            Err(Malformed),
            "{url}"
        );
    }
    assert_eq!(client_with("envPrefix", Some(json!("A_\") or (\""))), Err(Malformed));
    let opts = json!([{ "name": "api_key", "type": "string", "nullable": false, "readEnv": "X\")\nimport os#" }]);
    assert_eq!(client_with("opts", Some(opts)), Err(Malformed));
}

#[test]
fn rejects_untrusted_pagination_fields() {
    let cases = [
        ("response", json!({ "data; import os": "items" })),
        ("response", json!({ "data.[].id)": "cursor_item_id" })),
        ("request", json!({ "a-b": "next_cursor_param" })),
    ];
    for (side, map) in cases {
        let mut page = json!({ "scheme": "cursor", "page": "p", "request": {}, "response": {} });
        page.as_object_mut()
            .expect("an object")
            .insert(side.to_owned(), map.clone());
        let ext = pairs(&json!({
            "x-ferrule-resource": { "path": ["a"] },
            "x-ferrule-method-name": "list",
            "x-ferrule-package": { "name": "acme", "repo": "" },
            "x-ferrule-pagination": page,
        }));
        assert_eq!(code(read(&ext)), "FER101 #/p/x-ferrule-pagination", "{side} {map}");
    }
}

#[test]
fn visits_integer_like_keys_first_as_javascript_does() {
    let mut c = client();
    c.as_object_mut()
        .expect("an object")
        .insert("defaultHeaders".into(), json!({ "b": "1", "10": "2", "2": "3" }));
    let c = read_client(&to_json(&c)).expect("valid");
    let keys: Vec<&str> = c.default_headers.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(keys, ["2", "10", "b"]);
}
