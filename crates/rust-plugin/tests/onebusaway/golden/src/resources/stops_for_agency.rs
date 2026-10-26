//! The `stops_for_agency` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `stops_for_agency` resource.
#[derive(Debug, Clone, Copy)]
pub struct StopsForAgencyResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> StopsForAgencyResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get stops for a specific agency
    pub fn list(&self, agency_id: impl Into<String>) -> Request<'a, StopsForAgencyListParams, M> {
        let params = StopsForAgencyListParams {
            agency_id: agency_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/stops-for-agency/{agencyID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct StopsForAgencyListParams {
    /// ID of the agency
    pub agency_id: String,
}

impl Endpoint for StopsForAgencyListParams {
    type Output = StopsForAgencyListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/stops-for-agency/{}.json",
                path_param(&self.agency_id)
            ),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForAgencyListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: StopsForAgencyListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForAgencyListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<StopsForAgencyListResponseDataList>,
    #[serde(rename = "outOfRange")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_range: Option<bool>,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForAgencyListResponseDataList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    pub id: String,
    pub lat: f64,
    #[serde(rename = "locationType")]
    pub location_type: i64,
    pub lon: f64,
    pub name: String,
    pub parent: String,
    #[serde(rename = "routeIds")]
    pub route_ids: Vec<String>,
    #[serde(rename = "staticRouteIds")]
    pub static_route_ids: Vec<String>,
    #[serde(rename = "wheelchairBoarding")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wheelchair_boarding: Option<String>,
}
