//! Adapter tests: small OpenAPI documents read by Forge, then converted to the neutral model. Each
//! case is a document, since Forge can only be built from one.

use ferrule_sdk_model::model::{Client, Literal, Method, Object, Scalar, Type, TypeRef, Union};
use ferrule_sdk_model::{FerruleError, forge_to_model, group_to_model};
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
        "info": { "version": "1.2", "title": "T" },
        "servers": [{ "url": "https://x" }],
        "paths": { "/things/{id}": { "get": operation } },
        "components": { "schemas": schemas },
    })
}

fn forge(d: &Value) -> forge::Forge {
    serde_json::from_value(d.clone()).expect("a document Forge reads")
}

fn model(d: &Value) -> Result<Option<Client>, FerruleError> {
    forge_to_model(&forge(d))
}

fn only(d: &Value) -> (Client, Method) {
    let client = model(d).expect("converts").expect("has operations");
    let method = client.resources[0].methods[0].clone();
    (client, method)
}

fn scalar(s: Scalar) -> Type {
    Type::scalar(s)
}

fn code(r: Result<Option<Client>, FerruleError>) -> String {
    match r {
        Ok(_) => "none".to_owned(),
        Err(e) => e.code.to_string(),
    }
}

fn response_object(method: &Method) -> &Object {
    match method.response.as_ref().map(|r| &r.ty) {
        Some(Type::Object(o)) => o,
        other => panic!("an object response, got {other:?}"),
    }
}

fn field_type(obj: &Object, wire: &str) -> Type {
    obj.fields
        .iter()
        .find(|f| f.wire_name == wire)
        .map(|f| f.ty.clone())
        .expect(wire)
}

#[test]
fn keeps_every_scalar_forge_has() {
    let schema = json!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "format": "date-time" },
            "b": { "type": "string", "format": "date" },
            "c": { "type": "string", "format": "uuid" },
            "d": { "type": "string", "format": "binary" },
            "e": { "type": "number", "format": "float" },
            "f": { "type": "integer", "format": "int32" },
            "g": { "type": "integer", "format": "int64" },
            "h": { "type": "integer", "format": "uint8" },
            "i": { "type": "integer" },
            "j": { "type": "number" },
            "k": { "type": "string", "format": "byte" },
            "l": {},
            "m": { "type": "boolean" },
        },
    });
    let (_, method) = only(&doc(&json!({ "responses": ok_response(&schema) }), &json!({})));
    let obj = response_object(&method);
    for (wire, s) in [
        ("a", Scalar::DateTime),
        ("b", Scalar::Date),
        ("c", Scalar::Uuid),
        ("d", Scalar::Binary),
        ("e", Scalar::Float),
        ("f", Scalar::Int32),
        ("g", Scalar::Int64),
        ("h", Scalar::Uint8),
        ("i", Scalar::Int),
        ("j", Scalar::Double),
        ("k", Scalar::Bytes),
        ("l", Scalar::Unknown),
        ("m", Scalar::Boolean),
    ] {
        assert_eq!(field_type(obj, wire), scalar(s), "{wire}");
    }
    assert!(Scalar::Uint8.is_integer() && !Scalar::Float.is_integer());
}

#[test]
fn keeps_nullable_apart_from_optional_and_names_enum_members() {
    let schema = json!({
        "type": "object",
        "required": ["n"],
        "properties": {
            "n": { "type": "string", "nullable": true },
            "o": { "type": "string" },
            "u": { "oneOf": [{ "type": "string" }, { "type": "integer" }], "nullable": true },
            "e": { "enum": ["in_progress", 1, true] },
            "h": { "type": "object", "additionalProperties": { "type": "boolean" } },
            "i": { "type": "object" },
        },
    });
    let (_, method) = only(&doc(&json!({ "responses": ok_response(&schema) }), &json!({})));
    let obj = response_object(&method);
    let field = |w: &str| obj.fields.iter().find(|f| f.wire_name == w).expect(w);
    assert!(field("n").required && !field("o").required);
    assert_eq!(
        field("n").ty,
        Type::Nullable {
            inner: Box::new(scalar(Scalar::String))
        }
    );
    assert_eq!(field("o").ty, scalar(Scalar::String));
    let Type::Nullable { inner } = &field("u").ty else {
        panic!("a nullable union: {:?}", field("u").ty)
    };
    let Type::Union(Union { variants, .. }) = &**inner else {
        panic!("a union inside: {inner:?}")
    };
    assert_eq!(variants.len(), 2);
    let Type::Enum { members } = &field("e").ty else {
        panic!("an enum")
    };
    let values: Vec<&Literal> = members.iter().map(|m| &m.value).collect();
    assert_eq!(
        values,
        [
            &Literal::String("in_progress".into()),
            &Literal::Number(1.0),
            &Literal::Boolean(true)
        ]
    );
    assert_eq!(members[0].name.words, ["in", "progress"]);
    assert_eq!(
        field("h").ty,
        Type::Map {
            values: Box::new(scalar(Scalar::Boolean))
        }
    );
    // `{type: object}` allows any keys: Forge gives it unknown extra values, so it is a map.
    assert_eq!(
        field("i").ty,
        Type::Map {
            values: Box::new(scalar(Scalar::Unknown))
        }
    );
}

#[test]
fn names_union_variants_and_their_discriminator_values() {
    let r = |schema: &str| json!({ "$ref": format!("#/components/schemas/{schema}") });
    let d = doc(
        &json!({
            "x-ferrule-models": { "Cat": { "name": "Cat", "module": "cat" } },
            "responses": ok_response(&json!({
                "oneOf": [r("Cat"), r("Dog")],
                "discriminator": { "propertyName": "kind", "mapping": { "cat": "#/components/schemas/Cat" } },
            })),
        }),
        &json!({
            "Cat": { "type": "object", "required": ["kind"], "properties": { "kind": { "type": "string", "enum": ["cat"] } } },
            "Dog": { "type": "object", "required": ["kind"], "properties": { "kind": { "type": "string", "enum": ["dog"] } } },
        }),
    );
    let (_, method) = only(&d);
    let Some(Type::Union(u)) = method.response.as_ref().map(|r| &r.ty) else {
        panic!("a union response")
    };
    assert_eq!(u.discriminator.as_ref().map(|d| d.wire_name.as_str()), Some("kind"));
    assert!(u.exclusive);
    let mut named: Vec<(String, Option<String>)> = u
        .variants
        .iter()
        .map(|v| {
            (
                v.name.as_ref().map(|n| n.written.clone()).unwrap_or_default(),
                v.discriminator_value.clone(),
            )
        })
        .collect();
    named.sort();
    assert_eq!(
        named,
        [
            ("Cat".to_owned(), Some("cat".to_owned())),
            ("Dog".to_owned(), Some("dog".to_owned()))
        ]
    );
    let dog = u
        .variants
        .iter()
        .find(|v| v.name.as_ref().is_some_and(|n| n.written == "Dog"))
        .expect("Dog");
    let Type::Object(o) = &dog.ty else {
        panic!("Dog is inlined")
    };
    assert_eq!(o.origin.as_deref(), Some("Dog"));
}

/// A mapping may send several values to one variant; the model keeps every one, so a language
/// that dispatches on the value can register each.
#[test]
fn keeps_every_discriminator_value_a_mapping_gives_one_variant() {
    let r = |schema: &str| json!({ "$ref": format!("#/components/schemas/{schema}") });
    let d = doc(
        &json!({
            "x-ferrule-models": { "Cat": { "name": "Cat" }, "Dog": { "name": "Dog" } },
            "responses": ok_response(&json!({
                "oneOf": [r("Cat"), r("Dog")],
                "discriminator": { "propertyName": "kind", "mapping": {
                    "cat": "#/components/schemas/Cat",
                    "kitten": "#/components/schemas/Cat",
                    "dog": "#/components/schemas/Dog",
                } },
            })),
        }),
        &json!({
            "Cat": { "type": "object", "required": ["kind"], "properties": { "kind": { "type": "string" } } },
            "Dog": { "type": "object", "required": ["kind"], "properties": { "kind": { "type": "string" } } },
        }),
    );
    let (_, method) = only(&d);
    let response = serde_json::to_value(method.response.as_ref().map(|r| &r.ty)).expect("serializes");
    let variant = |i: usize| response["variants"][i]["name"]["written"].clone();
    let mut mapping: Vec<(String, Value)> = response["discriminator"]["mapping"]
        .as_array()
        .unwrap_or_else(|| panic!("a mapping on the discriminator: {}", response["discriminator"]))
        .iter()
        .map(|c| {
            let i = usize::try_from(c["variant"].as_u64().expect("an index")).expect("fits");
            (c["value"].as_str().expect("a value").to_owned(), variant(i))
        })
        .collect();
    mapping.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        mapping,
        [
            ("cat".to_owned(), json!("Cat")),
            ("dog".to_owned(), json!("Dog")),
            ("kitten".to_owned(), json!("Cat")),
        ]
    );
}

#[test]
fn splits_path_query_and_header_parameters_and_carries_the_body() {
    let (client, method) = only(&doc(
        &json!({
            "parameters": [
                { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } },
                { "name": "q", "in": "query", "explode": false, "schema": { "type": "array", "items": { "type": "string" } } },
                { "name": "X-Trace-ID", "in": "header", "deprecated": true, "schema": { "type": "string" } },
            ],
            "requestBody": { "required": true, "content": { "application/json": { "schema": {
                "type": "object", "properties": { "n": { "type": "string", "default": "x" } },
            } } } },
        }),
        &json!({}),
    ));
    let wires =
        |ps: &[ferrule_sdk_model::model::Param]| ps.iter().map(|p| p.field.wire_name.clone()).collect::<Vec<_>>();
    assert_eq!(wires(&method.path_params), ["id"]);
    assert_eq!(wires(&method.query_params), ["q"]);
    assert_eq!(method.query_params[0].exploded, Some(false));
    assert_eq!(method.path_params[0].exploded, None);
    assert_eq!(wires(&method.header_params), ["X-Trace-ID"]);
    let header = &method.header_params[0].field;
    assert!(header.deprecated);
    assert_eq!(header.name.words, ["x", "trace", "id"]);
    let body = method.body.as_ref().expect("a body");
    assert_eq!(body.media_type, "application/json");
    assert!(body.required);
    let body_obj = client.object_of(&body.ty).expect("an object body");
    assert_eq!(body_obj.fields[0].default, Some(json!("x")));
    assert_eq!(
        (
            client.base_url.as_str(),
            client.version.as_str(),
            client.description.as_str()
        ),
        ("https://x", "1.2", "T")
    );
    assert_eq!(method.pointer, "#/paths/~1things~1{id}/get");
    let response = method.response.as_ref().expect("a response");
    assert_eq!(
        (response.status.as_str(), response.media_type.as_str()),
        ("200", "application/json")
    );
}

/// With a body that is not an object, Forge's request is one object with a `body` field rather
/// than an intersection; its other fields are still the path and query parameters.
#[test]
fn reads_the_parameters_beside_a_body_that_is_not_an_object() {
    let (_, method) = only(&doc(
        &json!({
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "requestBody": { "content": { "multipart/form-data": { "schema": {
                "type": "object",
                "required": ["file"],
                "properties": { "file": { "type": "string", "format": "binary" } },
            } } } },
        }),
        &json!({}),
    ));
    let wires: Vec<_> = method.path_params.iter().map(|p| p.field.wire_name.as_str()).collect();
    assert_eq!(wires, ["id"]);
    assert_eq!(method.query_params, Vec::new());
    assert_eq!(
        method.body.as_ref().map(|b| b.media_type.as_str()),
        Some("multipart/form-data")
    );
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
    assert_eq!(method.body.map(|b| b.ty), Some(scalar(Scalar::String)));
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
            "Node": { "type": "object", "deprecated": true, "properties": { "next": { "$ref": "#/components/schemas/Node" } } },
        }),
    );
    let (client, method) = only(&d);
    let resp = response_object(&method);
    assert!(resp.bases.iter().all(|b| b.schema == "Base"), "{:?}", resp.bases);
    assert_ne!(resp.bases, Vec::new());
    let mut fields: Vec<&str> = resp.fields.iter().map(|f| f.wire_name.as_str()).collect();
    fields.sort_unstable();
    assert_eq!(fields, ["m", "node"]);
    let paths: Vec<String> = client
        .models
        .iter()
        .map(|m| {
            m.r#ref
                .path
                .iter()
                .map(|n| n.written.as_str())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect();
    assert_eq!(paths, ["Node", "shared.base"]);
    let node = client.models.iter().find(|m| m.r#ref.schema == "Node").expect("Node");
    assert!(node.deprecated);
    let Type::Object(body) = &node.ty else {
        panic!("an object")
    };
    let Type::Ref(TypeRef { schema, name, .. }) = &body.fields[0].ty else {
        panic!("a reference")
    };
    assert_eq!((schema.as_str(), name.written.as_str()), ("Node", "Node"));
}

#[test]
fn declares_named_models_that_are_not_objects_with_their_type() {
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
    let ty = |schema: &str| {
        client
            .models
            .iter()
            .find(|m| m.r#ref.schema == schema)
            .unwrap_or_else(|| panic!("{schema}"))
            .ty
            .clone()
    };
    assert_eq!(ty("S"), scalar(Scalar::String));
    assert!(matches!(ty("U"), Type::Union(_)), "{:?}", ty("U"));
    assert!(matches!(ty("E"), Type::Enum { members } if members.len() == 2));
}

#[test]
fn rejects_what_it_cannot_describe() {
    let r = |schema: &str| json!({ "$ref": format!("#/components/schemas/{schema}") });
    let cases = [
        (
            "a recursive inline schema",
            doc(
                &json!({ "responses": ok_response(&r("R")) }),
                &json!({ "R": { "type": "object", "properties": { "r": r("R") } } }),
            ),
            "FER201",
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
            "FER201",
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
            "FER101",
        ),
    ];
    for (what, d, expected) in cases {
        assert_eq!(code(model(&d)), expected, "{what}");
    }
}

#[test]
fn keeps_every_http_method_and_rejects_disagreeing_packages() {
    let mut trace = doc(&json!({}), &json!({}));
    let path = trace["paths"]["/things/{id}"].as_object_mut().expect("path item");
    let op = path.remove("get").expect("get");
    path.insert("trace".into(), op);
    let (_, method) = only(&trace);
    assert_eq!(method.http_method.as_str(), "trace");

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
        &json!({ "x-ferrule-resource": { "path": ["trip-details", "b"], "order": [0, 0] } }),
        &json!({}),
    ))
    .expect("converts")
    .expect("has operations");
    let top = &client.resources[0];
    assert_eq!(top.name.written, "trip-details");
    assert_eq!(top.name.words, ["trip", "details"]);
    let sub = &top.subresources[0];
    assert_eq!(
        sub.path.iter().map(|n| n.written.as_str()).collect::<Vec<_>>(),
        ["trip-details", "b"]
    );
    assert_eq!(sub.methods[0].name.written, "list");
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

#[test]
fn keeps_standalone_resources_without_methods_and_client_methods_apart() {
    let mut d = doc(
        &json!({
            "x-ferrule-standalone": [{ "path": ["extractions", "qa"], "order": [1, 0] }],
            "x-ferrule-casings": { "qa": { "initialism": true } },
        }),
        &json!({}),
    );
    let mut client_op = d["paths"]["/things/{id}"]["get"].clone();
    client_op["x-ferrule-resource"] = json!({ "path": ["$client"] });
    client_op["x-ferrule-method-name"] = json!("health");
    client_op["x-fern-sdk-method-name"] = json!("health");
    d["paths"]["/health"] = json!({ "get": client_op });
    let client = model(&d).expect("converts").expect("has operations");
    let names: Vec<&str> = client.resources.iter().map(|r| r.name.written.as_str()).collect();
    assert_eq!(names, ["things", "extractions"]);
    let qa = &client.resources[1].subresources[0];
    assert!(qa.standalone && qa.methods.is_empty());
    assert_eq!(client.client_methods[0].name.written, "health");
    assert_eq!(client.casings[0].word, "qa");
}

#[test]
fn prints_deterministic_json() {
    let d = doc(&json!({}), &json!({}));
    let a = ferrule_sdk_model::to_json(model(&d).expect("converts").as_ref());
    let b = ferrule_sdk_model::to_json(model(&d).expect("converts").as_ref());
    assert_eq!(a, b);
    assert!(a.ends_with("}\n"));
    let back: Client = serde_json::from_str(&a).expect("reads back");
    assert_eq!(Some(back), model(&d).expect("converts"));
    assert_eq!(ferrule_sdk_model::to_json(None), "null\n");
}
