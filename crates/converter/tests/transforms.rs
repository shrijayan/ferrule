//! Port of converter/test/transforms/{commands,apply}.test.ts (vitest).

use ferrule_converter::transforms::{
    ApplyTransformsResult, TRANSFORM_COMMANDS, apply_config_transforms, apply_transforms,
};
use ferrule_converter::value::{stable_stringify, to_json};
use ferrule_converter::{Diagnostic, Map, Value};
use proptest::prelude::*;
use serde_json::json;

fn j(v: serde_json::Value) -> Value {
    Value::from(v)
}

/// A JSON array literal as a list of values.
fn list(v: serde_json::Value) -> Vec<Value> {
    match Value::from(v) {
        Value::Array(items) => items,
        other => panic!("expected an array, got {other:?}"),
    }
}

/// Walk object keys and array indices; `None` when a step is missing.
fn get<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(v, |node, key| match node {
        Value::Array(items) => items.get(key.parse::<usize>().ok()?),
        _ => node.get(key),
    })
}

fn at<'a>(v: &'a Value, path: &[&str]) -> &'a Value {
    get(v, path).unwrap_or_else(|| panic!("no value at {path:?}"))
}

fn at_mut<'a>(v: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let mut node = v;
    for key in path {
        node = match node {
            Value::Array(items) => &mut items[key.parse::<usize>().unwrap()],
            Value::Object(map) => map.get_mut(key).unwrap(),
            _ => panic!("no value at {path:?}"),
        };
    }
    node
}

fn keys<'a>(v: &'a Value, path: &[&str]) -> Vec<&'a str> {
    at(v, path).as_object().unwrap().keys().map(String::as_str).collect()
}

fn codes(ds: &[Diagnostic]) -> Vec<String> {
    ds.iter().map(|d| d.code.clone()).collect()
}

fn pointers(ds: &[Diagnostic]) -> Vec<String> {
    ds.iter().map(|d| d.pointer.clone()).collect()
}

fn code_pointers(ds: &[Diagnostic]) -> Vec<[String; 2]> {
    ds.iter().map(|d| [d.code.clone(), d.pointer.clone()]).collect()
}

fn code_severities(ds: &[Diagnostic]) -> Vec<[String; 2]> {
    ds.iter().map(|d| [d.code.clone(), d.severity.to_string()]).collect()
}

fn code_severity_pointers(ds: &[Diagnostic]) -> Vec<[String; 3]> {
    ds.iter()
        .map(|d| [d.code.clone(), d.severity.to_string(), d.pointer.clone()])
        .collect()
}

fn no_diagnostics() -> Vec<Diagnostic> {
    Vec::new()
}

mod commands {
    use super::*;

    fn base() -> Value {
        j(json!({
            "openapi": "3.1.0",
            "info": { "title": "Pets", "version": "1" },
            "paths": {
                "/pets/{id}": {
                    "get": {
                        "parameters": [
                            { "name": "id", "in": "path", "schema": { "type": "integer" } },
                            { "name": "limit", "in": "query", "deprecated": true, "schema": { "type": "integer" } },
                        ],
                        "examples": { "a": 1 },
                        "responses": { "200": { "description": "ok" } },
                    },
                },
            },
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "description": "A pet.",
                        "required": ["name"],
                        "properties": { "name": { "type": "string" }, "role": { "type": "string", "default": "cat" } },
                    },
                    "user_response": { "type": "object" },
                    "ErrorTemplate": { "type": "object", "properties": { "message": { "type": "string" } } },
                    "NotFound": { "type": "object", "properties": { "message": { "type": "string" } } },
                    "Conflict": { "type": "object", "properties": { "message": { "type": "string" } } },
                    "Error": { "type": "object" },
                },
            },
        }))
    }

    fn run(command: &str, args: serde_json::Value) -> ApplyTransformsResult {
        let mut entry = Map::new();
        entry.insert("command", Value::str(command));
        entry.insert("reason", Value::str("test"));
        entry.insert("args", Value::from(args));
        apply_transforms(&base(), &[Value::Object(entry)], None)
    }

    fn first_code(command: &str, args: serde_json::Value) -> Option<String> {
        run(command, args).diagnostics.first().map(|d| d.code.clone())
    }

    const OP: [&str; 3] = ["paths", "/pets/{id}", "get"];
    const PARAMS: [&str; 4] = ["paths", "/pets/{id}", "get", "parameters"];
    const SCHEMAS: [&str; 2] = ["components", "schemas"];

    fn param_names(spec: &Value) -> Vec<&str> {
        at(spec, &PARAMS)
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.get("name").and_then(Value::as_str).unwrap())
            .collect()
    }

    mod update {
        use super::*;

        #[test]
        fn replaces_values_selected_by_a_filter() {
            let r = run(
                "update",
                json!({
                    "target": "$.paths..parameters[?(@.name == \"id\")].schema.type",
                    "value": "string",
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let params = at(&r.spec, &PARAMS);
            assert_eq!(at(params, &["0", "schema", "type"]).as_str(), Some("string"));
            assert_eq!(at(params, &["1", "schema", "type"]).as_str(), Some("integer"));
        }

        #[test]
        fn templates_strings_with_value() {
            let r = run(
                "update",
                json!({
                    "target": "$.components.schemas.Pet.description",
                    "value": "[BETA] {{value}} {{value}}",
                    "template": true,
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                at(&r.spec, &["components", "schemas", "Pet", "description"]).as_str(),
                Some("[BETA] A pet. A pet.")
            );
        }

        #[test]
        fn replaces_deep_equal_duplicates_found_with_matches_schema_honouring_ignore() {
            let r = run(
                "update",
                json!({
                    "target": [{
                        "matches_schema": "$.components.schemas.ErrorTemplate",
                        "ignore": "$.components.schemas.ErrorTemplate",
                    }],
                    "value": { "$ref": "#/components/schemas/Error" },
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let s = at(&r.spec, &SCHEMAS);
            let reference = j(json!({ "$ref": "#/components/schemas/Error" }));
            assert_eq!(at(s, &["NotFound"]), &reference);
            assert_eq!(at(s, &["Conflict"]), &reference);
            assert_eq!(at(s, &["ErrorTemplate", "type"]).as_str(), Some("object"));
        }

        #[test]
        fn accepts_ignore_as_a_list() {
            let r = run(
                "update",
                json!({
                    "target": [{
                        "matches_schema": "$.components.schemas.ErrorTemplate",
                        "ignore": ["$.components.schemas.ErrorTemplate", "$.components.schemas.Conflict"],
                    }],
                    "value": { "$ref": "#/components/schemas/Error" },
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let s = at(&r.spec, &SCHEMAS);
            assert_eq!(at(s, &["Conflict", "type"]).as_str(), Some("object"));
            assert!(get(s, &["NotFound", "$ref"]).is_some());
        }

        #[test]
        fn can_replace_the_whole_document_with_an_object() {
            let r = run("update", json!({ "target": "$", "value": { "openapi": "3.0.0" } }));
            assert_eq!(r.spec, j(json!({ "openapi": "3.0.0" })));
        }

        #[test]
        fn reports_a_target_that_matches_nothing_and_leaves_the_spec_alone() {
            let r = run(
                "update",
                json!({ "target": "$.components.schemas.Nope.type", "value": "x" }),
            );
            assert_eq!(r.spec, base());
            assert_eq!(r.diagnostics.len(), 1);
            let d = &r.diagnostics[0];
            assert_eq!(d.code, "FER1201");
            assert_eq!(d.severity.to_string(), "error");
            assert_eq!(d.pointer, "/openapi/transforms/0/args/target");
            assert!(d.suggestion.contains("delete this transform"), "{}", d.suggestion);
        }

        #[test]
        fn addresses_each_entry_of_a_target_list() {
            let r = run(
                "update",
                json!({ "target": ["$.info.title", "$.info.nope"], "value": "x" }),
            );
            assert_eq!(pointers(&r.diagnostics), ["/openapi/transforms/0/args/target/1"]);
            assert_eq!(at(&r.spec, &["info", "title"]).as_str(), Some("Pets"));
        }

        fn update_codes(args: serde_json::Value) -> Vec<String> {
            codes(&run("update", args).diagnostics)
        }

        #[test]
        fn rejects_bad_templates_non_string_template_targets_and_a_non_object_root() {
            assert_eq!(
                update_codes(json!({ "target": "$.info.title", "value": "no placeholder", "template": true })),
                ["FER1210"]
            );
            assert_eq!(
                update_codes(json!({ "target": "$.info.title", "value": 3, "template": true })),
                ["FER1210"]
            );
            assert_eq!(
                update_codes(json!({ "target": "$.info", "value": "{{value}}!", "template": true })),
                ["FER1205"]
            );
            assert_eq!(update_codes(json!({ "target": "$", "value": "x" })), ["FER1205"]);
        }

        fn target_diags(target: serde_json::Value) -> Vec<[String; 2]> {
            let mut args = serde_json::Map::new();
            args.insert("target".to_owned(), target);
            args.insert("value".to_owned(), json!(1));
            code_pointers(&run("update", serde_json::Value::Object(args)).diagnostics)
        }

        fn target_codes(target: serde_json::Value) -> Vec<String> {
            target_diags(target).into_iter().map(|[code, _]| code).collect()
        }

        #[test]
        fn reports_matches_schema_templates_that_match_zero_or_several_nodes_and_no_duplicates() {
            assert_eq!(
                target_diags(json!([{ "matches_schema": "$.components.schemas.Nope" }])),
                [["FER1207", "/openapi/transforms/0/args/target/0/matches_schema"]]
            );
            assert_eq!(
                target_diags(json!([{ "matches_schema": "$.components.schemas.*" }])),
                [["FER1207", "/openapi/transforms/0/args/target/0/matches_schema"]]
            );
            assert_eq!(
                target_codes(json!([{ "matches_schema": "$.info", "ignore": "$.info" }])),
                ["FER1201"]
            );
            assert_eq!(target_codes(json!([{ "matches_schema": "$.info[" }])), ["FER1202"]);
            assert_eq!(
                target_diags(json!([{ "matches_schema": "$.info", "ignore": ["$.info", "$["] }])),
                [
                    ["FER1202", "/openapi/transforms/0/args/target/0/ignore/1"],
                    ["FER1201", "/openapi/transforms/0/args/target/0"],
                ]
            );
        }

        #[test]
        fn reports_invalid_jsonpath_with_fer1202() {
            let r = run("update", json!({ "target": "$.info[", "value": 1 }));
            assert_eq!(r.diagnostics.len(), 1);
            assert_eq!(r.diagnostics[0].code, "FER1202");
            assert_eq!(r.diagnostics[0].pointer, "/openapi/transforms/0/args/target");
        }
    }

    mod append {
        use super::*;

        #[test]
        fn pushes_an_item_or_several_items_onto_arrays() {
            let one = run(
                "append",
                json!({ "target": "$.components.schemas.Pet.required", "value": "id" }),
            );
            assert_eq!(one.diagnostics, no_diagnostics());
            assert_eq!(
                at(&one.spec, &["components", "schemas", "Pet", "required"]),
                &j(json!(["name", "id"]))
            );
            let many = run(
                "append",
                json!({ "target": "$.components.schemas.Pet.required", "value": ["id", "role"] }),
            );
            assert_eq!(
                at(&many.spec, &["components", "schemas", "Pet", "required"]),
                &j(json!(["name", "id", "role"]))
            );
        }

        #[test]
        fn adds_new_properties_to_objects() {
            let r = run(
                "append",
                json!({
                    "target": "$.components.schemas.Pet.properties",
                    "value": { "id": { "type": "string", "format": "uuid" } },
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                at(&r.spec, &["components", "schemas", "Pet", "properties", "id"]),
                &j(json!({ "type": "string", "format": "uuid" }))
            );
        }

        #[test]
        fn fails_when_a_property_already_exists_so_a_fixed_spec_is_noticed() {
            let r = run(
                "append",
                json!({
                    "target": "$.components.schemas.Pet.properties",
                    "value": { "name": { "type": "integer" }, "a/b": 1 },
                }),
            );
            assert_eq!(r.spec, base());
            assert_eq!(r.diagnostics.len(), 1);
            assert_eq!(r.diagnostics[0].code, "FER1206");
            assert_eq!(r.diagnostics[0].pointer, "/openapi/transforms/0/args/value/name");
        }

        #[test]
        fn reports_no_match_and_wrong_target_or_value_types() {
            assert_eq!(
                first_code("append", json!({ "target": "$.nope", "value": 1 })).as_deref(),
                Some("FER1201")
            );
            assert_eq!(
                first_code("append", json!({ "target": "$.info.title", "value": 1 })).as_deref(),
                Some("FER1205")
            );
            let r = run("append", json!({ "target": "$.info", "value": "x" }));
            assert_eq!(r.diagnostics.len(), 1);
            assert_eq!(r.diagnostics[0].code, "FER1205");
            assert_eq!(r.diagnostics[0].pointer, "/openapi/transforms/0/args/value");
        }
    }

    mod merge {
        use super::*;

        #[test]
        fn deep_merges_into_every_target_overwriting_leaves() {
            let r = run(
                "merge",
                json!({
                    "target": "$..parameters[?(@.name == \"limit\")]",
                    "value": { "schema": { "x-stainless-skip": ["terraform"], "type": "number" }, "in": "header" },
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                at(at(&r.spec, &PARAMS), &["1"]),
                &j(json!({
                    "name": "limit",
                    "in": "header",
                    "deprecated": true,
                    "schema": { "type": "number", "x-stainless-skip": ["terraform"] },
                }))
            );
        }

        #[test]
        fn reports_no_match_and_non_object_targets() {
            assert_eq!(
                first_code("merge", json!({ "target": "$.nope", "value": {} })).as_deref(),
                Some("FER1201")
            );
            let r = run("merge", json!({ "target": "$.info.title", "value": {} }));
            assert_eq!(r.diagnostics.len(), 1);
            assert_eq!(r.diagnostics[0].code, "FER1205");
            assert!(
                r.diagnostics[0].message.contains("found string"),
                "{}",
                r.diagnostics[0].message
            );
            let r = run("merge", json!({ "target": "$.paths..parameters", "value": {} }));
            let message = &r.diagnostics.first().unwrap().message;
            assert!(message.contains("found array"), "{message}");
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn removes_nodes_from_objects_and_arrays() {
            let r = run(
                "remove",
                json!({
                    "target": ["$.paths.*.*.examples", "$..parameters[?(@.deprecated == true)]"],
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let op = at(&r.spec, &OP);
            assert!(op.get("examples").is_none());
            assert_eq!(param_names(&r.spec), ["id"]);
        }

        #[test]
        fn removes_several_elements_of_one_array_without_shifting_indices() {
            let r = run("remove", json!({ "target": "$..parameters[*]" }));
            assert_eq!(at(&r.spec, &PARAMS), &j(json!([])));
        }

        #[test]
        fn removes_nested_matches_together_with_their_ancestors() {
            let r = run("remove", json!({ "target": "$..type" }));
            assert_eq!(r.diagnostics, no_diagnostics());
            assert!(!to_json(&r.spec).contains("\"type\""));
        }

        #[test]
        fn removes_specific_keys() {
            let r = run(
                "remove",
                json!({ "target": "$.components.schemas.Pet.properties.role", "keys": ["default"] }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                at(&r.spec, &["components", "schemas", "Pet", "properties", "role"]),
                &j(json!({ "type": "string" }))
            );
        }

        #[test]
        fn reports_no_match_missing_keys_keyed_non_objects_and_the_root() {
            assert_eq!(
                first_code("remove", json!({ "target": "$.nope" })).as_deref(),
                Some("FER1201")
            );
            let r = run(
                "remove",
                json!({ "target": "$.components.schemas.Pet", "keys": ["type", "nope"] }),
            );
            assert_eq!(r.diagnostics.len(), 1);
            assert_eq!(r.diagnostics[0].code, "FER1209");
            assert_eq!(r.diagnostics[0].pointer, "/openapi/transforms/0/args/keys/1");
            assert_eq!(
                first_code("remove", json!({ "target": "$.info.title", "keys": ["x"] })).as_deref(),
                Some("FER1205")
            );
            assert_eq!(
                first_code("remove", json!({ "target": "$" })).as_deref(),
                Some("FER1211")
            );
        }
    }

    mod copy {
        use super::*;

        #[test]
        fn copies_to_a_new_key_whose_parent_exists() {
            let r = run(
                "copy",
                json!({ "from": "$.components.schemas.Pet", "to": "$.components.schemas.Admin" }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let mut spec = r.spec;
            let s = at(&spec, &SCHEMAS);
            assert_eq!(at(s, &["Admin"]), at(s, &["Pet"]));
            at_mut(&mut spec, &["components", "schemas", "Admin", "required"])
                .as_array_mut()
                .unwrap()
                .push(Value::str("x"));
            assert_eq!(
                at(&spec, &["components", "schemas", "Pet", "required"]),
                &j(json!(["name"]))
            );
        }

        #[test]
        fn copies_to_several_destinations_replacing_existing_values() {
            let r = run(
                "copy",
                json!({
                    "from": "$.components.schemas.NotFound",
                    "to": [
                        "$.paths./pets/{id}.get.responses.200['content']",
                        "$.components.schemas.Error",
                        "$.paths.*.get.parameters[2]",
                        "$.paths.*.get.parameters.0",
                    ],
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let s = &r.spec;
            let nf = at(s, &["components", "schemas", "NotFound"]);
            assert_eq!(at(at(s, &OP), &["responses", "200", "content"]), nf);
            assert_eq!(at(s, &["components", "schemas", "Error"]), nf);
            let original = base();
            assert_eq!(
                at(s, &PARAMS),
                &Value::Array(vec![nf.clone(), at(at(&original, &PARAMS), &["1"]).clone(), nf.clone(),])
            );
        }

        #[test]
        fn copies_onto_existing_nodes_selected_by_wildcards_or_matches_schema() {
            let r = run(
                "copy",
                json!({
                    "from": "$.info",
                    "to": [
                        "$.components.schemas[?(@.type == \"object\" && @property == \"Error\")]",
                        { "matches_schema": "$.components.schemas.user_response" },
                    ],
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            let original = base();
            let info = at(&original, &["info"]);
            assert_eq!(at(&r.spec, &["components", "schemas", "Error"]), info);
            assert_eq!(at(&r.spec, &["components", "schemas", "user_response"]), info);
        }

        #[test]
        fn can_replace_the_root_with_an_object() {
            assert_eq!(
                &run("copy", json!({ "from": "$.info", "to": "$" })).spec,
                at(&base(), &["info"])
            );
        }

        fn d(args: serde_json::Value) -> Vec<[String; 2]> {
            code_pointers(&run("copy", args).diagnostics)
        }

        #[test]
        fn reports_a_missing_source_an_ambiguous_source_a_missing_parent_and_a_bad_root_copy() {
            const TO: &str = "/openapi/transforms/0/args/to";
            assert_eq!(
                d(json!({ "from": "$.nope", "to": "$.info.x" })),
                [["FER1201", "/openapi/transforms/0/args/from"]]
            );
            assert_eq!(
                d(json!({ "from": "$.components.schemas.*", "to": "$.info.x" })),
                [["FER1207", "/openapi/transforms/0/args/from"]]
            );
            assert_eq!(d(json!({ "from": "$.info", "to": "$.nope.x" })), [["FER1208", TO]]);
            assert_eq!(
                d(json!({ "from": "$.info", "to": "$.info.title.x" })),
                [["FER1208", TO]]
            );
            assert_eq!(
                d(json!({ "from": "$.info", "to": "$.paths.*.get.parameters.x" })),
                [["FER1208", TO]]
            );
            assert_eq!(
                d(json!({ "from": "$.info", "to": "$.paths.*.get.parameters[7]" })),
                [["FER1208", TO]]
            );
            assert_eq!(d(json!({ "from": "$.info.title", "to": "$" })), [["FER1205", TO]]);
            assert_eq!(
                d(json!({ "from": "$[", "to": ["$.info.x", "$["] })),
                [
                    ["FER1202", "/openapi/transforms/0/args/from"],
                    ["FER1202", "/openapi/transforms/0/args/to/1"],
                ]
            );
        }
    }

    mod move_ {
        use super::*;

        #[test]
        fn renames_a_key_in_place_keeping_its_position() {
            let r = run(
                "move",
                json!({
                    "from": "$.components.schemas.user_response",
                    "to": "$.components.schemas.UserResponse",
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                keys(&r.spec, &SCHEMAS),
                ["Pet", "UserResponse", "ErrorTemplate", "NotFound", "Conflict", "Error"]
            );
        }

        #[test]
        fn renames_onto_an_existing_key_replacing_it() {
            let r = run(
                "move",
                json!({ "from": "$.components.schemas.NotFound", "to": "$.components.schemas.Pet" }),
            );
            assert_eq!(
                keys(&r.spec, &SCHEMAS),
                ["user_response", "ErrorTemplate", "Pet", "Conflict", "Error"]
            );
            assert_eq!(
                at(&r.spec, &["components", "schemas", "Pet"]),
                at(&base(), &["components", "schemas", "NotFound"])
            );
        }

        #[test]
        fn relocates_between_parents() {
            let r = run(
                "move",
                json!({
                    "from": "$.components.schemas.Pet.properties.role",
                    "to": "$.components.schemas.Error.properties",
                }),
            );
            assert_eq!(r.diagnostics, no_diagnostics());
            assert!(
                at(&r.spec, &["components", "schemas", "Pet", "properties"])
                    .get("role")
                    .is_none()
            );
            assert_eq!(
                at(&r.spec, &["components", "schemas", "Error", "properties"]),
                &j(json!({ "type": "string", "default": "cat" }))
            );
        }

        #[test]
        fn reorders_within_an_array_and_appends_to_arrays() {
            let r = run("move", json!({ "from": "$..parameters[0]", "to": "$..parameters[1]" }));
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(param_names(&r.spec), ["limit", "id"]);
            let s = run(
                "move",
                json!({ "from": "$.info.title", "to": "$.components.schemas.Pet.required[1]" }),
            );
            assert_eq!(s.diagnostics, no_diagnostics());
            assert_eq!(
                at(&s.spec, &["components", "schemas", "Pet", "required"]),
                &j(json!(["name", "Pets"]))
            );
        }

        #[test]
        fn moves_a_child_onto_its_own_ancestor() {
            let r = run("move", json!({ "from": "$.info.title", "to": "$.info" }));
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(at(&r.spec, &["info"]).as_str(), Some("Pets"));
        }

        #[test]
        fn moves_an_object_onto_the_root_and_a_node_onto_itself_is_a_no_op() {
            assert_eq!(
                &run("move", json!({ "from": "$.info", "to": "$" })).spec,
                at(&base(), &["info"])
            );
            assert_eq!(run("move", json!({ "from": "$.info", "to": "$.info" })).spec, base());
        }

        fn d(args: serde_json::Value) -> Vec<[String; 2]> {
            code_pointers(&run("move", args).diagnostics)
        }

        #[test]
        fn reports_bad_sources_and_destinations() {
            const FROM: &str = "/openapi/transforms/0/args/from";
            const TO: &str = "/openapi/transforms/0/args/to";
            assert_eq!(d(json!({ "from": "$.nope", "to": "$.info.x" })), [["FER1201", FROM]]);
            assert_eq!(d(json!({ "from": "$.info", "to": "$.nope.x" })), [["FER1208", TO]]);
            assert_eq!(d(json!({ "from": "$", "to": "$.info.x" })), [["FER1211", FROM]]);
            assert_eq!(d(json!({ "from": "$.info", "to": "$.info.inner" })), [["FER1211", TO]]);
            assert_eq!(d(json!({ "from": "$.info.title", "to": "$" })), [["FER1205", TO]]);
            assert_eq!(
                d(json!({ "from": "$.info", "to": "$.components.schemas.*" })),
                [["FER1207", TO]]
            );
        }
    }
}

mod apply {
    use super::*;

    fn spec() -> Value {
        j(json!({
            "info": { "title": "T" },
            "components": { "schemas": { "A": { "type": "object" }, "B": { "type": "string" } } },
        }))
    }

    fn apply(transforms: serde_json::Value) -> ApplyTransformsResult {
        apply_transforms(&spec(), &list(transforms), None)
    }

    mod apply_transforms_ {
        use super::*;

        #[test]
        fn supports_exactly_the_commands_the_schema_defines() {
            assert_eq!(
                TRANSFORM_COMMANDS,
                ["update", "append", "merge", "remove", "copy", "move"]
            );
        }

        #[test]
        fn applies_transforms_in_order_each_seeing_the_previous_result() {
            let r = apply(json!([
                {
                    "command": "copy",
                    "reason": "r",
                    "args": { "from": "$.components.schemas.A", "to": "$.components.schemas.C" },
                },
                {
                    "command": "merge",
                    "reason": "r",
                    "args": { "target": "$.components.schemas.C", "value": { "title": "C" } },
                },
                {
                    "command": "move",
                    "reason": "r",
                    "args": { "from": "$.components.schemas.C", "to": "$.components.schemas.D" },
                },
                { "command": "remove", "reason": "r", "args": { "target": "$.components.schemas.A" } },
            ]));
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                r.spec,
                j(json!({
                    "info": { "title": "T" },
                    "components": { "schemas": { "B": { "type": "string" }, "D": { "type": "object", "title": "C" } } },
                }))
            );
        }

        #[test]
        fn order_matters_removing_before_copying_fails_the_copy() {
            let r = apply(json!([
                { "command": "remove", "reason": "r", "args": { "target": "$.components.schemas.A" } },
                {
                    "command": "copy",
                    "reason": "r",
                    "args": { "from": "$.components.schemas.A", "to": "$.components.schemas.C" },
                },
            ]));
            assert_eq!(
                code_pointers(&r.diagnostics),
                [["FER1201", "/openapi/transforms/1/args/from"]]
            );
            assert_eq!(
                r.spec,
                j(json!({
                    "info": { "title": "T" },
                    "components": { "schemas": { "B": { "type": "string" } } },
                }))
            );
        }

        #[test]
        fn skips_a_failing_transform_as_a_whole_and_carries_on() {
            let r = apply(json!([
                { "command": "update", "reason": "r", "args": { "target": ["$.info.title", "$.nope"], "value": "X" } },
                { "command": "update", "reason": "r", "args": { "target": "$.info.title", "value": "Y" } },
            ]));
            assert_eq!(r.diagnostics.len(), 1);
            let mut expected = spec();
            expected
                .as_object_mut()
                .unwrap()
                .insert("info", j(json!({ "title": "Y" })));
            assert_eq!(r.spec, expected);
        }

        #[test]
        fn uses_a_custom_pointer_base() {
            let r = apply_transforms(
                &spec(),
                &list(json!([{ "command": "remove", "reason": "r", "args": { "target": "$.x" } }])),
                Some("/openapi/transformations"),
            );
            assert_eq!(
                r.diagnostics.first().map(|d| d.pointer.as_str()),
                Some("/openapi/transformations/0/args/target")
            );
        }

        #[test]
        fn validates_entries_without_crashing() {
            let r = apply(json!([
                "nope",
                { "command": "rename", "reason": "r", "args": {} },
                { "command": "update", "reason": "r" },
                { "command": "update", "reason": "r", "args": { "target": "$.info.title" } },
                { "command": "merge", "reason": "r", "args": { "target": 3, "value": [] } },
                { "command": "copy", "args": { "from": "$.info", "to": "$.components.info" }, "extra": 1 },
                { "command": "remove", "reason": " ", "args": { "target": "$.info", "keys": ["title"], "bogus": true } },
            ]));
            assert_eq!(
                code_severity_pointers(&r.diagnostics),
                [
                    ["FER1203", "error", "/openapi/transforms/0"],
                    ["FER1203", "error", "/openapi/transforms/1/command"],
                    ["FER1203", "error", "/openapi/transforms/2/args"],
                    ["FER1203", "error", "/openapi/transforms/3/args/value"],
                    ["FER1203", "error", "/openapi/transforms/4/args/target"],
                    ["FER1203", "error", "/openapi/transforms/4/args/value"],
                    ["FER1204", "warning", "/openapi/transforms/5/extra"],
                    ["FER1204", "warning", "/openapi/transforms/5/reason"],
                    ["FER1204", "warning", "/openapi/transforms/6/reason"],
                    ["FER1204", "warning", "/openapi/transforms/6/args/bogus"],
                ]
            );
            assert!(r.diagnostics.iter().all(|d| !d.suggestion.is_empty()));
            // Warnings do not stop a transform.
            assert_eq!(
                r.spec,
                j(json!({
                    "info": {},
                    "components": {
                        "schemas": { "A": { "type": "object" }, "B": { "type": "string" } },
                        "info": { "title": "T" },
                    },
                }))
            );
        }

        #[test]
        fn is_deterministic() {
            let transforms = list(json!([
                { "command": "merge", "reason": "r", "args": { "target": "$..[?(@.type)]", "value": { "x-a": [1] } } },
                { "command": "remove", "reason": "r", "args": { "target": "$..type" } },
            ]));
            let input = spec();
            let a = apply_transforms(&input, &transforms, None);
            let b = apply_transforms(&input.clone(), &transforms.clone(), None);
            assert_eq!(stable_stringify(&a.spec), stable_stringify(&b.spec));
            assert_eq!(to_json(&a.spec), to_json(&b.spec));
            assert_eq!(a.diagnostics, b.diagnostics);
        }
    }

    mod properties {
        use super::*;

        /// An object from fixed entries, in order.
        fn obj<const N: usize>(entries: [(&str, Value); N]) -> Value {
            let mut map = Map::new();
            for (k, v) in entries {
                map.insert(k, v);
            }
            Value::Object(map)
        }

        /// `fc.dictionary(fc.constantFrom(...keys), values, { maxKeys })`.
        fn dict(
            keys: &'static [&'static str],
            values: impl Strategy<Value = Value>,
            max_keys: usize,
        ) -> impl Strategy<Value = Value> {
            prop::collection::vec((prop::sample::select(keys), values), 0..=max_keys)
                .prop_map(|entries| Value::Object(entries.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()))
        }

        /// The letrec JSON value: small leaves, arrays up to 3, dictionaries up to 3 keys.
        fn json_value() -> impl Strategy<Value = Value> {
            let leaf = prop_oneof![
                Just(Value::Null),
                Just(Value::Bool(true)),
                Just(Value::Bool(false)),
                Just(Value::Number(0.0)),
                Just(Value::Number(1.0)),
                Just(Value::str("a")),
                Just(Value::str("b")),
            ];
            leaf.prop_recursive(3, 24, 3, |inner| {
                prop_oneof![
                    prop::collection::vec(inner.clone(), 0..=3).prop_map(Value::Array),
                    dict(&["a", "b", "type", "name"], inner, 3),
                ]
            })
        }

        const PATHS: &[&str] = &[
            "$",
            "$.a",
            "$.b",
            "$..a",
            "$..type",
            "$.*",
            "$..*",
            "$..[0]",
            "$..[?(@.name == \"a\")]",
            "$.a.b",
            "$.c",
        ];

        fn path() -> impl Strategy<Value = Value> {
            prop::sample::select(PATHS).prop_map(Value::str)
        }

        fn entry(command: &str, args: Value) -> Value {
            obj([
                ("command", Value::str(command)),
                ("reason", Value::str("p")),
                ("args", args),
            ])
        }

        fn transform() -> impl Strategy<Value = Value> {
            prop_oneof![
                (path(), json_value()).prop_map(|(t, v)| entry("update", obj([("target", t), ("value", v)]))),
                (
                    prop::sample::select(&["append", "merge"][..]),
                    path(),
                    dict(&["a", "z"], json_value(), 2),
                )
                    .prop_map(|(c, t, v)| entry(c, obj([("target", t), ("value", v)]))),
                path().prop_map(|t| entry("remove", obj([("target", t)]))),
                (prop::sample::select(&["copy", "move"][..]), path(), path())
                    .prop_map(|(c, from, to)| entry(c, obj([("from", from), ("to", to)]))),
            ]
        }

        fn is_fer12_code(code: &str) -> bool {
            code.len() == 7 && code.starts_with("FER12") && code[5..].bytes().all(|b| b.is_ascii_digit())
        }

        fn is_transforms_pointer(pointer: &str) -> bool {
            pointer
                .strip_prefix("/openapi/transforms/")
                .and_then(|rest| rest.bytes().next())
                .is_some_and(|b| b.is_ascii_digit())
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn never_mutates_the_input_spec_or_the_transforms_and_is_deterministic(
                input in dict(&["a", "b", "c", "type"], json_value(), 4),
                transforms in prop::collection::vec(transform(), 0..=4),
            ) {
                let before = input.clone();
                let t_before = transforms.clone();
                let first = apply_transforms(&input, &transforms, None);
                prop_assert_eq!(&input, &before);
                prop_assert_eq!(&transforms, &t_before);
                let second = apply_transforms(&input, &transforms, None);
                prop_assert_eq!(to_json(&second.spec), to_json(&first.spec));
                prop_assert_eq!(&second.diagnostics, &first.diagnostics);
                // "Output never aliases input" has no Rust meaning: the result is owned.
            }

            #[test]
            fn diagnostics_always_carry_a_fer12xx_code_a_transforms_pointer_and_a_suggestion(
                input in dict(&["a", "b"], json_value(), 3),
                transforms in prop::collection::vec(transform(), 0..=3),
            ) {
                for d in apply_transforms(&input, &transforms, None).diagnostics {
                    prop_assert!(is_fer12_code(&d.code), "{}", d.code);
                    prop_assert!(is_transforms_pointer(&d.pointer), "{}", d.pointer);
                    prop_assert_ne!(d.suggestion.as_str(), "");
                }
            }
        }
    }

    mod apply_config_transforms_ {
        use super::*;

        fn t() -> serde_json::Value {
            json!([{ "command": "remove", "reason": "r", "args": { "target": "$.info" } }])
        }

        #[test]
        fn reads_openapi_transforms() {
            let r = apply_config_transforms(&spec(), &j(json!({ "openapi": { "transforms": t() } })));
            assert_eq!(r.diagnostics, no_diagnostics());
            assert!(r.spec.get("info").is_none());
        }

        #[test]
        fn accepts_the_deprecated_openapi_transformations_with_a_warning() {
            let r = apply_config_transforms(&spec(), &j(json!({ "openapi": { "transformations": t() } })));
            assert_eq!(
                code_severity_pointers(&r.diagnostics),
                [["FER1212", "warning", "/openapi/transformations"]]
            );
            assert!(r.spec.get("info").is_none());
        }

        #[test]
        fn prefers_transforms_when_both_are_set() {
            let r = apply_config_transforms(
                &spec(),
                &j(json!({
                    "openapi": {
                        "transforms": t(),
                        "transformations": [{ "command": "remove", "reason": "r", "args": { "target": "$.x" } }],
                    },
                })),
            );
            assert_eq!(code_severities(&r.diagnostics), [["FER1212", "error"]]);
            assert!(r.spec.get("info").is_none());
        }

        #[test]
        fn passes_the_spec_through_when_there_is_nothing_to_do_and_rejects_a_non_list() {
            let r = apply_config_transforms(&spec(), &j(json!({})));
            assert_eq!(r.spec, spec());
            assert_eq!(r.diagnostics, no_diagnostics());
            let r = apply_config_transforms(&spec(), &j(json!({ "openapi": { "code_samples": "x" } })));
            assert_eq!(r.spec, spec());
            assert_eq!(r.diagnostics, no_diagnostics());
            assert_eq!(
                apply_config_transforms(&spec(), &j(json!({ "openapi": { "transforms": null } }))).spec,
                spec()
            );
            let r = apply_config_transforms(&spec(), &j(json!({ "openapi": { "transforms": {} } })));
            assert_eq!(code_pointers(&r.diagnostics), [["FER1203", "/openapi/transforms"]]);
            assert_eq!(r.spec, spec());
        }
    }
}
