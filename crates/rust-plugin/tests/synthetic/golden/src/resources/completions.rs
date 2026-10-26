//! The `completions` resource.

use crate::rt::{
    Async, Core, Endpoint, EventStream, Request, RequestSpec, Result, decode_json, decode_sse,
};
use serde::{Deserialize, Serialize};

/// The `completions` resource.
#[derive(Debug, Clone, Copy)]
pub struct CompletionsResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> CompletionsResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Complete a note
    pub fn create(
        &self,
        model: CompletionsCreateModel,
        prompt: impl Into<String>,
    ) -> Request<'a, CompletionsCreateParams, M> {
        let params = CompletionsCreateParams {
            max_tokens: None,
            metadata: None,
            model,
            prompt: prompt.into(),
        };
        Request::new(self.core, params, self.mode)
    }

    /// Complete a note
    pub fn create_streaming(
        &self,
        model: CompletionsCreateModel,
        prompt: impl Into<String>,
    ) -> Request<'a, CompletionsCreateStreamingParams, M> {
        let params = CompletionsCreateStreamingParams {
            max_tokens: None,
            metadata: None,
            model,
            prompt: prompt.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `create` (POST `/v1/completions`).
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionsCreateParams {
    pub max_tokens: Option<i64>,
    pub metadata: Option<std::collections::BTreeMap<String, String>>,
    pub model: CompletionsCreateModel,
    pub prompt: String,
}

impl<M> Request<'_, CompletionsCreateParams, M> {
    pub fn max_tokens(mut self, max_tokens: i64) -> Self {
        self.params_mut().max_tokens = Some(max_tokens);
        self
    }

    pub fn metadata(mut self, metadata: std::collections::BTreeMap<String, String>) -> Self {
        self.params_mut().metadata = Some(metadata);
        self
    }
}

impl Endpoint for CompletionsCreateParams {
    type Output = CompletionsCreateResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::POST, "/v1/completions".to_owned());
        spec.json(&serde_json::Map::new())?;
        spec.body_field("max_tokens", &self.max_tokens, false)?;
        spec.body_field("metadata", &self.metadata, false)?;
        spec.body_field("model", &self.model, true)?;
        spec.body_field("prompt", &self.prompt, true)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

/// The params of `create_streaming` (POST `/v1/completions`).
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionsCreateStreamingParams {
    pub max_tokens: Option<i64>,
    pub metadata: Option<std::collections::BTreeMap<String, String>>,
    pub model: CompletionsCreateModel,
    pub prompt: String,
}

impl<M> Request<'_, CompletionsCreateStreamingParams, M> {
    pub fn max_tokens(mut self, max_tokens: i64) -> Self {
        self.params_mut().max_tokens = Some(max_tokens);
        self
    }

    pub fn metadata(mut self, metadata: std::collections::BTreeMap<String, String>) -> Self {
        self.params_mut().metadata = Some(metadata);
        self
    }
}

impl Endpoint for CompletionsCreateStreamingParams {
    type Output = EventStream<CompletionsCreateEvent>;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::POST, "/v1/completions".to_owned());
        spec.accept = "text/event-stream";
        spec.json(&serde_json::Map::new())?;
        spec.body_field("max_tokens", &self.max_tokens, false)?;
        spec.body_field("metadata", &self.metadata, false)?;
        spec.body_field("model", &self.model, true)?;
        spec.body_field("prompt", &self.prompt, true)?;
        spec.body_field("stream", &true, true)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_sse(response).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionsCreateModel {
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "large")]
    Large,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionsCreateResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<CompletionsCreateResponseFinishReason>,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionsCreateResponseFinishReason {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "length")]
    Length,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionsCreateEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<CompletionsCreateEventFinishReason>,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionsCreateEventFinishReason {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "length")]
    Length,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}
