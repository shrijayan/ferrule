use ferrule_converter::diagnostic::{Seg, to_json_pointer};
use ferrule_converter::schema::json_schema::{
    ErrorSeverity, Keyword, deep_equal, describe_alternatives, json_type, pattern_compiles, resolve_ref,
    unsupported_keywords, validate,
};
use ferrule_converter::schema::stainless_config_schema;
use ferrule_converter::schema::validate::{ConfigValidation, NOT_ALLOWED, validate_config, validate_config_value};
use ferrule_converter::{Diagnostic, Severity, Value, convert};
use proptest::prelude::*;
use serde_json::{Value as Json, json};

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

const BASE: &str = "organization:
  name: acme
environments:
  production: https://api.acme.test
resources: {}
readme:
  example_requests:
    default: { type: request, endpoint: get /x, params: {} }
    headline: { type: request, endpoint: get /x, params: {} }
";

fn vc(text: &str) -> ConfigValidation {
    validate_config(text).expect("validateConfig does not throw")
}

/// `[code, severity, pointer, message]` of every diagnostic.
fn brief(text: &str) -> Vec<(String, String, String, String)> {
    vc(text)
        .diagnostics
        .iter()
        .map(|d| {
            (
                d.code.clone(),
                d.severity.to_string(),
                d.pointer.clone(),
                d.message.clone(),
            )
        })
        .collect()
}

fn diag(code: &str, severity: Severity, message: &str, pointer: &str, suggestion: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity,
        message: message.to_owned(),
        pointer: pointer.to_owned(),
        suggestion: suggestion.to_owned(),
    }
}

fn organization_name(config: Option<&Value>) -> Option<&str> {
    config?.get("organization")?.get("name")?.as_str()
}

/// `/line \d+, column \d+/`.
fn has_line_column(message: &str) -> bool {
    message.match_indices("line ").any(|(i, _)| {
        let rest = &message[i + "line ".len()..];
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        digits > 0
            && rest[digits..]
                .strip_prefix(", column ")
                .is_some_and(|tail| tail.starts_with(|c: char| c.is_ascii_digit()))
    })
}

/// Whether `parts` occur in `haystack` in order (a regex `a.*b.*c`).
fn contains_in_order(haystack: &str, parts: &[&str]) -> bool {
    let mut rest = haystack;
    for part in parts {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    true
}

fn v(json: &Json) -> Value {
    Value::from(json)
}

/// `[keyword, path joined with "/"]` of every validation error.
fn keywords(schema: &Json, value: &Json) -> Vec<(&'static str, String)> {
    let schema = v(schema);
    validate(&schema, &v(value))
        .iter()
        .map(|e| {
            (
                e.keyword.name(),
                e.path.iter().map(ToString::to_string).collect::<Vec<_>>().join("/"),
            )
        })
        .collect()
}

fn kw(expected: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
    expected.iter().map(|(k, p)| (*k, (*p).to_owned())).collect()
}

fn repo_root() -> String {
    format!("{}/../../", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn accepts_a_minimal_config_and_returns_the_typed_config() {
    let r = vc(BASE);
    assert_eq!(r.diagnostics, Vec::<Diagnostic>::new());
    assert_eq!(organization_name(r.config.as_ref()), Some("acme"));
    let production = r
        .value
        .as_ref()
        .and_then(|value| value.get("environments"))
        .and_then(|e| e.get("production"))
        .and_then(Value::as_str);
    assert_eq!(production, Some("https://api.acme.test"));
}

#[test]
fn reports_yaml_syntax_errors_as_fer1100_with_line_and_column() {
    let r = vc("organization:\n  name: [unclosed\n");
    assert_eq!(r.config, None);
    assert_eq!(r.value, None);
    let first = &r.diagnostics[0];
    assert_eq!(first.code, "FER1100");
    assert_eq!(first.severity, Severity::Error);
    assert_eq!(first.pointer, "");
    assert!(has_line_column(&first.message), "{}", first.message);
}

#[test]
fn reports_duplicate_keys_as_fer1100() {
    assert_eq!(brief(&format!("{BASE}readme: {{}}\n"))[0].0, "FER1100");
}

#[test]
fn reports_missing_required_keys_as_fer1111_at_the_parent_with_a_hint() {
    let r = vc("organization:\n  name: acme\nresources: {}\n");
    let got: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.as_str(), d.pointer.as_str(), d.message.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (
                "FER1111",
                "",
                "config: missing required key \"environments\" (line 1, column 1)"
            ),
            (
                "FER1111",
                "",
                "config: missing required key \"readme\" (line 1, column 1)"
            ),
        ]
    );
    assert!(
        r.diagnostics[0]
            .suggestion
            .starts_with("Add `environments` (Map of the name"),
        "{}",
        r.diagnostics[0].suggestion
    );
}

#[test]
fn treats_an_empty_document_as_an_empty_config() {
    let codes: Vec<_> = brief("").into_iter().map(|d| d.0).collect();
    assert_eq!(codes, vec!["FER1111", "FER1111", "FER1111", "FER1111"]);
}

#[test]
fn reports_type_mismatches_as_fer1110_at_the_value_position() {
    let r = vc(&BASE.replacen("name: acme", "name: 42", 1));
    assert_eq!(
        r.diagnostics,
        vec![diag(
            "FER1110",
            Severity::Error,
            "organization.name: expected string, got integer (line 2, column 9)",
            "/organization/name",
            "Change the value to a string.",
        )]
    );
}

#[test]
fn reports_a_non_mapping_root_as_fer1110() {
    assert_eq!(
        brief("- a\n- b\n"),
        vec![(
            "FER1110".to_owned(),
            "error".to_owned(),
            String::new(),
            "config: expected object, got array (line 1, column 1)".to_owned(),
        )]
    );
}

#[test]
fn reports_enum_misses_as_fer1113_with_a_did_you_mean() {
    let r = vc(&format!("{BASE}query_settings:\n  array_format: repeet\n"));
    assert_eq!(r.diagnostics.len(), 1);
    let first = &r.diagnostics[0];
    assert_eq!(first.code, NOT_ALLOWED);
    assert_eq!(first.pointer, "/query_settings/array_format");
    assert_eq!(first.suggestion, "Did you mean \"repeat\"?");
    assert!(first.message.contains("(line 11, column 17)"), "{}", first.message);
}

#[test]
fn lists_the_allowed_values_when_no_enum_value_is_close() {
    let r = vc(&format!("{BASE}query_settings:\n  array_format: zzzzzzzzzzzz\n"));
    assert!(
        r.diagnostics[0].suggestion.starts_with("Use one of \"comma\", "),
        "{}",
        r.diagnostics[0].suggestion
    );
}

#[test]
fn reports_string_length_and_pattern_failures_as_fer1114() {
    let r = vc(&format!(
        "{BASE}targets:\n  python:\n    production_repo: not a repo\n  java:\n    reverse_domain: Org.Acme\n"
    ));
    let codes: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.as_str(), d.pointer.as_str()))
        .collect();
    assert!(
        codes.contains(&("FER1114", "/targets/java/reverse_domain")),
        "{codes:?}"
    );
    let first = brief(&BASE.replacen("name: acme", "name: ''", 1)).remove(0);
    assert_eq!(
        (first.0.as_str(), first.1.as_str(), first.2.as_str()),
        ("FER1114", "error", "/organization/name")
    );
}

#[test]
fn reports_numeric_range_failures_as_fer1115() {
    let r = vc(&format!(
        "{BASE}client_settings:\n  default_retries:\n    max_retries: -1\n    initial_delay_seconds: 1\n    max_delay_seconds: 2\n"
    ));
    let got: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.as_str(), d.pointer.as_str(), d.suggestion.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![(
            "FER1115",
            "/client_settings/default_retries/max_retries",
            "Use a number inside the allowed range."
        )]
    );
}

#[test]
fn reports_bad_environment_names_as_fer1117_at_the_key() {
    let long = "e".repeat(257);
    let r = vc(&BASE.replacen("production:", &format!("{long}:"), 1));
    let got: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.pointer.clone()))
        .collect();
    assert_eq!(got, vec![("FER1117".to_owned(), format!("/environments/{long}"))]);
    assert!(
        r.diagnostics[0].message.contains("(line 4, column 3)"),
        "{}",
        r.diagnostics[0].message
    );
}

#[test]
fn reports_unknown_keys_under_additional_properties_false_as_fer1112() {
    let tf = |body: &str| format!("{BASE}targets:\n  terraform:\n    options:\n      data_source:\n{body}");
    let r = vc(&tf(
        "        filtering: {}\n        response_items: {}\n        filterng: {}\n",
    ));
    let unknown: Vec<_> = r.diagnostics.into_iter().filter(|d| d.code == "FER1112").collect();
    assert_eq!(
        unknown,
        vec![diag(
            "FER1112",
            Severity::Error,
            "targets.terraform.options.data_source: unknown key \"filterng\" (line 16, column 9)",
            "/targets/terraform/options/data_source/filterng",
            "Did you mean `filtering`?",
        )]
    );
    let far = vc(&tf(
        "        filtering: {}\n        response_items: {}\n        zzzzzzzzzz: {}\n",
    ));
    assert_eq!(
        far.diagnostics
            .iter()
            .find(|d| d.code == "FER1112")
            .map(|d| d.suggestion.as_str()),
        Some("Remove `zzzzzzzzzz`; allowed keys are `filtering`, `response_items`.")
    );
}

#[test]
fn warns_about_deprecated_keys_with_fer1118_and_still_returns_the_config() {
    let r = vc(&format!("{BASE}settings:\n  disable_tests: true\n"));
    assert_eq!(
        r.diagnostics,
        vec![diag(
            "FER1118",
            Severity::Warning,
            "settings.disable_tests: \"disable_tests\" is deprecated (line 11, column 3)",
            "/settings/disable_tests",
            "Rename `disable_tests` to `disable_mock_tests`.",
        )]
    );
    assert!(r.config.is_some());
}

#[test]
fn picks_the_intended_any_of_branch_for_method_objects() {
    let r = vc(&BASE.replacen(
        "resources: {}",
        "resources:\n  users:\n    methods:\n      list:\n        paginated: false\n",
        1,
    ));
    let got: Vec<_> = r
        .diagnostics
        .iter()
        .map(|d| (d.code.as_str(), d.pointer.as_str(), d.message.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![(
            "FER1111",
            "/resources/users/methods/list",
            "resources.users.methods.list: missing required key \"endpoint\" (line 9, column 9)",
        )]
    );
}

#[test]
fn reports_values_that_match_no_alternative_as_fer1116() {
    let r = vc(&BASE.replacen(
        "resources: {}",
        "resources:\n  users:\n    methods:\n      list: 3\n",
        1,
    ));
    assert_eq!(r.diagnostics.len(), 1);
    let first = &r.diagnostics[0];
    assert_eq!(first.code, "FER1116");
    assert_eq!(first.pointer, "/resources/users/methods/list");
    assert!(
        contains_in_order(&first.message, &["expected a string or ", "an object", ", got 3"]),
        "{}",
        first.message
    );
}

#[test]
fn follows_yaml_aliases_when_locating_errors() {
    let text = BASE
        .replacen("organization:", "organization: &org", 1)
        .replacen("name: acme", "name: 1", 1)
        + "settings:\n  x: *org\n";
    let r = vc(&text);
    let pointers: Vec<_> = r.diagnostics.iter().map(|d| d.pointer.as_str()).collect();
    assert_eq!(pointers, vec!["/organization/name"]);
}

#[test]
fn locates_errors_inside_sequences() {
    let r = vc(&format!("{BASE}pagination:\n  - name: p\n    type: 1\n"));
    let found = r.diagnostics.iter().find(|x| x.pointer == "/pagination/0/type");
    let message = found.map_or("", |d| d.message.as_str());
    assert!(message.contains("(line 12, column 11)"), "{message}");
}

#[test]
fn reads_endpointless_methods_such_as_webhook_unwrap() {
    let r = vc(&BASE.replacen(
        "resources: {}",
        "resources: { a: { methods: { hook: { type: webhook_unwrap } } } }",
        1,
    ));
    assert_eq!(r.diagnostics, Vec::<Diagnostic>::new());
    assert!(r.config.is_some());
}

#[test]
fn accepts_legacy_one_of_to_any_of_in_the_deprecated_transformations_list_only() {
    let legacy = |key: &str| {
        vc(&format!(
            "{BASE}openapi:\n  {key}:\n    - {{ command: oneOfToAnyOf, reason: r }}\n"
        ))
    };
    let got: Vec<_> = legacy("transformations")
        .diagnostics
        .into_iter()
        .map(|d| (d.code, d.severity))
        .collect();
    assert_eq!(got, vec![("FER1118".to_owned(), Severity::Warning)]);
    assert!(
        legacy("transforms")
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
}

#[test]
fn validates_parsed_values_without_positions() {
    let messages: Vec<_> = validate_config_value(&v(&json!({ "organization": { "name": "a" } })))
        .into_iter()
        .map(|d| d.message)
        .collect();
    assert_eq!(
        messages,
        vec![
            "config: missing required key \"environments\"",
            "config: missing required key \"resources\"",
            "config: missing required key \"readme\"",
        ]
    );
}

#[test]
fn validates_against_the_public_schema_with_zero_diagnostics() {
    let path = format!("{}oracles/cache/onebusaway-config/stainless-config.yml", repo_root());
    let Ok(text) = std::fs::read_to_string(&path) else {
        // describe.skipIf(!existsSync(path))
        return;
    };
    let result = vc(&text);
    assert_eq!(result.diagnostics, Vec::<Diagnostic>::new());
    assert_eq!(organization_name(result.config.as_ref()), Some("onebusaway-sdk"));
}

#[test]
fn uses_only_keywords_the_validator_implements() {
    assert_eq!(unsupported_keywords(stainless_config_schema()), Vec::<String>::new());
}

#[test]
fn has_patterns_that_compile_as_unicode_regexes() {
    fn collect<'a>(node: &'a Value, out: &mut Vec<&'a str>) {
        match node {
            Value::Object(map) => {
                for (key, value) in map {
                    if let (true, Value::String(p)) = (key == "pattern", value) {
                        out.push(p);
                    }
                    collect(value, out);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect(item, out);
                }
            }
            _ => {}
        }
    }
    let mut patterns = Vec::new();
    collect(stainless_config_schema(), &mut patterns);
    assert_ne!(patterns, Vec::<&str>::new());
    for p in patterns {
        assert!(pattern_compiles(p), "{p}");
    }
}

#[test]
fn flags_unknown_keywords_anywhere_ignoring_x_extensions() {
    let schema = v(&json!({
        "x-anything": 1,
        "if": {},
        "properties": { "a": { "not": {} } },
        "$defs": { "d": { "dependentSchemas": {} } },
        "anyOf": [true, { "else": {} }],
        "items": { "patternProperties": {} },
    }));
    assert_eq!(
        unsupported_keywords(&schema),
        vec![
            "/if",
            "/properties/a/not",
            "/$defs/d/dependentSchemas",
            "/anyOf/1/else",
            "/items/patternProperties"
        ]
    );
}

#[test]
fn handles_boolean_schemas() {
    assert_eq!(keywords(&json!(true), &json!(1)), kw(&[]));
    assert_eq!(keywords(&json!(false), &json!(1)), kw(&[("false", "")]));
}

#[test]
fn checks_types_with_integer_as_a_subtype_of_number() {
    assert_eq!(keywords(&json!({ "type": "number" }), &json!(3)), kw(&[]));
    assert_eq!(
        keywords(&json!({ "type": "integer" }), &json!(3.5)),
        kw(&[("type", "")])
    );
    assert_eq!(keywords(&json!({ "type": ["string", "null"] }), &json!(null)), kw(&[]));
    assert_eq!(keywords(&json!({ "type": "object" }), &json!([])), kw(&[("type", "")]));
    assert_eq!(
        keywords(&json!({ "type": "string", "minLength": 3 }), &json!(1)),
        kw(&[("type", "")])
    );
}

#[test]
fn checks_const_and_enum_by_deep_equality() {
    assert_eq!(
        keywords(&json!({ "const": { "a": [1] } }), &json!({ "a": [1] })),
        kw(&[])
    );
    assert_eq!(keywords(&json!({ "const": "a" }), &json!("b")), kw(&[("const", "")]));
    assert_eq!(keywords(&json!({ "enum": [1, "x"] }), &json!("x")), kw(&[]));
    assert_eq!(keywords(&json!({ "enum": [1, "x"] }), &json!(2)), kw(&[("enum", "")]));
}

#[test]
fn checks_string_length_in_code_points_and_patterns() {
    assert_eq!(
        keywords(&json!({ "minLength": 2 }), &json!("😀")),
        kw(&[("minLength", "")])
    );
    assert_eq!(keywords(&json!({ "maxLength": 1 }), &json!("😀")), kw(&[]));
    assert_eq!(
        keywords(&json!({ "maxLength": 1 }), &json!("ab")),
        kw(&[("maxLength", "")])
    );
    assert_eq!(keywords(&json!({ "pattern": "^a+$" }), &json!("aa")), kw(&[]));
    assert_eq!(
        keywords(&json!({ "pattern": "^a+$" }), &json!("ab")),
        kw(&[("pattern", "")])
    );
}

#[test]
fn checks_numeric_bounds() {
    let s = json!({ "minimum": 1, "maximum": 5, "exclusiveMinimum": 1, "exclusiveMaximum": 5 });
    assert_eq!(keywords(&s, &json!(3)), kw(&[]));
    assert_eq!(keywords(&s, &json!(1)), kw(&[("exclusiveMinimum", "")]));
    assert_eq!(keywords(&s, &json!(5)), kw(&[("exclusiveMaximum", "")]));
    assert_eq!(
        keywords(&s, &json!(0)),
        kw(&[("minimum", ""), ("exclusiveMinimum", "")])
    );
    assert_eq!(
        keywords(&s, &json!(9)),
        kw(&[("maximum", ""), ("exclusiveMaximum", "")])
    );
}

#[test]
fn checks_objects_required_properties_additional_properties_property_names() {
    let s = json!({
        "type": "object",
        "required": ["a"],
        "properties": { "a": { "type": "string" } },
        "additionalProperties": { "type": "integer" },
        "propertyNames": { "minLength": 1 },
    });
    assert_eq!(keywords(&s, &json!({ "a": "x", "b": 1 })), kw(&[]));
    assert_eq!(
        keywords(&s, &json!({ "b": "no", "": 1 })),
        kw(&[("required", ""), ("type", "b"), ("propertyNames", "")])
    );
    assert_eq!(
        keywords(
            &json!({ "properties": { "a": true }, "additionalProperties": false }),
            &json!({ "a": 1, "z": 2 })
        ),
        kw(&[("additionalProperties", "z")])
    );
}

#[test]
fn does_not_treat_inherited_names_as_declared_properties() {
    assert_eq!(
        keywords(
            &json!({ "properties": {}, "additionalProperties": false }),
            &json!({ "toString": 1 })
        ),
        kw(&[("additionalProperties", "toString")])
    );
}

#[test]
fn checks_array_items() {
    assert_eq!(
        keywords(&json!({ "items": { "type": "string" } }), &json!(["a", 2])),
        kw(&[("type", "1")])
    );
}

#[test]
fn applies_all_of_and_ref() {
    let root = json!({
        "$defs": { "S": { "type": "string" } },
        "allOf": [{ "$ref": "#/$defs/S" }, { "minLength": 2 }],
    });
    assert_eq!(keywords(&root, &json!("ab")), kw(&[]));
    assert_eq!(keywords(&root, &json!("a")), kw(&[("minLength", "")]));
    assert_eq!(keywords(&root, &json!(1)), kw(&[("type", "")]));
}

#[test]
fn passes_any_of_when_any_branch_passes_and_keeps_that_branchs_warnings() {
    let s = v(&json!({
        "anyOf": [{ "type": "string" }, { "properties": { "old": { "deprecated": true } } }],
    }));
    assert_eq!(validate(&s, &v(&json!("x"))).len(), 0);
    let got: Vec<_> = validate(&s, &v(&json!({ "old": 1 })))
        .iter()
        .map(|e| (e.keyword.name(), e.severity))
        .collect();
    assert_eq!(got, vec![("deprecated", ErrorSeverity::Warning)]);
}

#[test]
fn reports_the_only_type_compatible_any_of_branch() {
    let s = json!({
        "anyOf": [{ "type": "string" }, { "type": "object", "required": ["endpoint"] }],
    });
    assert_eq!(keywords(&s, &json!({})), kw(&[("required", "")]));
}

#[test]
fn uses_a_discriminator_to_pick_between_object_branches() {
    let branch = |cmd: &str, field: &str| {
        json!({
            "type": "object",
            "required": ["command", field],
            "properties": { "command": { "const": cmd }, (field): { "type": "string" } },
        })
    };
    let s = json!({ "oneOf": [branch("update", "a"), branch("remove", "b")] });
    assert_eq!(
        keywords(&s, &json!({ "command": "remove", "b": 1 })),
        kw(&[("type", "b")])
    );
    assert_eq!(keywords(&s, &json!({ "command": "move" })), kw(&[("oneOf", "")]));
}

#[test]
fn prefers_the_branch_whose_errors_go_deepest_then_the_one_with_fewest_errors() {
    let deepest = json!({
        "anyOf": [
            { "type": "object", "required": ["x", "y"] },
            { "type": "object", "properties": { "a": { "properties": { "b": { "type": "string" } } } } },
        ],
    });
    assert_eq!(keywords(&deepest, &json!({ "a": { "b": 1 } })), kw(&[("type", "a/b")]));
    let fewest = json!({
        "anyOf": [
            { "type": "object", "required": ["x", "y"] },
            { "type": "object", "required": ["z"] },
        ],
    });
    assert_eq!(keywords(&fewest, &json!({})), kw(&[("required", "")]));
}

#[test]
fn reports_any_of_of_constants_as_an_enum_with_the_full_list() {
    let s = v(&json!({
        "$defs": { "C": { "const": "c" } },
        "anyOf": [{ "const": "a" }, { "enum": ["b"] }, { "$ref": "#/$defs/C" }, { "anyOf": [{ "const": "d" }] }],
    }));
    let errors = validate(&s, &v(&json!("z")));
    let first = errors.first().expect("an error");
    assert_eq!(first.keyword, Keyword::Enum);
    assert_eq!(
        first.params.allowed,
        Some(vec![
            Value::from("a"),
            Value::from("b"),
            Value::from("c"),
            Value::from("d")
        ])
    );
}

#[test]
fn summarises_unmatched_alternatives() {
    let s = v(&json!({
        "$defs": { "O": { "type": "object" } },
        "anyOf": [
            { "type": "string" },
            { "anyOf": [{ "$ref": "#/$defs/O" }, { "type": ["array", "integer"] }] },
        ],
    }));
    let errors = validate(&s, &Value::Bool(true));
    assert_eq!(
        errors.first().map(|e| e.message.as_str()),
        Some("expected a string or an object or an array or an integer, got true")
    );
    let t = v(&json!({ "anyOf": [false, { "type": "string" }] }));
    assert_eq!(
        validate(&t, &v(&json!(2))).first().map(|e| e.keyword),
        Some(Keyword::AnyOf)
    );
}

#[test]
fn rejects_one_of_when_several_branches_pass() {
    assert_eq!(
        keywords(
            &json!({ "oneOf": [{ "type": "string" }, { "minLength": 1 }] }),
            &json!("x")
        ),
        kw(&[("oneOf", "")])
    );
    assert_eq!(
        keywords(
            &json!({ "oneOf": [{ "type": "string" }, { "type": "number" }] }),
            &json!("x")
        ),
        kw(&[])
    );
}

#[test]
fn reports_deprecated_properties_as_warnings_with_rename_hints() {
    let s = v(&json!({
        "properties": { "old": { "deprecated": true, "x-deprecated-renamed-to": "new" } },
    }));
    let errors = validate(&s, &v(&json!({ "old": 1 })));
    let first = errors.first().expect("a warning");
    assert_eq!(first.keyword, Keyword::Deprecated);
    assert_eq!(first.severity, ErrorSeverity::Warning);
    assert_eq!(first.params.renamed_to, Some(Value::from("new")));
    assert!(!first.params.removed);
}

#[test]
fn ignores_deprecation_warnings_raised_by_property_names() {
    let s = json!({ "propertyNames": { "properties": {} }, "properties": {} });
    assert_eq!(keywords(&s, &json!({ "a": 1 })), kw(&[]));
}

#[test]
fn resolves_escaped_pointers_and_rejects_remote_or_missing_refs() {
    let root = v(&json!({ "$defs": { "a/b": { "const": 1 }, "c~d": true } }));
    assert_eq!(resolve_ref(&root, "#/$defs/a~1b"), Ok(&v(&json!({ "const": 1 }))));
    assert_eq!(resolve_ref(&root, "#/$defs/c~0d"), Ok(&Value::Bool(true)));
    let remote = resolve_ref(&root, "other.json#/x").expect_err("remote ref");
    assert!(remote.contains("only local refs"), "{remote}");
    let missing = resolve_ref(&root, "#/$defs/missing").expect_err("missing ref");
    assert!(missing.contains("unresolvable"), "{missing}");
    let through_value = resolve_ref(&root, "#/$defs/a~1b/const").expect_err("ref into a value");
    assert!(through_value.contains("unresolvable"), "{through_value}");
}

#[test]
fn describes_booleans_and_stops_at_the_ref_budget() {
    assert_eq!(
        describe_alternatives(&Value::Bool(true), &[Value::Bool(true), Value::Bool(false)]),
        vec!["any value"]
    );
    let root = v(&json!({ "$defs": { "L": { "$ref": "#/$defs/L" } } }));
    assert_eq!(
        describe_alternatives(&root, &[v(&json!({ "$ref": "#/$defs/L" }))]),
        Vec::<String>::new()
    );
}

#[test]
fn escapes_tilde_and_slash() {
    assert_eq!(
        to_json_pointer(&[
            Seg::from("paths"),
            Seg::from("/users/{id}"),
            Seg::from(0usize),
            Seg::from("a~b")
        ]),
        "/paths/~1users~1{id}/0/a~0b"
    );
    assert_eq!(to_json_pointer(&[]), "");
}

// ---------------------------------------------------------------------------------------------
// Properties (fast-check `fc.jsonValue()` -> proptest)
// ---------------------------------------------------------------------------------------------

/// Object keys: arbitrary strings plus the names JavaScript objects treat specially.
fn json_key() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => any::<String>(),
        1 => prop_oneof![
            Just("__proto__".to_owned()),
            Just("toString".to_owned()),
            Just("constructor".to_owned()),
            Just("0".to_owned()),
            Just(String::new()),
        ],
    ]
}

/// Any JSON value (`fc.jsonValue()`): finite numbers only, nested arrays and objects.
fn json_value() -> impl Strategy<Value = Json> {
    let leaf = prop_oneof![
        Just(Json::Null),
        any::<bool>().prop_map(Json::Bool),
        any::<i32>().prop_map(Json::from),
        (prop::num::f64::NORMAL | prop::num::f64::SUBNORMAL | prop::num::f64::ZERO).prop_map(Json::from),
        any::<String>().prop_map(Json::String),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Json::Array),
            prop::collection::vec((json_key(), inner), 0..4)
                .prop_map(|entries| Json::Object(entries.into_iter().collect())),
        ]
    })
}

/// `fc.dictionary(fc.string(), fc.jsonValue())`.
fn json_dictionary() -> impl Strategy<Value = Json> {
    prop::collection::vec((json_key(), json_value()), 0..6)
        .prop_map(|entries| Json::Object(entries.into_iter().collect()))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn a_value_always_matches_its_own_const_and_enum(raw in json_value()) {
        let value = v(&raw);
        let mut by_const = ferrule_converter::Map::new();
        by_const.insert("const", value.clone());
        let mut by_enum = ferrule_converter::Map::new();
        by_enum.insert("enum", Value::Array(vec![value.clone()]));
        prop_assert_eq!(validate(&Value::Object(by_const), &value).len(), 0);
        prop_assert_eq!(validate(&Value::Object(by_enum), &value).len(), 0);
    }

    #[test]
    fn a_value_always_matches_a_schema_of_its_own_type(raw in json_value()) {
        let value = v(&raw);
        let schema = v(&json!({ "type": json_type(&value) }));
        prop_assert_eq!(validate(&schema, &value).len(), 0);
    }

    #[test]
    fn deep_equal_agrees_with_structural_json_equality(raw_a in json_value(), raw_b in json_value()) {
        let (a, b) = (v(&raw_a), v(&raw_b));
        prop_assert!(deep_equal(&a, &a.clone()));
        if deep_equal(&a, &b) {
            prop_assert!(deep_equal(&b, &a));
        }
        // The fixed cases after `fc.assert` in the same `it`.
        assert!(!deep_equal(&v(&json!([1])), &v(&json!({ "0": 1 }))));
        assert!(!deep_equal(&v(&json!({ "a": 1 })), &v(&json!({ "a": 1, "b": 2 }))));
        assert!(!deep_equal(&v(&json!({ "a": 1 })), &v(&json!({ "b": 1 }))));
    }

    #[test]
    fn any_of_of_true_and_anything_accepts_every_value(raw in json_value()) {
        let schema = v(&json!({ "anyOf": [{ "type": "null" }, true] }));
        prop_assert_eq!(validate(&schema, &v(&raw)).len(), 0);
    }

    #[test]
    fn returns_the_spec_unchanged_with_no_diagnostics(raw in json_dictionary()) {
        let spec = v(&raw);
        let result = convert(&spec, &Value::object());
        prop_assert_eq!(result.spec, spec);
        prop_assert_eq!(result.diagnostics, Vec::<Diagnostic>::new());
    }
}
