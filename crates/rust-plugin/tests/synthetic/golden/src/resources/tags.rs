//! The `tags` resource.

use crate::rt::{Async, Core, Endpoint, Page, Request, RequestSpec, Result, decode_page};
use serde::{Deserialize, Serialize};

/// The `tags` resource.
#[derive(Debug, Clone, Copy)]
pub struct TagsResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> TagsResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn list(&self) -> Request<'a, TagsListParams, M> {
        let params = TagsListParams {
            offset: None,
            r#type: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/tags`).
#[derive(Debug, Clone, PartialEq)]
pub struct TagsListParams {
    pub offset: Option<i64>,
    /// Only tags of this kind; a Rust keyword as a param name.
    pub r#type: Option<String>,
}

impl<M> Request<'_, TagsListParams, M> {
    pub fn offset(mut self, offset: i64) -> Self {
        self.params_mut().offset = Some(offset);
        self
    }

    /// Only tags of this kind; a Rust keyword as a param name.
    pub fn r#type(mut self, r#type: impl Into<String>) -> Self {
        self.params_mut().r#type = Some(r#type.into());
        self
    }
}

impl Endpoint for TagsListParams {
    type Output = Page<TagsListResponse>;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::GET, "/v1/tags".to_owned());
        spec.query("offset", &self.offset)?;
        spec.query("type", &self.r#type)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Self::Output> {
        decode_page(&crate::pages::OFFSET_PAGE, response, spec).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagsListResponse {
    pub name: String,
}
