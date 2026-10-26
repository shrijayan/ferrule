//! The `comments` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `comments` resource.
#[derive(Debug, Clone, Copy)]
pub struct CommentsResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> CommentsResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn list(&self, note_id: impl Into<String>) -> Request<'a, CommentsListParams, M> {
        let params = CommentsListParams {
            note_id: note_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/notes/{note_id}/comments`).
#[derive(Debug, Clone, PartialEq)]
pub struct CommentsListParams {
    pub note_id: String,
}

impl Endpoint for CommentsListParams {
    type Output = Vec<CommentsListResponseItem>;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/v1/notes/{}/comments", path_param(&self.note_id)?),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentsListResponseItem {
    pub body: String,
}
