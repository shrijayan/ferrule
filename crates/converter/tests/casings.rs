//! `custom_casings` and `standalone_api`: the converter carries both to the plugin and applies
//! them to model names and model modules. Reduced from the isaacus and lumaai oracles
//! (isaacus-dev/openapi `config/stainless.yaml`, lumalabs/lumaai-api `openapi.stainless.yml`).

use ferrule_converter::schema::config::parse_config;
use ferrule_converter::{Diagnostic, Value, convert};
use serde_json::json;

fn v(j: &serde_json::Value) -> Value {
    Value::from(j)
}

fn spec() -> Value {
    v(&json!({
        "openapi": "3.1.0",
        "paths": {
            "/extractions/qa": {"post": {"responses": {}}},
            "/embeddings": {"post": {"responses": {}}}
        },
        "components": {"schemas": {
            "AnswerExtractionResponse": {"type": "object"},
            "Embedding": {"type": "object"},
            "ILGSv1 Crossreference": {"type": "object"},
            "ILGSv1 ID Number": {"type": "object"},
            "LumaThing": {"type": "object"}
        }}
    }))
}

fn op<'a>(out: &'a Value, path: &str, key: &str) -> Option<&'a Value> {
    out.get("paths")
        .and_then(|p| p.get(path))
        .and_then(|i| i.get("post"))
        .and_then(|o| o.get(key))
}

fn codes(ds: &[Diagnostic]) -> Vec<(&str, String, &str)> {
    ds.iter()
        .map(|d| (d.code.as_str(), d.severity.to_string(), d.pointer.as_str()))
        .collect()
}

/// The isaacus shape: two standalone subresources (one with only models) and three casings.
const ISAACUS: &str = "
organization: { name: isaacus }
targets: { python: { package_name: isaacus, production_repo: isaacus-dev/isaacus-python } }
resources:
  embeddings:
    models: { embedding: Embedding, lumaai_thing: LumaThing }
    methods: { create: post /embeddings }
  extractions:
    subresources:
      qa:
        standalone_api: true
        models: { answer_extraction_response: AnswerExtractionResponse }
        methods: { create: post /extractions/qa }
  ilgs:
    subresources:
      v1:
        standalone_api: true
        models:
          crossreference: ILGSv1 Crossreference
          id_number: ILGSv1 ID Number
custom_casings:
  qa: { initialism: true }
  ilgs: { initialism: true }
  v1: { pascal: v1, camel: v1, capital: V1, snake: v1 }
  lumaai: { pascal: LumaAI, camel: lumaai, snake: lumaai, capital: LumaAI }
";

#[test]
fn carries_custom_casings_to_the_root_and_every_operation() {
    let r = convert(&spec(), &parse_config(ISAACUS).expect("config"));
    assert_eq!(codes(&r.diagnostics), []);
    let casings = v(&json!({
        "ilgs": {"initialism": true},
        "lumaai": {"pascal": "LumaAI"},
        "qa": {"initialism": true},
        "v1": {"pascal": "v1"}
    }));
    assert_eq!(r.spec.get("x-ferrule-casings"), Some(&casings));
    for path in ["/extractions/qa", "/embeddings"] {
        assert_eq!(op(&r.spec, path, "x-ferrule-casings"), Some(&casings), "{path}");
    }
}

#[test]
fn names_models_with_custom_casings_and_the_default_initialisms() {
    let r = convert(&spec(), &parse_config(ISAACUS).expect("config"));
    let models = op(&r.spec, "/embeddings", "x-ferrule-models").expect("models");
    let name = |schema: &str| models.get(schema).and_then(|m| m.get("name")).cloned();
    assert_eq!(name("ILGSv1 ID Number"), Some(Value::from("IDNumber")));
    assert_eq!(name("LumaThing"), Some(Value::from("LumaAIThing")));
    assert_eq!(name("Embedding"), Some(Value::from("Embedding")));
}

#[test]
fn an_initialism_set_to_false_leaves_the_default_list() {
    let config = parse_config(
        "
resources: { ilgs: { models: { id_number: ILGSv1 ID Number }, methods: { create: post /embeddings } } }
custom_casings: { id: { initialism: false } }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    let models = op(&r.spec, "/embeddings", "x-ferrule-models").expect("models");
    assert_eq!(
        models.get("ILGSv1 ID Number").and_then(|m| m.get("name")),
        Some(&Value::from("IdNumber"))
    );
}

#[test]
fn puts_a_standalone_resources_models_in_its_own_directory() {
    let r = convert(&spec(), &parse_config(ISAACUS).expect("config"));
    let models = op(&r.spec, "/embeddings", "x-ferrule-models").expect("models");
    let module = |schema: &str| models.get(schema).and_then(|m| m.get("module")).cloned();
    assert_eq!(
        module("AnswerExtractionResponse"),
        Some(Value::from("extractions.qa.answer_extraction_response"))
    );
    assert_eq!(
        module("ILGSv1 Crossreference"),
        Some(Value::from("ilgs.v1.crossreference"))
    );
    assert_eq!(module("Embedding"), Some(Value::from("embedding")));
}

#[test]
fn lists_standalone_resources_at_the_root_and_on_every_operation() {
    let r = convert(&spec(), &parse_config(ISAACUS).expect("config"));
    let standalone = v(&json!([
        {"path": ["extractions", "qa"], "order": [1, 0]},
        {"path": ["ilgs", "v1"], "order": [2, 0]}
    ]));
    assert_eq!(r.spec.get("x-ferrule-standalone"), Some(&standalone));
    for path in ["/extractions/qa", "/embeddings"] {
        assert_eq!(op(&r.spec, path, "x-ferrule-standalone"), Some(&standalone), "{path}");
    }
}

#[test]
fn writes_neither_extension_when_the_config_has_neither() {
    let config = parse_config(
        "
targets: { python: { package_name: acme } }
resources: { embeddings: { methods: { create: post /embeddings } } }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    for key in ["x-ferrule-casings", "x-ferrule-standalone"] {
        assert!(r.spec.get(key).is_none(), "{key}");
        assert!(op(&r.spec, "/embeddings", key).is_none(), "{key}");
    }
}

#[test]
fn warns_on_casings_it_cannot_apply_and_keeps_the_rest() {
    let config = parse_config(
        "
targets: { python: { package_name: acme } }
resources: { embeddings: { methods: { create: post /embeddings } } }
custom_casings:
  payment_method: { pascal: PaymentMethod, camel: paymentMethod, snake: payment_method, capital: Payment Method }
  datum:
    singular: { pascal: Datum, camel: datum, snake: datum, capital: Datum }
    plural: { pascal: Data, camel: data, snake: data, capital: Data }
  odd: { pascal: \"Odd-Name\", camel: odd, snake: odd, capital: Odd }
  wrong: 3
  qa: { initialism: true }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    assert_eq!(
        codes(&r.diagnostics),
        [
            ("FER1701", "warning".to_owned(), "/custom_casings/payment_method"),
            ("FER1702", "warning".to_owned(), "/custom_casings/datum"),
            ("FER1703", "warning".to_owned(), "/custom_casings/odd/pascal"),
            ("FER1703", "warning".to_owned(), "/custom_casings/wrong"),
        ]
    );
    assert!(r.diagnostics.iter().all(|d| !d.suggestion.is_empty()));
    assert_eq!(
        r.spec.get("x-ferrule-casings"),
        Some(&v(&json!({"qa": {"initialism": true}})))
    );
}

/// The class name `config` gives the model that names the `Embedding` schema.
fn embedding_name(config: &str) -> Option<Value> {
    let r = convert(&spec(), &parse_config(config).expect("config"));
    let models = op(&r.spec, "/embeddings", "x-ferrule-models")?;
    models.get("Embedding").and_then(|m| m.get("name")).cloned()
}

#[test]
fn an_initialism_set_to_false_is_not_read_as_a_plural() {
    // `https` is a default initialism; turned off, it is not the plural of `http` either.
    let name = embedding_name(
        "
resources: { embeddings: { models: { https_result: Embedding }, methods: { create: post /embeddings } } }
custom_casings: { https: { initialism: false } }
",
    );
    assert_eq!(name, Some(Value::from("HttpsResult")));
}

#[test]
fn ignores_a_pascal_spelling_that_puts_a_digit_first() {
    // `123` would start `qa_answer` as `123Answer`; a word that starts with a digit (`3ds`) may
    // keep one, since it cannot start a class name either way.
    let config = parse_config(
        "
targets: { python: { package_name: acme } }
resources: { embeddings: { models: { qa_answer: Embedding }, methods: { create: post /embeddings } } }
custom_casings:
  qa: { pascal: \"123\", camel: qa, snake: qa, capital: QA }
  3ds: { pascal: 3DS, camel: 3ds, snake: 3ds, capital: 3DS }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    assert_eq!(
        codes(&r.diagnostics),
        [("FER1703", "warning".to_owned(), "/custom_casings/qa/pascal")]
    );
    assert_eq!(
        r.spec.get("x-ferrule-casings"),
        Some(&v(&json!({"3ds": {"pascal": "3DS"}})))
    );
    let models = op(&r.spec, "/embeddings", "x-ferrule-models").expect("models");
    assert_eq!(
        models.get("Embedding").and_then(|m| m.get("name")),
        Some(&Value::from("QaAnswer"))
    );
}
