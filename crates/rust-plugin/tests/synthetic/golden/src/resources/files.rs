//! The `files` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `files` resource.
#[derive(Debug, Clone, Copy)]
pub struct FilesResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> FilesResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn upload(&self, file_id: i64, body: bytes::Bytes) -> Request<'a, FilesUploadParams, M> {
        let params = FilesUploadParams { file_id, body };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `upload` (POST `/v1/files/{file_id}/content`).
#[derive(Debug, Clone, PartialEq)]
pub struct FilesUploadParams {
    pub file_id: i64,
    pub body: bytes::Bytes,
}

impl Endpoint for FilesUploadParams {
    type Output = FilesUploadResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::POST,
            format!("/v1/files/{}/content", path_param(&self.file_id)?),
        );
        spec.bytes("application/octet-stream", self.body.clone());
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilesUploadResponse {
    pub size: i64,
}
