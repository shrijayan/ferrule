//! The `current_time` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `current_time` resource.
#[derive(Debug, Clone, Copy)]
pub struct CurrentTimeResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> CurrentTimeResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// current-time
    pub fn retrieve(&self) -> Request<'a, CurrentTimeRetrieveParams, M> {
        let params = CurrentTimeRetrieveParams {};
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/current-time.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentTimeRetrieveParams {}

impl Endpoint for CurrentTimeRetrieveParams {
    type Output = CurrentTimeRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/current-time.json".to_owned(),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentTimeRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: CurrentTimeRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentTimeRetrieveResponseData {
    pub entry: CurrentTimeRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentTimeRetrieveResponseDataEntry {
    #[serde(rename = "readableTime")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readable_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
}
