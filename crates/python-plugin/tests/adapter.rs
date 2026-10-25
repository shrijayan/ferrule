//! Adapter tests: small OpenAPI documents read by Forge, then converted to the model. Port of the
//! `toModel` tests in `test/adapter.test.ts`; the TypeScript fake Forge does not carry over, so
//! each case is a document (Forge can only be built from one).

use ferrule_python_plugin::adapter::{forge_to_model, group_to_model};
use ferrule_python_plugin::diagnostics::FerruleError;
use ferrule_python_plugin::model::{SdkClient, SdkLiteral, SdkMethod, SdkPrimitive, SdkRef, SdkType};
use serde_json::{Value, json};

fn base_ext() -> Value {
    json!({
        "x-fern-sdk-group-name": ["things"],
        "x-fern-sdk-method-name": "list",
        "x-ferrule-resource": { "path": ["things"] },
        "x-ferrule-method-name": "list",
        "x-ferrule-package": { "name": "acme", "repo": "acme/acme-python" },
    })
}

fn ok_response(schema: &Value) -> Value {
    json!({ "200": { "description": "ok", "content": { "application/json": { "schema": schema } } } })
}

/// A one-operation document; `op` overrides members of the operation.
fn doc(op: &Value, schemas: &Value) -> Value {
    let mut operation = base_ext();
    let o = operation.as_object_mut().expect("an object");
    o.insert("responses".into(), ok_response(&json!({ "type": "string" })));
    for (k, v) in op.as_object().expect("an object") {
        o.insert(k.clone(), v.clone());
    }
    json!({
        "openapi": "3.0.3",
        "info": { "version": "1", "title": "T" },
        "servers": [{ "url": "https://x" }],
        "paths": { "/things/{id}": { "get": operation } },
        "components": { "schemas": schemas },
    })
}

fn forge(d: &Value) -> forge::Forge {
    serde_json::from_value(d.clone()).expect("a document Forge reads")
}

fn model(d: &Value) -> Result<Option<SdkClient>, FerruleError> {
    forge_to_model(&forge(d))
}

fn only(d: &Value) -> (SdkClient, SdkMethod) {
    let client = model(d).expect("converts").expect("has operations");
    let method = client.resources[0].methods[0].clone();
    (client, method)
}

fn prim(p: SdkPrimitive) -> SdkType {
    SdkType::Primitive(p)
}

fn code(r: Result<Option<SdkClient>, FerruleError>) -> String {
    match r {
        Ok(_) => "none".to_owned(),
        Err(e) => e.code.to_string(),
    }
}

#[test]
fn maps_scalars_enums_maps_unions_and_nullability() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "format": "date-time" },
            "b": { "type": "string", "format": "date" },
            "c": { "type": "string", "format": "uuid" },
            "d": { "type": "string", "format": "binary" },
            "e": { "type": "number", "format": "float" },
            "f": { "type": "integer", "format": "int32" },
            "g": { "enum": ["x", 1] },
            "h": { "type": "object", "additionalProperties": { "type": "boolean" } },
            "i": { "type": "object" },
            "j": { "oneOf": [{ "type": "string" }, { "type": "integer" }] },
            "k": { "type": "string", "nullable": true },
            "l": {},
        },
    });
    let (_, method) = only(&doc(&json!({ "responses": ok_response(&schema) }), &json!({})));
    let Some(SdkType::Object(obj)) = &method.response else {
        panic!("an object response")
    };
    let ty = |wire: &str| {
        obj.fields
            .iter()
            .find(|f| f.wire_name == wire)
            .map(|f| f.ty.clone())
            .expect(wire)
    };
    assert_eq!(ty("a"), prim(SdkPrimitive::Datetime));
    assert_eq!(ty("b"), prim(SdkPrimitive::Date));
    assert_eq!(ty("c"), prim(SdkPrimitive::String));
    assert_eq!(ty("d"), SdkType::Unknown);
    assert_eq!(ty("e"), prim(SdkPrimitive::Number));
    assert_eq!(ty("f"), prim(SdkPrimitive::Integer));
    assert_eq!(
        ty("g"),
        SdkType::Literal(vec![SdkLiteral::String("x".into()), SdkLiteral::Number(1.0)])
    );
    assert_eq!(ty("h"), SdkType::Map(Box::new(prim(SdkPrimitive::Boolean))));
    // `{type: object}` allows any keys: Forge gives it unknown extra values, so it is a map.
    assert_eq!(ty("i"), SdkType::Map(Box::new(SdkType::Unknown)));
    assert_eq!(ty("l"), SdkType::Unknown);
    // Forge gives union members in an unspecified order (gap G17); the type mapper sorts them.
    for (wire, members) in [
        ("j", [prim(SdkPrimitive::String), prim(SdkPrimitive::Integer)]),
        ("k", [prim(SdkPrimitive::String), prim(SdkPrimitive::Null)]),
    ] {
        let SdkType::Union {
            variants,
            discriminator: None,
        } = ty(wire)
        else {
            panic!("{wire} is a union")
        };
        assert_eq!(variants.len(), 2);
        assert!(members.iter().all(|m| variants.contains(m)), "{wire}: {variants:?}");
    }
}

#[test]
fn splits_path_and_query_parameters_and_carries_the_body() {
    let (client, method) = only(&doc(
        &json!({
            "parameters": [
                { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } },
                { "name": "q", "in": "query", "schema": { "type": "string" } },
            ],
            "requestBody": { "content": { "application/json": { "schema": {
                "type": "object", "properties": { "n": { "type": "string" } },
            } } } },
        }),
        &json!({}),
    ));
    let names =
        |ps: &[ferrule_python_plugin::model::SdkParam]| ps.iter().map(|p| p.py_name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&method.path_params), ["id"]);
    assert_eq!(names(&method.query_params), ["q"]);
    assert_eq!(names(&method.body_params), ["n"]);
    assert_eq!(method.body_media_type.as_deref(), Some("application/json"));
    assert_eq!(client.base_url, "https://x");
    assert_eq!(method.pointer, "#/paths/~1things~1{id}/get");
}

/// With a body that is not an object, Forge's request is one object with a `body` field rather
/// than an intersection; the TypeScript adapter then reads no parameters, and so does the port
/// (the resource emitter rejects such a body with FER201).
#[test]
fn reads_no_parameters_beside_a_body_that_is_not_an_object() {
    let (_, method) = only(&doc(
        &json!({
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "requestBody": { "content": { "application/json": { "schema": { "type": "string" } } } },
        }),
        &json!({}),
    ));
    assert_eq!(method.path_params, Vec::new());
    assert_eq!(method.body, Some(prim(SdkPrimitive::String)));
    assert_eq!(method.body_params, Vec::new());
}

#[test]
fn handles_a_body_only_request_and_no_json_response() {
    let (_, method) = only(&doc(
        &json!({
            "requestBody": { "content": { "application/json": { "schema": { "type": "string" } } } },
            "responses": { "204": { "description": "none" } },
        }),
        &json!({}),
    ));
    assert_eq!(method.query_params, Vec::new());
    assert_eq!(method.response, None);
}

#[test]
fn names_models_flattens_all_of_and_keeps_self_references_finite() {
    let d = doc(
        &json!({
            "x-ferrule-models": { "Node": { "name": "Node" }, "Base": { "name": "Base", "module": "shared.base" } },
            "responses": ok_response(&json!({
                "allOf": [
                    { "$ref": "#/components/schemas/Base" },
                    { "$ref": "#/components/schemas/Mixin" },
                    { "type": "object", "properties": { "node": { "$ref": "#/components/schemas/Node" } } },
                ],
            })),
        }),
        &json!({
            "Base": { "type": "object", "properties": { "code": { "type": "integer" } } },
            "Mixin": {
                "allOf": [
                    { "$ref": "#/components/schemas/Base" },
                    { "type": "object", "properties": { "m": { "type": "string" } } },
                ],
            },
            "Node": { "type": "object", "properties": { "next": { "$ref": "#/components/schemas/Node" } } },
        }),
    );
    let (client, method) = only(&d);
    let Some(SdkType::Object(resp)) = &method.response else {
        panic!("an object response")
    };
    assert!(resp.bases.iter().all(|b| b.name == "Base"), "{:?}", resp.bases);
    assert_ne!(resp.bases, Vec::new());
    let mut fields: Vec<&str> = resp.fields.iter().map(|f| f.wire_name.as_str()).collect();
    fields.sort_unstable();
    assert_eq!(fields, ["m", "node"]);
    let modules: Vec<&str> = client.models.iter().map(|m| m.r#ref.module.as_str()).collect();
    assert_eq!(modules, ["node", "shared.base"]);
    let node = client.models.iter().find(|m| m.r#ref.name == "Node").expect("Node");
    assert_eq!(
        node.ty.fields[0].ty,
        SdkType::Ref(SdkRef {
            name: "Node".into(),
            module: "node".into()
        })
    );
}

#[test]
fn declares_named_models_that_are_not_objects_as_aliases() {
    let r = |schema: &str| json!({ "$ref": format!("#/components/schemas/{schema}") });
    let d = doc(
        &json!({
            "x-ferrule-models": { "S": { "name": "S" }, "U": { "name": "U" }, "E": { "name": "E" } },
            "responses": ok_response(&json!({ "type": "object", "properties": {
                "s": r("S"), "u": r("U"), "e": r("E"),
            } })),
        }),
        &json!({
            "S": { "type": "string" },
            "U": { "oneOf": [{ "type": "string" }, { "type": "integer" }] },
            "E": { "type": "string", "enum": ["a", "b"] },
        }),
    );
    let (client, _) = only(&d);
    let alias = |name: &str| {
        client
            .models
            .iter()
            .find(|m| m.r#ref.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .alias
            .clone()
    };
    assert_eq!(alias("S"), Some(prim(SdkPrimitive::String)));
    assert!(matches!(alias("U"), Some(SdkType::Union { .. })), "{:?}", alias("U"));
    assert_eq!(
        alias("E"),
        Some(SdkType::Literal(vec![
            SdkLiteral::String("a".into()),
            SdkLiteral::String("b".into())
        ]))
    );
}

#[test]
fn rejects_what_it_cannot_generate() {
    let r = |schema: &str| json!({ "$ref": format!("#/components/schemas/{schema}") });
    let cases = [
        (
            "a recursive inline schema",
            doc(
                &json!({ "responses": ok_response(&r("R")) }),
                &json!({ "R": { "type": "object", "properties": { "r": r("R") } } }),
            ),
        ),
        (
            "an allOf of a scalar",
            doc(
                &json!({ "responses": ok_response(&json!({ "allOf": [
                    { "type": "string" },
                    { "type": "object", "properties": { "a": { "type": "string" } } },
                ] })) }),
                &json!({}),
            ),
        ),
        (
            "a pagination without an items field",
            doc(
                &json!({
                    "x-ferrule-pagination": {
                        "scheme": "cursor", "page": "cursor_page", "request": {}, "response": { "data": "items" },
                    },
                }),
                &json!({}),
            ),
        ),
    ];
    for (what, d) in cases {
        let c = code(model(&d));
        assert!(c == "FER101" || c == "FER201", "{what}: {c}");
    }
}

#[test]
fn rejects_unsupported_http_methods_and_disagreeing_packages() {
    let mut trace = doc(&json!({}), &json!({}));
    let path = trace["paths"]["/things/{id}"].as_object_mut().expect("path item");
    let op = path.remove("get").expect("get");
    path.insert("trace".into(), op);
    assert_eq!(code(model(&trace)), "FER201");

    let mut two = doc(&json!({}), &json!({}));
    let mut other = two["paths"]["/things/{id}"]["get"].clone();
    other["x-fern-sdk-method-name"] = json!("other");
    other["x-ferrule-package"] = json!({ "name": "other", "repo": "o/o" });
    two["paths"]["/things/{id}"]["post"] = other;
    assert_eq!(code(model(&two)), "FER101");
}

#[test]
fn builds_nested_resources_and_returns_none_without_operations() {
    let client = model(&doc(
        &json!({ "x-ferrule-resource": { "path": ["a", "b"] } }),
        &json!({}),
    ))
    .expect("converts")
    .expect("has operations");
    assert_eq!(client.resources[0].path, ["a"]);
    assert_eq!(client.resources[0].subresources[0].path, ["a", "b"]);
    let mut none = doc(&json!({}), &json!({}));
    let op = &mut none["paths"]["/things/{id}"]["get"];
    let o = op.as_object_mut().expect("an object");
    o.remove("x-fern-sdk-group-name");
    o.remove("x-fern-sdk-method-name");
    assert_eq!(model(&none).expect("converts"), None);
    let f = forge(&doc(&json!({}), &json!({})));
    let group = f.groups().next().expect("a group");
    assert!(group_to_model(group).expect("converts").is_some());
}
