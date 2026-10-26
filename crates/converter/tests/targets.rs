//! Every stainless.yml target and its `skip:`/`only:` lists reach the plugins: `x-ferrule-targets`
//! (root, mirrored on every method) and `x-ferrule-targets-only` (method and `x-ferrule-models`
//! entry). Shapes reduced from the onebusaway and imagekit oracles
//! (OneBusAway/sdk-config `stainless-config.yml`, imagekit-developer/imagekit-openapi
//! `stainless-config/main.yaml`).

use ferrule_converter::editions::apply_edition;
use ferrule_converter::schema::config::parse_config;
use ferrule_converter::{Diagnostic, Value, convert};
use proptest::prelude::*;
use serde_json::json;

fn v(j: &serde_json::Value) -> Value {
    Value::from(j)
}

fn spec() -> Value {
    v(&json!({
        "openapi": "3.1.0",
        "paths": {
            "/files": {"get": {"responses": {}}, "post": {"responses": {}}},
            "/dummy": {"post": {"responses": {}}},
            "/dummy/inner": {"get": {"responses": {}}}
        },
        "components": {"schemas": {
            "File": {"type": "object"},
            "AITag": {"type": "object"},
            "ExtensionItem": {"type": "object"},
            "Hidden": {"type": "object"}
        }}
    }))
}

fn op<'a>(out: &'a Value, path: &str, verb: &str, key: &str) -> Option<&'a Value> {
    out.get("paths")
        .and_then(|p| p.get(path))
        .and_then(|i| i.get(verb))
        .and_then(|o| o.get(key))
}

fn codes(ds: &[Diagnostic]) -> Vec<(&str, String, &str)> {
    ds.iter()
        .map(|d| (d.code.as_str(), d.severity.to_string(), d.pointer.as_str()))
        .collect()
}

const IMAGEKIT: &str = r##"
organization: { name: imagekit }
targets:
  typescript:
    package_name: "@imagekit/nodejs"
    production_repo: imagekit-developer/imagekit-nodejs#master
    publish: { npm: true }
    options: { mcp_server: { package_name: "@imagekit/api-mcp" } }
  python:
    package_name: imagekitio
    production_repo: imagekit-developer/imagekit-python#master
    project_name: imagekitio
    publish: { pypi: true }
  go:
    package_name: imagekit
    edition: go.2026-03-10
    options: { enable_v2: true }
  java:
    reverse_domain: io.imagekit.api
    skip: false
resources:
  $shared:
    models:
      AITag: { openapi_uri: "#/components/schemas/AITag", only: [go, java] }
      ExtensionItem: { openapi_uri: "#/components/schemas/ExtensionItem", only: [ruby] }
      Hidden: { openapi_uri: "#/components/schemas/Hidden", skip: true }
  files:
    models: { File: "#/components/schemas/File" }
    methods:
      list: get /files
      upload: { endpoint: post /files, skip: [java] }
  dummy:
    only: [go, python]
    methods:
      create: post /dummy
    subresources:
      inner:
        skip: [go]
        methods:
          get: { endpoint: get /dummy/inner }
"##;

fn targets() -> Value {
    v(&json!({
        "typescript": {
            "package_name": "@imagekit/nodejs",
            "edition": "typescript.2025-10-08",
            "production_repo": "imagekit-developer/imagekit-nodejs#master",
            "options": {"options": {"mcp_server": {"package_name": "@imagekit/api-mcp"}}}
        },
        "python": {
            "package_name": "imagekitio",
            "edition": "python.2025-10-08",
            "production_repo": "imagekit-developer/imagekit-python#master",
            "options": {"project_name": "imagekitio", "options": {"use_uv": false}}
        },
        "go": {
            "package_name": "imagekit",
            "edition": "go.2026-03-10",
            "options": {"options": {"enable_v2": true}}
        },
        "java": {
            "edition": "java.2025-10-08",
            "options": {"reverse_domain": "io.imagekit.api", "skip": false}
        }
    }))
}

#[test]
fn carries_every_target_to_the_root_and_every_method() {
    let r = convert(&spec(), &parse_config(IMAGEKIT).expect("config"));
    assert_eq!(codes(&r.diagnostics), []);
    assert_eq!(r.spec.get("x-ferrule-targets"), Some(&targets()));
    for (path, verb) in [
        ("/files", "get"),
        ("/files", "post"),
        ("/dummy", "post"),
        ("/dummy/inner", "get"),
    ] {
        assert_eq!(
            op(&r.spec, path, verb, "x-ferrule-targets"),
            Some(&targets()),
            "{verb} {path}"
        );
    }
}

#[test]
fn lists_the_targets_a_restricted_method_is_generated_for() {
    let r = convert(&spec(), &parse_config(IMAGEKIT).expect("config"));
    let only = |path: &str, verb: &str| op(&r.spec, path, verb, "x-ferrule-targets-only").cloned();
    assert_eq!(only("/files", "get"), None);
    assert_eq!(only("/files", "post"), Some(v(&json!(["typescript", "python", "go"]))));
    assert_eq!(only("/dummy", "post"), Some(v(&json!(["python", "go"]))));
    assert_eq!(only("/dummy/inner", "get"), Some(v(&json!(["python"]))));
}

#[test]
fn keeps_models_not_generated_for_python_with_the_targets_that_generate_them() {
    let r = convert(&spec(), &parse_config(IMAGEKIT).expect("config"));
    let models = op(&r.spec, "/files", "get", "x-ferrule-models").expect("models");
    assert_eq!(
        models,
        &v(&json!({
            "AITag": {"name": "AITag", "module": "shared.AITag", "x-ferrule-targets-only": ["go", "java"]},
            "File": {"name": "File", "module": "File"}
        }))
    );
    let type_name = |schema: &str| {
        r.spec
            .get("components")
            .and_then(|c| c.get("schemas"))
            .and_then(|s| s.get(schema))
            .and_then(|s| s.get("x-fern-type-name"))
            .cloned()
    };
    assert_eq!(type_name("AITag"), Some(Value::from("AITag")));
    assert_eq!(type_name("ExtensionItem"), None);
    assert_eq!(type_name("Hidden"), None);
}

#[test]
fn leaves_a_python_only_config_without_lists_as_before() {
    let config = parse_config(
        "
targets: { python: { package_name: acme } }
resources: { files: { methods: { list: get /files } } }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    assert_eq!(r.spec.get("x-ferrule-targets"), None);
    assert_eq!(op(&r.spec, "/files", "get", "x-ferrule-targets"), None);
    assert_eq!(op(&r.spec, "/files", "get", "x-ferrule-targets-only"), None);
}

#[test]
fn writes_the_targets_for_a_python_only_config_with_lists() {
    let config = parse_config(
        "
targets: { python: { package_name: acme, edition: python.2025-11-20 } }
resources: { files: { methods: { list: { endpoint: get /files, skip: [go] } } } }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    let expected = v(&json!({"python": {
        "package_name": "acme", "edition": "python.2025-11-20", "options": {"options": {"use_uv": true}}
    }}));
    assert_eq!(r.spec.get("x-ferrule-targets"), Some(&expected));
    assert_eq!(
        op(&r.spec, "/files", "get", "x-ferrule-targets-only"),
        Some(&v(&json!(["python"])))
    );
}

#[test]
fn keeps_unknown_targets_drops_null_settings_and_reports_unknown_editions() {
    let config = parse_config(
        "
targets:
  python: { package_name: acme }
  go: { edition: go.2030-01-01, production_repo: null }
  cobol: { edition: cobol.1959, package_name: ACME }
resources: { files: { methods: { list: get /files } } }
",
    )
    .expect("config");
    // convert_text reports the editions' diagnostics; convert gets an edition-applied config.
    let ed = apply_edition(&config);
    assert_eq!(
        codes(&ed.diagnostics),
        [("FER1103", "warning".to_owned(), "/targets/go/edition")]
    );

    let r = convert(&spec(), &config);
    let t = r.spec.get("x-ferrule-targets").expect("targets");
    assert_eq!(
        t.get("go"),
        Some(&v(&json!({"edition": "go.2025-10-08", "options": {}})))
    );
    assert_eq!(
        t.get("cobol"),
        Some(&v(
            &json!({"package_name": "ACME", "edition": "cobol.1959", "options": {}})
        ))
    );
}

#[test]
fn writes_the_targets_into_every_alias() {
    let config = parse_config(
        "
targets: { python: { package_name: acme }, node: { package_name: acme } }
resources:
  files: { methods: { list: get /files } }
  documents: { only: [node], methods: { list: get /files } }
",
    )
    .expect("config");
    let r = convert(&spec(), &config);
    let aliases = op(&r.spec, "/files", "get", "x-forge-aliases")
        .and_then(Value::as_array)
        .expect("aliases");
    assert_eq!(aliases.len(), 2);
    assert!(aliases.iter().all(|a| a.get("x-ferrule-targets").is_some()));
    assert_eq!(aliases[0].get("x-ferrule-targets-only"), None);
    assert_eq!(aliases[1].get("x-ferrule-targets-only"), Some(&v(&json!(["node"]))));
}

const NAMES: [&str; 5] = ["node", "python", "go", "java", "ruby"];

fn list() -> impl Strategy<Value = Option<Vec<&'static str>>> {
    proptest::option::of(proptest::sample::subsequence(NAMES.to_vec(), 0..=NAMES.len()))
}

fn yaml_list(l: Option<&Vec<&str>>, key: &str) -> String {
    l.map_or_else(String::new, |l| format!(", {key}: [{}]", l.join(", ")))
}

proptest! {
    #[test]
    fn a_method_is_generated_for_the_targets_every_only_lists_and_no_skip_lists(
        targets in proptest::sample::subsequence(NAMES.to_vec(), 1..=NAMES.len()),
        r_only in list(), r_skip in list(), m_only in list(), m_skip in list(),
    ) {
        let config = format!(
            "targets: {{ {} }}\nresources: {{ files: {{ methods: {{ list: {{ endpoint: get /files{}{} }} }}{}{} }} }}\n",
            targets.iter().map(|t| format!("{t}: {{}}")).collect::<Vec<_>>().join(", "),
            yaml_list(m_only.as_ref(), "only"), yaml_list(m_skip.as_ref(), "skip"),
            yaml_list(r_only.as_ref(), "only"), yaml_list(r_skip.as_ref(), "skip"),
        );
        let r = convert(&spec(), &parse_config(&config).expect("config"));
        let got = op(&r.spec, "/files", "get", "x-ferrule-targets-only").cloned();
        let restricted = r_only.is_some() || r_skip.is_some() || m_only.is_some() || m_skip.is_some();
        if restricted {
            let expected: Vec<serde_json::Value> = targets
                .iter()
                .filter(|t| [&r_only, &m_only].iter().all(|o| o.as_ref().is_none_or(|o| o.contains(t))))
                .filter(|t| [&r_skip, &m_skip].iter().all(|s| s.as_ref().is_none_or(|s| !s.contains(t))))
                .map(|t| json!(t))
                .collect();
            prop_assert_eq!(got, Some(v(&serde_json::Value::Array(expected))));
        } else {
            prop_assert_eq!(got, None);
        }
    }
}
