use ferrule_converter::normalize::normalize;
use ferrule_converter::swagger2::swagger2_to_openapi3;
use ferrule_converter::value::{parse_json, stable_stringify};
use ferrule_converter::{Diagnostic, Value, convert};
use serde_json::json;

// ---------------------------------------------------------------------------------------------
// Shared helpers

fn v(j: serde_json::Value) -> Value {
    Value::from(j)
}

/// Walk object keys and array indices; `None` is `undefined`.
fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |cur, seg| match cur {
        Value::Object(m) => m.get(seg),
        Value::Array(a) => seg.parse::<usize>().ok().and_then(|i| a.get(i)),
        _ => None,
    })
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

#[test]
fn rewrites_a_null_literal_in_a_type_array_to_null_imagekit() {
    let spec = v(json!({
        "components": {
            "schemas": {"F": {"properties": {"tags": {"type": ["array", null], "items": {}}}}}
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let tags = at(&out, &["components", "schemas", "F", "properties", "tags"]).expect("tags");
    assert_eq!(tags.get("type"), Some(&v(json!(["array", "null"]))));
    assert_eq!(tags.get("x-ferrule-normalized"), Some(&v(json!(["type-null-literal"]))));
    assert_eq!(
        csp(&diagnostics),
        [["FER1601", "info", "/components/schemas/F/properties/tags/type"]]
    );
    assert_eq!(
        at(&spec, &["components", "schemas", "F", "properties", "tags", "type"]),
        Some(&v(json!(["array", null])))
    );
}

/// knock-mgmt writes an optional `$ref` as `anyOf: [$ref, {nullable: true}]`. Without a `type`,
/// `nullable` constrains nothing, so Forge reads that member as any value and the whole field as
/// `object`; Stainless reads it as null (knock-mgmt-python `workflow.py`
/// `conditions: Optional[ConditionGroup]`, `email_template.py` `Optional[List[VisualBlock]]`).
#[test]
fn reads_a_member_that_is_only_nullable_as_null_knock_mgmt() {
    let member = json!({"nullable": true, "x-struct": null, "x-validate": null});
    let spec = v(json!({
        "openapi": "3.0.0",
        "components": {
            "schemas": {
                "C": {"type": "object", "properties": {"a": {"type": "string"}}},
                "W": {"properties": {
                    "conditions": {
                        "anyOf": [{"$ref": "#/components/schemas/C"}, member],
                        "description": "Conditions.",
                        "type": "object"
                    },
                    "settings": {"oneOf": [{"type": "object", "properties": {}}, member]},
                    "kept": {"anyOf": [{"type": "string"}, {"nullable": true, "type": "integer"}]}
                }}
            }
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let props = at(&out, &["components", "schemas", "W", "properties"]).expect("properties");
    let null = v(json!({
        "type": "null",
        "x-struct": null,
        "x-validate": null,
        "x-ferrule-normalized": ["nullable-only-member"]
    }));
    assert_eq!(at(props, &["conditions", "anyOf", "1"]), Some(&null));
    assert_eq!(at(props, &["settings", "oneOf", "1"]), Some(&null));
    assert_eq!(
        at(props, &["conditions", "anyOf", "0"]),
        Some(&v(json!({"$ref": "#/components/schemas/C"})))
    );
    assert_eq!(
        at(props, &["kept"]),
        at(&spec, &["components", "schemas", "W", "properties", "kept"])
    );
    assert_eq!(
        csp(&diagnostics),
        [
            ["FER1608", "info", "/components/schemas/W/properties/conditions/anyOf/1"],
            ["FER1608", "info", "/components/schemas/W/properties/settings/oneOf/1"],
        ]
    );
}

/// Stainless names an inline union member after its `title` (knock-mgmt-python
/// `GuideBooleanActivationParams`, `RecipientObjectRecipientReference`,
/// `HeadersRequestTemplateHeadersArray`; imagekit-python `UpdateFileDetails`), which Forge does
/// not expose for inline schemas. The member moves to `components.schemas` under its title in
/// words (imagekit `Google Cloud Storage (GCS)` is `GoogleCloudStorageGcs`), so the plugin sees it as the
/// schema name, or becomes a `$ref` to an equal schema already there. Only object members and
/// arrays of objects move, and only from unions without a discriminator: with one, Stainless
/// names variants after the discriminator value (imagekit `origin_response.py` `Gcs`).
#[test]
fn moves_titled_inline_union_members_to_components_knock_mgmt() {
    let object = |title: &str| json!({"title": title, "description": "One.", "type": "object", "properties": {"a": {"type": "string"}}});
    let array = json!({
        "title": "HeadersArray",
        "type": "array",
        "items": {"type": "object", "properties": {"k": {"type": "string"}}}
    });
    let spec = v(json!({
        "openapi": "3.0.0",
        "components": {
            "schemas": {
                "HeadersArray": array.clone(),
                "Taken": {"type": "string"},
                "P": {"oneOf": [object("Google Cloud Storage (GCS)"), {"title": "S", "type": "string"}]},
                "R": {"properties": {
                    "headers": {"oneOf": [{"type": "string"}, array]},
                    "taken": {"anyOf": [object("Taken"), {"type": "string"}]},
                    "tagged": {
                        "oneOf": [object("Update file details")],
                        "discriminator": {"propertyName": "a"}
                    }
                }}
            }
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let schemas = at(&out, &["components", "schemas"]).expect("schemas");
    let reference = |name: &str| {
        v(json!({"$ref": format!("#/components/schemas/{name}"), "x-ferrule-normalized": ["title-hoisted"]}))
    };
    assert_eq!(
        at(schemas, &["P", "oneOf", "0"]),
        Some(&reference("GoogleCloudStorageGcs"))
    );
    assert_eq!(
        at(schemas, &["GoogleCloudStorageGcs"]),
        Some(&v(object("Google Cloud Storage (GCS)")))
    );
    assert_eq!(
        at(schemas, &["P", "oneOf", "1"]),
        Some(&v(json!({"title": "S", "type": "string"})))
    );
    assert_eq!(
        at(schemas, &["R", "properties", "headers", "oneOf", "1"]),
        Some(&reference("HeadersArray"))
    );
    assert_eq!(
        at(schemas, &["R", "properties", "taken", "anyOf", "0"]),
        Some(&v(object("Taken")))
    );
    assert_eq!(
        at(schemas, &["R", "properties", "tagged", "oneOf", "0"]),
        Some(&v(object("Update file details")))
    );
    assert_eq!(at(schemas, &["UpdateFileDetails"]), None);
    assert_eq!(
        csp(&diagnostics),
        [
            ["FER1609", "info", "/components/schemas/P/oneOf/0"],
            ["FER1609", "info", "/components/schemas/R/properties/headers/oneOf/1"],
        ]
    );
}

#[test]
fn rewrites_a_string_discriminator_to_a_discriminator_object_knock() {
    let spec = v(json!({"components": {"schemas": {"R": {"oneOf": [], "discriminator": "__typename"}}}}));
    let (out, diagnostics) = normalize(&spec);
    let r = at(&out, &["components", "schemas", "R"]).expect("R");
    assert_eq!(r.get("discriminator"), Some(&v(json!({"propertyName": "__typename"}))));
    assert_eq!(r.get("x-ferrule-normalized"), Some(&v(json!(["discriminator-string"]))));
    assert_eq!(cp(&diagnostics), [["FER1602", "/components/schemas/R/discriminator"]]);
}

#[test]
fn records_several_rules_on_one_node_and_leaves_data_and_extensions_alone() {
    let node_json = json!({
        "type": [null, "object"],
        "discriminator": "kind",
        "example": {"type": ["x", null], "discriminator": "y"},
        "enum": [{"type": [null]}],
        "x-ext": {"discriminator": "z"},
        "properties": {"type": {"type": "string"}, "discriminator": {"type": "string"}}
    });
    let spec = v(json!({"components": {"schemas": {"N": node_json}}}));
    let node = v(node_json);
    let (out, diagnostics) = normalize(&spec);
    let n = at(&out, &["components", "schemas", "N"]).expect("N");
    assert_eq!(
        n.get("x-ferrule-normalized"),
        Some(&v(json!(["type-null-literal", "discriminator-string"])))
    );
    assert_eq!(n.get("example"), node.get("example"));
    assert_eq!(n.get("enum"), node.get("enum"));
    assert_eq!(n.get("x-ext"), node.get("x-ext"));
    assert_eq!(n.get("properties"), node.get("properties"));
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn is_the_identity_on_a_spec_with_nothing_to_rewrite() {
    let spec = v(json!({"openapi": "3.1.0", "paths": {"/a": {"get": {"tags": ["x"]}}}, "list": [1, "a"]}));
    assert_eq!(normalize(&spec), (spec.clone(), Vec::<Diagnostic>::new()));
}

#[test]
fn runs_inside_convert_before_annotation() {
    let spec = v(json!({"openapi": "3.1.0", "components": {"schemas": {"R": {"discriminator": "k"}}}}));
    let result = convert(&spec, &Value::object());
    assert_eq!(
        at(&result.spec, &["components", "schemas", "R", "discriminator"]),
        Some(&v(json!({"propertyName": "k"})))
    );
    assert!(result.diagnostics.iter().any(|d| d.code == "FER1602"));
}

// Reduction of a fast-check counterexample: `{"":{"__proto__":{}}}`.
#[test]
fn keeps_a_proto_key_as_data() {
    let spec = parse_json(r#"{"":{"__proto__":{"a":1}}}"#).expect("json");
    let (out, _) = normalize(&spec);
    let keys: Vec<String> = at(&out, &[""])
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    assert_eq!(keys, ["__proto__"]);
    assert_eq!(stable_stringify(&out), stable_stringify(&spec));
    assert_eq!(parse_json(&stable_stringify(&spec)), Some(spec));
}

#[test]
fn drops_a_query_parameter_style_forge_does_not_cover_stripe_openai() {
    let spec = v(json!({
        "paths": {
            "/v1/webhook_endpoints/{id}": {
                "get": {
                    "parameters": [
                        {"name": "id", "in": "path", "required": true, "schema": {"type": "string"}},
                        {
                            "name": "expand",
                            "in": "query",
                            "style": "deepObject",
                            "explode": true,
                            "schema": {"type": "array"}
                        },
                        {"name": "tags", "in": "query", "style": "form", "schema": {"type": "array"}}
                    ]
                }
            }
        },
        "components": {
            "parameters": {"metadata": {"name": "metadata", "in": "query", "style": "deepObject"}}
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let params = ["paths", "/v1/webhook_endpoints/{id}", "get", "parameters"];
    let param = |root: &'_ Value, i: &str| -> Option<Value> {
        let mut p = params.to_vec();
        p.push(i);
        at(root, &p).cloned()
    };
    assert_eq!(
        param(&out, "1"),
        Some(v(json!({
            "name": "expand",
            "in": "query",
            "explode": true,
            "schema": {"type": "array"},
            "x-ferrule-normalized": ["query-style-deepObject"]
        })))
    );
    assert_eq!(param(&out, "2"), param(&spec, "2"));
    assert_eq!(at(&out, &["components", "parameters", "metadata", "style"]), None);
    assert_eq!(
        cp(&diagnostics),
        [
            ["FER1603", "/paths/~1v1~1webhook_endpoints~1{id}/get/parameters/1/style"],
            ["FER1603", "/components/parameters/metadata/style"],
        ]
    );
}

#[test]
fn spells_out_symbols_in_properties_whose_names_forge_finds_equal_github() {
    let spec = v(json!({
        "components": {
            "schemas": {
                "reactions": {
                    "properties": {"+1": {"type": "integer"}, "-1": {"type": "integer"}, "laugh": {}}
                }
            }
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let props = at(&out, &["components", "schemas", "reactions", "properties"]).expect("props");
    assert_eq!(
        props.get("+1"),
        Some(&v(json!({
            "type": "integer",
            "x-fern-property-name": "plus_1",
            "x-ferrule-normalized": ["equal-names"]
        })))
    );
    assert_eq!(
        at(props, &["-1", "x-fern-property-name"]),
        Some(&Value::from("minus_1"))
    );
    assert_eq!(props.get("laugh"), Some(&Value::object()));
    assert_eq!(
        csp(&diagnostics),
        [
            ["FER1604", "info", "/components/schemas/reactions/properties/+1"],
            ["FER1604", "info", "/components/schemas/reactions/properties/-1"],
        ]
    );
}

#[test]
fn spells_out_comparison_suffixes_on_parameters_forge_finds_equal_twilio() {
    let spec = v(json!({
        "paths": {
            "/Recordings.json": {
                "parameters": [{"name": "DateCreated", "in": "query", "schema": {"type": "string"}}],
                "get": {
                    "parameters": [
                        {"$ref": "#/components/parameters/lt"},
                        {"name": "DateCreated>", "in": "query", "schema": {"type": "string"}},
                        {"name": "date-created", "in": "header", "schema": {"type": "string"}}
                    ]
                }
            }
        },
        "components": {
            "parameters": {"lt": {"name": "DateCreated<", "in": "query", "schema": {"type": "string"}}}
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let item = at(&out, &["paths", "/Recordings.json"]).expect("item");
    assert_eq!(at(item, &["parameters", "0", "x-fern-parameter-name"]), None);
    assert_eq!(
        at(item, &["get", "parameters", "0"]),
        Some(&v(json!({
            "name": "DateCreated<",
            "in": "query",
            "schema": {"type": "string"},
            "x-fern-parameter-name": "date_created_lt",
            "x-ferrule-normalized": ["equal-names"]
        })))
    );
    assert_eq!(
        at(item, &["get", "parameters", "1", "x-fern-parameter-name"]),
        Some(&Value::from("date_created_gt"))
    );
    assert_eq!(at(item, &["get", "parameters", "2", "x-fern-parameter-name"]), None);
    assert_eq!(
        at(&out, &["components", "parameters", "lt", "x-fern-parameter-name"]),
        None
    );
    let pointers: Vec<&str> = diagnostics.iter().map(|d| d.pointer.as_str()).collect();
    assert_eq!(
        pointers,
        [
            "/paths/~1Recordings.json/get/parameters/0",
            "/paths/~1Recordings.json/get/parameters/1",
        ]
    );
}

#[test]
fn numbers_a_field_that_differs_from_another_only_by_case_with_a_warning_cloudflare() {
    let spec = v(json!({
        "components": {
            "schemas": {
                "hook": {
                    "properties": {
                        "notificationUrl": {"type": "string"},
                        "notification_url": {"type": "string"}
                    }
                }
            }
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let props = at(&out, &["components", "schemas", "hook", "properties"]).expect("props");
    assert_eq!(at(props, &["notification_url", "x-fern-property-name"]), None);
    assert_eq!(
        at(props, &["notificationUrl", "x-fern-property-name"]),
        Some(&Value::from("notification_url_2"))
    );
    let code_severity: Vec<[String; 2]> = diagnostics
        .iter()
        .map(|d| [d.code.clone(), d.severity.to_string()])
        .collect();
    assert_eq!(code_severity, [["FER1605", "warning"]]);
}

#[test]
fn replaces_a_ref_to_a_missing_schema_with_an_unknown_schema_with_a_warning_square() {
    let spec = v(json!({
        "components": {
            "schemas": {
                "Refund": {
                    "properties": {
                        "app_fee": {"type": "array", "items": {"$ref": "#/components/schemas/AppFeeAllocation"}},
                        "money": {"$ref": "#/components/schemas/Money"}
                    }
                },
                "Money": {"type": "object"}
            }
        }
    }));
    let (out, diagnostics) = normalize(&spec);
    let props = at(&out, &["components", "schemas", "Refund", "properties"]).expect("props");
    assert_eq!(
        at(props, &["app_fee", "items"]),
        Some(&v(json!({"x-ferrule-normalized": ["dangling-ref"]})))
    );
    assert_eq!(
        props.get("money"),
        Some(&v(json!({"$ref": "#/components/schemas/Money"})))
    );
    assert_eq!(
        csp(&diagnostics),
        [[
            "FER1606",
            "warning",
            "/components/schemas/Refund/properties/app_fee/items/$ref"
        ]]
    );
}

#[test]
fn rewrites_a_list_valued_items_to_prefix_items_which_forge_reads_slack() {
    let latest_items = json!([{"$ref": "#/components/schemas/M"}, {"type": "null"}]);
    let spec = v(json!({
        "components": {"schemas": {"M": {}, "C": {"properties": {"latest": {"items": latest_items}}}}}
    }));
    let (out, diagnostics) = normalize(&spec);
    assert_eq!(
        at(&out, &["components", "schemas", "C", "properties", "latest"]),
        Some(&v(json!({
            "prefixItems": latest_items,
            "x-ferrule-normalized": ["items-tuple"]
        })))
    );
    assert_eq!(
        cp(&diagnostics),
        [["FER1607", "/components/schemas/C/properties/latest/items"]]
    );
}

#[test]
fn treats_property_names_that_are_keywords_as_names_not_data_kubernetes() {
    let spec = v(json!({
        "components": {
            "schemas": {
                "L": {
                    "properties": {
                        "default": {"additionalProperties": {"$ref": "#/components/schemas/Gone"}}
                    }
                }
            }
        }
    }));
    let (_, diagnostics) = normalize(&spec);
    assert_eq!(
        cp(&diagnostics),
        [[
            "FER1606",
            "/components/schemas/L/properties/default/additionalProperties/$ref"
        ]]
    );
}

fn docker() -> Value {
    v(json!({
        "swagger": "2.0",
        "host": "localhost",
        "basePath": "/v1.41",
        "schemes": ["http", "https"],
        "consumes": ["application/json"],
        "produces": ["application/json"],
        "definitions": {
            "ContainerConfig": {
                "type": "object",
                "properties": {"Image": {"type": "string", "x-nullable": true}}
            },
            "Container": {"allOf": [{"$ref": "#/definitions/ContainerConfig"}], "discriminator": "Kind"}
        },
        "parameters": {
            "limit": {"name": "limit", "in": "query", "type": "integer", "default": 10},
            "config": {
                "name": "config",
                "in": "body",
                "required": true,
                "schema": {"$ref": "#/definitions/ContainerConfig"}
            }
        },
        "responses": {"NotFound": {"description": "no such thing", "schema": {"type": "string"}}},
        "securityDefinitions": {"key": {"type": "apiKey", "in": "header", "name": "X-Key"}},
        "paths": {
            "/containers/create": {
                "parameters": [{"$ref": "#/parameters/limit"}],
                "post": {
                    "operationId": "ContainerCreate",
                    "parameters": [
                        {"name": "name", "in": "query", "type": "string", "pattern": "^/?[a-z]+$"},
                        {
                            "name": "ids",
                            "in": "query",
                            "type": "array",
                            "items": {"type": "string"},
                            "collectionFormat": "csv"
                        },
                        {"name": "X-Trace", "in": "header", "type": "string"},
                        {"$ref": "#/parameters/config"}
                    ],
                    "responses": {
                        "201": {
                            "description": "created",
                            "schema": {"$ref": "#/definitions/Container"},
                            "headers": {"X-Id": {"type": "string"}}
                        },
                        "204": {"description": "empty"},
                        "404": {"$ref": "#/responses/NotFound"}
                    }
                }
            }
        }
    }))
}

fn upgrade(spec: &Value) -> (Value, Vec<Diagnostic>) {
    swagger2_to_openapi3(spec).expect("a Swagger 2 document")
}

#[test]
fn converts_the_document_skeleton_and_components_docker() {
    let docker = docker();
    let (out, diagnostics) = upgrade(&docker);
    assert_eq!(out.get("swagger"), None);
    assert_eq!(out.get("openapi"), Some(&Value::from("3.0.3")));
    assert_eq!(
        out.get("servers"),
        Some(&v(json!([
            {"url": "http://localhost/v1.41"},
            {"url": "https://localhost/v1.41"}
        ])))
    );
    assert_eq!(
        at(
            &out,
            &["components", "schemas", "ContainerConfig", "properties", "Image"]
        ),
        Some(&v(json!({"type": "string", "nullable": true})))
    );
    assert_eq!(
        at(&out, &["components", "schemas", "Container"]),
        Some(&v(json!({
            "allOf": [{"$ref": "#/components/schemas/ContainerConfig"}],
            "discriminator": {"propertyName": "Kind"}
        })))
    );
    assert_eq!(
        at(&out, &["components", "parameters"]),
        Some(&v(json!({
            "limit": {"name": "limit", "in": "query", "schema": {"type": "integer", "default": 10}}
        })))
    );
    assert_eq!(
        at(&out, &["components", "responses", "NotFound"]),
        Some(&v(json!({
            "description": "no such thing",
            "content": {"application/json": {"schema": {"type": "string"}}}
        })))
    );
    assert_eq!(
        at(&out, &["components", "securitySchemes"]),
        Some(&v(json!({"key": {"type": "apiKey", "in": "header", "name": "X-Key"}})))
    );
    assert_eq!(out.get("x-ferrule-normalized"), Some(&v(json!(["swagger2"]))));
    assert_eq!(csp(&diagnostics), [["FER1610", "info", "/swagger"]]);
    assert_eq!(
        at(
            &docker,
            &["definitions", "ContainerConfig", "properties", "Image", "x-nullable"]
        ),
        Some(&Value::Bool(true))
    );
}

#[test]
fn turns_body_query_and_header_parameters_into_openapi_3_shapes_docker() {
    let (out, _) = upgrade(&docker());
    let op = at(&out, &["paths", "/containers/create", "post"]).expect("op");
    assert_eq!(
        op.get("parameters"),
        Some(&v(json!([
            {"name": "name", "in": "query", "schema": {"type": "string", "pattern": "^/?[a-z]+$"}},
            {
                "name": "ids",
                "in": "query",
                "style": "form",
                "explode": false,
                "schema": {"type": "array", "items": {"type": "string"}}
            },
            {"name": "X-Trace", "in": "header", "schema": {"type": "string"}}
        ])))
    );
    assert_eq!(
        op.get("requestBody"),
        Some(&v(json!({
            "required": true,
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/ContainerConfig"}}},
            "x-ferrule-normalized": ["swagger2-body"]
        })))
    );
    assert_eq!(
        at(op, &["responses", "201"]),
        Some(&v(json!({
            "description": "created",
            "headers": {"X-Id": {"schema": {"type": "string"}}},
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Container"}}}
        })))
    );
    assert_eq!(at(op, &["responses", "204"]), Some(&v(json!({"description": "empty"}))));
    assert_eq!(
        at(op, &["responses", "404"]),
        Some(&v(json!({"$ref": "#/components/responses/NotFound"})))
    );
    let (again, _) = upgrade(&docker());
    assert_eq!(
        at(&again, &["paths", "/containers/create", "parameters"]),
        Some(&v(json!([{"$ref": "#/components/parameters/limit"}])))
    );
}

fn slack() -> Value {
    v(json!({
        "swagger": "2.0",
        "paths": {
            "/files.upload": {
                "post": {
                    "consumes": ["application/x-www-form-urlencoded", "multipart/form-data"],
                    "parameters": [
                        {
                            "name": "token",
                            "in": "formData",
                            "type": "string",
                            "required": true,
                            "description": "Auth token"
                        },
                        {"name": "file", "in": "formData", "type": "file"},
                        {"name": "channels", "in": "formData", "type": "string"}
                    ],
                    "responses": {"200": {"description": "ok", "schema": {"type": "object"}}}
                }
            },
            "/chat.postMessage": {
                "post": {
                    "consumes": ["application/x-www-form-urlencoded"],
                    "produces": ["application/json"],
                    "parameters": [{"name": "text", "in": "formData", "type": "string"}],
                    "responses": {"default": {"description": "ok"}}
                }
            }
        }
    }))
}

#[test]
fn collects_form_data_parameters_into_a_form_request_body_slack() {
    let (out, _) = upgrade(&slack());
    let paths = out.get("paths").expect("paths");
    let upload = at(paths, &["/files.upload", "post"]).expect("upload");
    assert_eq!(upload.get("parameters"), None);
    assert_eq!(
        upload.get("requestBody"),
        Some(&v(json!({
            "required": true,
            "content": {
                "multipart/form-data": {
                    "schema": {
                        "type": "object",
                        "required": ["token"],
                        "properties": {
                            "token": {"type": "string", "description": "Auth token"},
                            "file": {"type": "string", "format": "binary"},
                            "channels": {"type": "string"}
                        }
                    }
                }
            },
            "x-ferrule-normalized": ["swagger2-form-data"]
        })))
    );
    let post = at(paths, &["/chat.postMessage", "post"]).expect("post");
    let content_types: Vec<String> = at(post, &["requestBody", "content"])
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    assert_eq!(content_types, ["application/x-www-form-urlencoded"]);
    assert_eq!(at(post, &["requestBody", "required"]), None);
}

#[test]
fn rewrites_refs_under_properties_named_like_keywords_kubernetes() {
    let k8s = v(json!({
        "swagger": "2.0",
        "definitions": {
            "Q": {"type": "string"},
            "L": {
                "properties": {
                    "default": {"additionalProperties": {"$ref": "#/definitions/Q"}},
                    "example": {"$ref": "#/definitions/Q"}
                }
            }
        }
    }));
    let (out, _) = upgrade(&k8s);
    let props = at(&out, &["components", "schemas", "L", "properties"]).expect("props");
    assert_eq!(
        at(props, &["default", "additionalProperties", "$ref"]),
        Some(&Value::from("#/components/schemas/Q"))
    );
    assert_eq!(
        at(props, &["example", "$ref"]),
        Some(&Value::from("#/components/schemas/Q"))
    );
}

#[test]
fn is_the_identity_on_an_openapi_3_document() {
    let spec = v(json!({"openapi": "3.1.0", "paths": {}}));
    assert_eq!(swagger2_to_openapi3(&spec), None);
}

#[test]
fn runs_inside_convert_before_openapi_transforms() {
    let config = v(json!({
        "openapi": {
            "transforms": [
                {"command": "remove", "reason": "test", "args": {"target": "$.components.schemas.Container"}}
            ]
        }
    }));
    let result = convert(&docker(), &config);
    assert_eq!(at(&result.spec, &["components", "schemas", "Container"]), None);
    assert_eq!(result.spec.get("openapi"), Some(&Value::from("3.0.3")));
}
