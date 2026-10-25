//! The options the JVM runtime template is rendered with (`ferrule_runtime.JavaOptions`).

use ferrule_sdk_model::model::{ArrayFormat, Method, Resource};

use crate::emit::Gen;

/// `ferrule_runtime.JavaOptions`, as `ferrule-gen generate --target java|kotlin` writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub package_name: String,
    pub client_name: String,
    pub artifact_name: String,
    pub repo: String,
    /// `java` or `kotlin`.
    pub target: String,
    /// `comma` or `repeat`.
    pub array_format: String,
    pub empty_responses: bool,
}

impl RuntimeOptions {
    /// The options as a JSON object, keys in `JavaOptions` field order.
    #[must_use]
    pub fn to_json(&self) -> String {
        let value = serde_json::json!({
            "package_name": self.package_name,
            "client_name": self.client_name,
            "artifact_name": self.artifact_name,
            "repo": self.repo,
            "target": self.target,
            "array_format": self.array_format,
            "empty_responses": self.empty_responses,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

fn any_method(resources: &[Resource], f: &dyn Fn(&Method) -> bool) -> bool {
    resources
        .iter()
        .any(|r| r.methods.iter().any(f) || any_method(&r.subresources, f))
}

/// The runtime options of one SDK.
#[must_use]
pub fn runtime_options(g: &Gen<'_>) -> RuntimeOptions {
    let repeat = g
        .client
        .settings
        .as_ref()
        .is_some_and(|s| s.array_format == ArrayFormat::Repeat);
    RuntimeOptions {
        package_name: g.target.package.clone(),
        client_name: g.target.client.clone(),
        artifact_name: g.target.artifact.clone(),
        repo: g.target.repo.clone(),
        target: g.target.flavor.key().to_owned(),
        array_format: if repeat { "repeat" } else { "comma" }.to_owned(),
        empty_responses: any_method(&g.client.resources, &|m| m.response.is_none()),
    }
}
