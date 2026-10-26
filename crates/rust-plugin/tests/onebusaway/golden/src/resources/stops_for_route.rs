//! The `stops_for_route` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `stops_for_route` resource.
#[derive(Debug, Clone, Copy)]
pub struct StopsForRouteResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> StopsForRouteResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get stops for a specific route
    pub fn list(&self, route_id: impl Into<String>) -> Request<'a, StopsForRouteListParams, M> {
        let params = StopsForRouteListParams {
            route_id: route_id.into(),
            include_polylines: None,
            time: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/stops-for-route/{routeID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct StopsForRouteListParams {
    /// ID of the route
    pub route_id: String,
    /// Include polyline elements in the response (default true)
    pub include_polylines: Option<bool>,
    /// Specify service date (YYYY-MM-DD or epoch) (default today)
    pub time: Option<String>,
}

impl<M> Request<'_, StopsForRouteListParams, M> {
    /// Include polyline elements in the response (default true)
    pub fn include_polylines(mut self, include_polylines: bool) -> Self {
        self.params_mut().include_polylines = Some(include_polylines);
        self
    }

    /// Specify service date (YYYY-MM-DD or epoch) (default today)
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.params_mut().time = Some(time.into());
        self
    }
}

impl Endpoint for StopsForRouteListParams {
    type Output = StopsForRouteListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/stops-for-route/{}.json",
                path_param(&self.route_id)
            ),
        );
        spec.query("includePolylines", &self.include_polylines)?;
        spec.query("time", &self.time)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: StopsForRouteListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponseData {
    pub entry: StopsForRouteListResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponseDataEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polylines: Option<Vec<StopsForRouteListResponseDataEntryPolylines>>,
    #[serde(rename = "routeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(rename = "stopGroupings")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_groupings: Option<Vec<StopsForRouteListResponseDataEntryStopGroupings>>,
    #[serde(rename = "stopIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponseDataEntryPolylines {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponseDataEntryStopGroupings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<StopsForRouteListResponseDataEntryStopGroupingsName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polylines: Option<Vec<StopsForRouteListResponseDataEntryStopGroupingsPolylines>>,
    #[serde(rename = "stopIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponseDataEntryStopGroupingsName {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub names: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForRouteListResponseDataEntryStopGroupingsPolylines {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<String>,
}
