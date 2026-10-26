//! The `stop` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `stop` resource.
#[derive(Debug, Clone, Copy)]
pub struct StopResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> StopResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get details of a specific stop
    pub fn retrieve(&self, stop_id: impl Into<String>) -> Request<'a, StopRetrieveParams, M> {
        let params = StopRetrieveParams {
            stop_id: stop_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/stop/{stopID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct StopRetrieveParams {
    /// ID of the stop
    pub stop_id: String,
}

impl Endpoint for StopRetrieveParams {
    type Output = StopRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/api/where/stop/{}.json", path_param(&self.stop_id)?),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: StopRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopRetrieveResponseData {
    pub entry: StopRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopRetrieveResponseDataEntry {
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
