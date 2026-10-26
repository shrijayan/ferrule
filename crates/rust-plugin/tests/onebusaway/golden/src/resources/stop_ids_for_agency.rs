//! The `stop_ids_for_agency` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `stop_ids_for_agency` resource.
#[derive(Debug, Clone, Copy)]
pub struct StopIdsForAgencyResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> StopIdsForAgencyResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get stop IDs for a specific agency
    pub fn list(&self, agency_id: impl Into<String>) -> Request<'a, StopIdsForAgencyListParams, M> {
        let params = StopIdsForAgencyListParams {
            agency_id: agency_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/stop-ids-for-agency/{agencyID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct StopIdsForAgencyListParams {
    /// ID of the agency
    pub agency_id: String,
}

impl Endpoint for StopIdsForAgencyListParams {
    type Output = StopIdsForAgencyListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/stop-ids-for-agency/{}.json",
                path_param(&self.agency_id)
            ),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopIdsForAgencyListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: StopIdsForAgencyListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopIdsForAgencyListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<String>,
    pub references: crate::shared::References,
}
