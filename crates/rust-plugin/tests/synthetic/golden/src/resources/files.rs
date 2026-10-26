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

    /// Upload a file
    pub fn create(
        &self,
        file: crate::rt::FileUpload,
        purpose: FilesCreatePurpose,
    ) -> Request<'a, FilesCreateParams, M> {
        let params = FilesCreateParams {
            extra_files: None,
            file,
            purpose,
            tags: None,
        };
        Request::new(self.core, params, self.mode)
    }

    pub fn upload(&self, file_id: i64, body: bytes::Bytes) -> Request<'a, FilesUploadParams, M> {
        let params = FilesUploadParams { file_id, body };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `create` (POST `/v1/files`).
#[derive(Debug, Clone, PartialEq)]
pub struct FilesCreateParams {
    pub extra_files: Option<Vec<crate::rt::FileUpload>>,
    /// The file to upload.
    pub file: crate::rt::FileUpload,
    pub purpose: FilesCreatePurpose,
    pub tags: Option<Vec<String>>,
}

impl<M> Request<'_, FilesCreateParams, M> {
    pub fn extra_files(mut self, extra_files: Vec<crate::rt::FileUpload>) -> Self {
        self.params_mut().extra_files = Some(extra_files);
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.params_mut().tags = Some(tags);
        self
    }
}

impl Endpoint for FilesCreateParams {
    type Output = FilesCreateResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::POST, "/v1/files".to_owned());
        spec.multipart_files("extra_files", self.extra_files.iter().flatten());
        spec.multipart_files("file", [&self.file]);
        spec.multipart_field("purpose", &self.purpose)?;
        spec.multipart_field("tags", &self.tags)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilesCreatePurpose {
    #[serde(rename = "notes")]
    Notes,
    #[serde(rename = "attachments")]
    Attachments,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilesCreateResponse {
    pub id: String,
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
