//! What the PHP runtime template needs from the plugin (`runtime/php`, rendered by
//! `ferrule_runtime.render_php_files`): its options.

use super::Gen;

/// `PhpOptions` of the runtime renderer, field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeOptions {
    pub namespace: String,
    pub timeout: f64,
    pub max_retries: u32,
    pub initial_retry_delay: f64,
    pub max_retry_delay: f64,
}

impl RuntimeOptions {
    /// The options as the JSON object `ferrule generate` passes to `PhpOptions(**options)`.
    #[must_use]
    pub fn to_json(&self) -> String {
        let value = serde_json::json!({
            "namespace": self.namespace,
            "timeout": self.timeout,
            "max_retries": self.max_retries,
            "initial_retry_delay": self.initial_retry_delay,
            "max_retry_delay": self.max_retry_delay,
        });
        let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
        text.push('\n');
        text
    }
}

/// The runtime options of `g`: the namespace and `client_settings` (Stainless's defaults without
/// them).
#[must_use]
pub fn runtime_options(g: &Gen<'_>) -> RuntimeOptions {
    let s = g.client.settings.as_ref();
    RuntimeOptions {
        namespace: g.ns.clone(),
        timeout: s.map_or(60.0, |s| s.timeout_seconds),
        max_retries: s.map_or(2, |s| s.max_retries),
        initial_retry_delay: s.and_then(|s| s.initial_retry_delay_seconds).unwrap_or(0.5),
        max_retry_delay: s.and_then(|s| s.max_retry_delay_seconds).unwrap_or(8.0),
    }
}
