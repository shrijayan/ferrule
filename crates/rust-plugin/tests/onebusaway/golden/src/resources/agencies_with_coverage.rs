//! The `agencies_with_coverage` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `agencies_with_coverage` resource.
#[derive(Debug, Clone, Copy)]
pub struct AgenciesWithCoverageResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> AgenciesWithCoverageResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Returns a list of all transit agencies currently supported by OneBusAway along with the center of their coverage area.
    pub fn list(&self) -> Request<'a, AgenciesWithCoverageListParams, M> {
        let params = AgenciesWithCoverageListParams {};
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/agencies-with-coverage.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct AgenciesWithCoverageListParams {}

impl Endpoint for AgenciesWithCoverageListParams {
    type Output = AgenciesWithCoverageListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/agencies-with-coverage.json".to_owned(),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenciesWithCoverageListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: AgenciesWithCoverageListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenciesWithCoverageListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<AgenciesWithCoverageListResponseDataList>,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgenciesWithCoverageListResponseDataList {
    #[serde(rename = "agencyId")]
    pub agency_id: String,
    pub lat: f64,
    #[serde(rename = "latSpan")]
    pub lat_span: f64,
    pub lon: f64,
    #[serde(rename = "lonSpan")]
    pub lon_span: f64,
}
