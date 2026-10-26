//! The `logs` resource.

use crate::rt::{Async, Core, Endpoint, Page, Request, RequestSpec, Result, decode_page};
use serde::{Deserialize, Serialize};

/// The `logs` resource.
#[derive(Debug, Clone, Copy)]
pub struct LogsResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> LogsResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn list(&self) -> Request<'a, LogsListParams, M> {
        let params = LogsListParams {};
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/logs`).
#[derive(Debug, Clone, PartialEq)]
pub struct LogsListParams {}

impl Endpoint for LogsListParams {
    type Output = Page<LogsListResponse>;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            "/v1/logs".to_owned(),
        ))
    }

    async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Self::Output> {
        decode_page(&crate::pages::URL_CURSOR, response, spec).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogsListResponse {
    pub line: String,
}
