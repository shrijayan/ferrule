//! The `route_ids_for_agency` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `route_ids_for_agency` resource.
#[derive(Debug, Clone, Copy)]
pub struct RouteIdsForAgencyResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> RouteIdsForAgencyResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get route IDs for a specific agency
    pub fn list(
        &self,
        agency_id: impl Into<String>,
    ) -> Request<'a, RouteIdsForAgencyListParams, M> {
        let params = RouteIdsForAgencyListParams {
            agency_id: agency_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/route-ids-for-agency/{agencyID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct RouteIdsForAgencyListParams {
    /// ID of the agency
    pub agency_id: String,
}

impl Endpoint for RouteIdsForAgencyListParams {
    type Output = RouteIdsForAgencyListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/route-ids-for-agency/{}.json",
                path_param(&self.agency_id)
            ),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteIdsForAgencyListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: RouteIdsForAgencyListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteIdsForAgencyListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<String>,
    pub references: crate::shared::References,
}
