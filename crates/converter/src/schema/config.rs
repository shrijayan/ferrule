//! Ferrule's typed reader of stainless.yml: the subset of the config the converter reads, checked
//! with the same rules (and messages) as the Zod schema.

use crate::value::Value;
use crate::zod::{Issue, Z};

fn opt(name: &'static str, z: Z) -> (&'static str, Z, bool) {
    (name, z, true)
}

fn req(name: &'static str, z: Z) -> (&'static str, Z, bool) {
    (name, z, false)
}

/// `"get /path"` shorthand, the object form, or an endpointless method (`type: webhook_unwrap`).
fn method_schema() -> Z {
    Z::Union(vec![
        Z::String,
        Z::Object(vec![
            req("endpoint", Z::String),
            opt("paginated", Z::Union(vec![Z::Boolean, Z::String])),
            opt("type", Z::String),
        ]),
        Z::Object(vec![req("type", Z::String)]),
    ])
}

fn model_schema() -> Z {
    Z::Union(vec![Z::String, Z::Object(vec![])])
}

/// Resources nest; the depth bound only stops runaway recursion on hostile input.
fn resource_schema(depth: usize) -> Z {
    let sub = if depth > 64 {
        Z::Unknown
    } else {
        Z::Record(Box::new(resource_schema(depth + 1)))
    };
    Z::Object(vec![
        opt("methods", Z::Record(Box::new(method_schema()))),
        opt("models", Z::Record(Box::new(model_schema()))),
        opt("subresources", sub),
    ])
}

fn pagination_field() -> Z {
    Z::Nullable(Box::new(Z::Object(vec![opt(
        "x-stainless-pagination-property",
        Z::Object(vec![req("purpose", Z::String)]),
    )])))
}

fn pagination_schema(named: bool) -> Z {
    Z::Object(vec![
        (if named {
            req("name", Z::String)
        } else {
            opt("name", Z::String)
        }),
        req("type", Z::String),
        opt("request", Z::Record(Box::new(pagination_field()))),
        opt("response", Z::Record(Box::new(pagination_field()))),
    ])
}

/// `ConfigSchema`.
#[must_use]
pub fn config_schema() -> Z {
    Z::Object(vec![
        opt("edition", Z::String),
        opt("organization", Z::Object(vec![req("name", Z::String)])),
        opt("resources", Z::Record(Box::new(resource_schema(0)))),
        opt(
            "pagination",
            Z::Union(vec![
                Z::Array(Box::new(pagination_schema(true))),
                pagination_schema(false),
            ]),
        ),
        opt(
            "targets",
            Z::Object(vec![opt(
                "python",
                Z::Object(vec![
                    opt("package_name", Z::String),
                    opt("project_name", Z::String),
                    opt("production_repo", Z::String),
                ]),
            )]),
        ),
    ])
}

/// `ConfigSchema.safeParse(value)` issues.
#[must_use]
pub fn config_issues(value: &Value) -> Vec<Issue> {
    config_schema().issues(value)
}

/// `parseConfig`: YAML text to a config the typed reader accepts.
///
/// # Errors
///
/// When the YAML does not parse or the typed reader rejects it.
pub fn parse_config(text: &str) -> Result<Value, String> {
    let value = crate::yaml::parse(text)?;
    let value = if value.is_null() { Value::object() } else { value };
    let issues = config_issues(&value);
    if let Some(i) = issues.first() {
        return Err(i.message.clone());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_minimal_config_and_keeps_unknown_keys() {
        assert_eq!(
            config_issues(&Value::from(json!({"organization": {"name": "acme"}, "extra": true}))),
            []
        );
        assert_eq!(
            config_issues(&Value::from(json!({"organization": {"name": 1}}))).len(),
            1
        );
    }

    #[test]
    fn parses_yaml_text() {
        let c = parse_config("organization:\n  name: acme\n").unwrap();
        assert_eq!(
            c.get("organization").and_then(|o| o.get("name")),
            Some(&Value::from("acme"))
        );
        assert_eq!(parse_config("").unwrap(), Value::object());
    }

    #[test]
    fn accepts_the_method_forms_models_shared_and_unknown_keys() {
        let config = parse_config(
            "
resources:
  a:
    methods:
      list: get /a
      create: { endpoint: post /a, paginated: false, type: http }
    models:
      thing: Thing
    subresources:
      b:
        methods:
          list: { endpoint: get /a/b, paginated: page }
  $shared:
    models: { ref: Reference }
pagination:
  - name: page
    type: offset
    request: { offset: { type: integer } }
    response: { items: { type: array } }
client_settings: { anything: true }
",
        )
        .unwrap();
        let list = config
            .get("resources")
            .and_then(|r| r.get("a"))
            .and_then(|a| a.get("subresources"))
            .and_then(|s| s.get("b"))
            .and_then(|b| b.get("methods"))
            .and_then(|m| m.get("list"));
        assert_eq!(
            list,
            Some(&Value::from(json!({"endpoint": "get /a/b", "paginated": "page"})))
        );
        assert!(config.get("client_settings").is_some());
    }

    #[test]
    fn rejects_malformed_methods_and_pagination() {
        assert!(parse_config("resources: { a: { methods: { x: 3 } } }").is_err());
        assert!(parse_config("pagination: [{ name: p }]").is_err());
    }

    #[test]
    fn issues_read_like_zod() {
        let msgs = |v: serde_json::Value| -> Vec<(String, String)> {
            config_issues(&Value::from(v))
                .into_iter()
                .map(|i| (crate::diagnostic::to_json_pointer(&i.path), i.message))
                .collect()
        };
        assert_eq!(
            msgs(json!({"resources": {"a": {"methods": {"x": 5}}}})),
            [("/resources/a/methods/x".to_owned(), "Invalid input".to_owned())]
        );
        assert_eq!(
            msgs(json!({"resources": {"a": {"models": null}}})),
            [(
                "/resources/a/models".to_owned(),
                "Invalid input: expected record, received null".to_owned()
            )]
        );
        assert_eq!(
            msgs(json!({"organization": {}})),
            [(
                "/organization/name".to_owned(),
                "Invalid input: expected string, received undefined".to_owned()
            )]
        );
        assert_eq!(
            msgs(json!({"targets": {"python": null}})),
            [(
                "/targets/python".to_owned(),
                "Invalid input: expected object, received null".to_owned()
            )]
        );
        assert_eq!(msgs(json!({"pagination": {"type": "x", "request": {"a": null}}})), []);
        assert_eq!(
            msgs(json!({"pagination": 5})),
            [("/pagination".to_owned(), "Invalid input".to_owned())]
        );
    }
}
