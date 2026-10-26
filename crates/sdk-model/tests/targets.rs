//! Every stainless.yml target and the per-target `skip:`/`only:` lists in the neutral model:
//! `Client.targets`, `Method.only`, `ModelDecl.only`, and `Client::for_target`.

use ferrule_sdk_model::model::{Client, TargetSettings, Type};
use ferrule_sdk_model::{FerruleError, forge_to_model, to_json};
use serde_json::{Value, json};

fn r(schema: &str) -> Value {
    json!({ "$ref": format!("#/components/schemas/{schema}") })
}

fn ok_response(schema: &Value) -> Value {
    json!({ "200": { "description": "ok", "content": { "application/json": { "schema": schema } } } })
}

fn targets() -> Value {
    json!({
        "python": { "package_name": "acme", "edition": "python.2025-10-08", "options": { "options": { "use_uv": false } } },
        "go": { "package_name": "acme", "edition": "go.2026-03-10", "production_repo": "acme/acme-go", "options": {} },
    })
}

/// One method per `(path, resource, name, extra extensions, response)`; every method carries
/// `mirrored`.
fn doc(methods: &[(&str, &str, &str, Value, Value)], mirrored: &Value, schemas: &Value) -> Value {
    let mut paths = serde_json::Map::new();
    for (path, resource, name, extra, response) in methods {
        let order = methods.iter().position(|m| m.1 == *resource).unwrap_or_default();
        let mut op = json!({
            "x-fern-sdk-group-name": [resource],
            "x-fern-sdk-method-name": name,
            "x-ferrule-resource": { "path": [resource], "order": [order] },
            "x-ferrule-method-name": name,
            "x-ferrule-package": { "name": "acme", "repo": "acme/acme-python" },
            "responses": ok_response(response),
        });
        let o = op.as_object_mut().expect("an object");
        for (k, v) in mirrored
            .as_object()
            .expect("an object")
            .iter()
            .chain(extra.as_object().expect("an object"))
        {
            o.insert(k.clone(), v.clone());
        }
        paths.insert((*path).to_owned(), json!({ "get": op }));
    }
    json!({
        "openapi": "3.0.3",
        "info": { "version": "1", "title": "T" },
        "servers": [{ "url": "https://x" }],
        "paths": paths,
        "components": { "schemas": schemas },
    })
}

fn model(d: &Value) -> Result<Option<Client>, FerruleError> {
    forge_to_model(&serde_json::from_value(d.clone()).expect("a document Forge reads"))
}

fn client(d: &Value) -> Client {
    model(d).expect("converts").expect("has operations")
}

fn schemas() -> Value {
    json!({
        "Kept": { "type": "object", "properties": { "a": { "type": "string" } } },
        "GoOnly": { "type": "object", "description": "Go only.", "properties": { "b": { "type": "integer" } } },
        "PythonOnly": { "type": "object", "properties": { "c": { "type": "boolean" } } },
    })
}

/// `things.list` for every target, `things.get` for Go only, `extras.list` for Python only.
fn restricted(models: &Value) -> Value {
    doc(
        &[
            (
                "/things",
                "things",
                "list",
                json!({}),
                json!({ "type": "object", "properties": { "k": r("Kept"), "g": r("GoOnly") } }),
            ),
            (
                "/things/{id}",
                "things",
                "get",
                json!({ "x-ferrule-targets-only": ["go"] }),
                r("GoOnly"),
            ),
            (
                "/extras",
                "extras",
                "list",
                json!({ "x-ferrule-targets-only": ["python"] }),
                r("PythonOnly"),
            ),
        ],
        &json!({ "x-ferrule-targets": targets(), "x-ferrule-models": models }),
        &schemas(),
    )
}

fn all_models() -> Value {
    json!({
        "Kept": { "name": "Kept" },
        "GoOnly": { "name": "GoOnly", "x-ferrule-targets-only": ["go"] },
        "PythonOnly": { "name": "PythonOnly", "x-ferrule-targets-only": ["python"] },
    })
}

#[test]
fn reads_every_target_and_what_each_method_and_model_is_generated_for() {
    let c = client(&restricted(&all_models()));
    let keys: Vec<&str> = c.targets.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["python", "go"]);
    assert_eq!(
        c.target("go"),
        Some(&TargetSettings {
            package_name: Some("acme".into()),
            edition: Some("go.2026-03-10".into()),
            production_repo: Some("acme/acme-go".into()),
            options: json!({}),
        })
    );
    assert_eq!(
        c.target("python").map(|t| &t.options),
        Some(&json!({ "options": { "use_uv": false } }))
    );
    assert_eq!(c.target("ruby"), None);
    let only = |resource: &str, method: &str| {
        c.resources
            .iter()
            .find(|r| r.name.written == resource)
            .and_then(|r| r.methods.iter().find(|m| m.name.written == method))
            .map(|m| m.only.clone())
            .expect("method")
    };
    assert_eq!(only("things", "list"), None);
    assert_eq!(only("things", "get"), Some(vec!["go".to_owned()]));
    assert_eq!(only("extras", "list"), Some(vec!["python".to_owned()]));
    let decl_only: Vec<(&str, Option<Vec<String>>)> = c
        .models
        .iter()
        .map(|d| (d.r#ref.schema.as_str(), d.only.clone()))
        .collect();
    assert_eq!(
        decl_only,
        [
            ("GoOnly", Some(vec!["go".to_owned()])),
            ("Kept", None),
            ("PythonOnly", Some(vec!["python".to_owned()])),
        ]
    );
}

#[test]
fn a_document_without_the_extensions_has_no_targets_and_no_restrictions() {
    let c = client(&doc(
        &[("/things", "things", "list", json!({}), json!({ "type": "string" }))],
        &json!({}),
        &json!({}),
    ));
    assert_eq!(c.targets, Vec::new());
    assert_eq!(c.resources[0].methods[0].only, None);
    assert_eq!(c.for_target("python"), c);
}

#[test]
fn for_target_drops_what_the_target_does_not_generate_and_inlines_its_models() {
    let c = client(&restricted(&all_models()));
    let go = c.for_target("go");
    let names: Vec<&str> = go.resources.iter().map(|r| r.name.written.as_str()).collect();
    assert_eq!(names, ["things"]);
    assert_eq!(go.resources[0].methods.len(), 2);
    let schemas: Vec<&str> = go.models.iter().map(|d| d.r#ref.schema.as_str()).collect();
    assert_eq!(schemas, ["GoOnly", "Kept"]);

    let python = c.for_target("python");
    let names: Vec<&str> = python.resources.iter().map(|r| r.name.written.as_str()).collect();
    assert_eq!(names, ["things", "extras"]);
    assert_eq!(python.resources[0].methods.len(), 1);
    let schemas: Vec<&str> = python.models.iter().map(|d| d.r#ref.schema.as_str()).collect();
    assert_eq!(schemas, ["Kept", "PythonOnly"]);
    let Some(Type::Object(response)) = python.resources[0].methods[0].response.as_ref().map(|r| &r.ty) else {
        panic!("an object response")
    };
    let g = response.fields.iter().find(|f| f.wire_name == "g").expect("g");
    let Type::Object(inlined) = &g.ty else {
        panic!("GoOnly inlined, got {:?}", g.ty)
    };
    assert_eq!(inlined.origin.as_deref(), Some("GoOnly"));
    assert_eq!(inlined.description.as_deref(), Some("Go only."));
}

#[test]
fn for_target_gives_the_model_of_the_document_without_the_other_targets_models() {
    let mut without = all_models();
    without.as_object_mut().expect("an object").remove("GoOnly");
    let mut expected = client(&restricted(&without));
    let python = client(&restricted(&all_models())).for_target("python");
    // `things.get` is Go's alone; the rest is what the Python model would have been.
    expected.resources[0].methods.retain(|m| m.only.is_none());
    assert_eq!(python, expected.for_target("python"));
    assert_eq!(python.models, expected.models);
}

#[test]
fn rejects_malformed_target_extensions_with_fer101() {
    let cases = [
        json!({ "x-ferrule-targets": ["python"] }),
        json!({ "x-ferrule-targets": { "go": { "edition": 7, "options": {} } } }),
        json!({ "x-ferrule-targets": { "go": { "options": [] } } }),
        json!({ "x-ferrule-targets-only": "go" }),
        json!({ "x-ferrule-targets-only": [1] }),
        json!({ "x-ferrule-models": { "Kept": { "name": "Kept", "x-ferrule-targets-only": {} } } }),
    ];
    for extra in cases {
        let d = doc(
            &[("/things", "things", "list", extra.clone(), r("Kept"))],
            &json!({}),
            &schemas(),
        );
        let code = match model(&d) {
            Ok(_) => "none".to_owned(),
            Err(e) => e.code.to_string(),
        };
        assert_eq!(code, "FER101", "{extra}");
    }
}

#[test]
fn prints_the_targets_and_restrictions() {
    let c = client(&restricted(&all_models()));
    let printed = to_json(Some(&c));
    assert!(printed.contains("\"targets\": ["), "{printed}");
    assert!(printed.contains("\"only\": [\"go\"]"), "{printed}");
    assert!(!printed.contains("\"only\": null"), "{printed}");
    let plain = client(&doc(
        &[("/things", "things", "list", json!({}), json!({ "type": "string" }))],
        &json!({}),
        &json!({}),
    ));
    let printed = to_json(Some(&plain));
    assert!(
        !printed.contains("\"targets\"") && !printed.contains("\"only\""),
        "{printed}"
    );
}

#[test]
fn a_config_without_a_python_target_has_no_package_but_its_own_target() {
    // imagekit's csharp-branch config: `targets.csharp` only, so the converter writes no
    // x-ferrule-package (the Python target's); every other target reads its own settings.
    let mut d = doc(
        &[("/things", "things", "list", json!({}), json!({ "type": "string" }))],
        &json!({ "x-ferrule-targets": {
            "csharp": { "package_name": "Acme", "edition": "csharp.2025-10-08", "production_repo": "acme/acme-csharp#main", "options": {} },
        } }),
        &json!({}),
    );
    d["paths"]["/things"]["get"]
        .as_object_mut()
        .expect("an object")
        .remove("x-ferrule-package");
    let c = client(&d);
    assert_eq!((c.package.name.as_str(), c.package.repo.as_str()), ("", ""));
    assert_eq!(
        c.target("csharp").and_then(|t| t.package_name.as_deref()),
        Some("Acme")
    );
    assert_eq!(c.for_target("csharp").resources.len(), 1);
}
