//! The `search_for_stop` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `search_for_stop` resource.
#[derive(Debug, Clone, Copy)]
pub struct SearchForStopResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> SearchForStopResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Search for a stop based on its name.
    pub fn list(&self, input: impl Into<String>) -> Request<'a, SearchForStopListParams, M> {
        let params = SearchForStopListParams {
            input: input.into(),
            max_count: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/search/stop.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct SearchForStopListParams {
    /// The string to search for.
    pub input: String,
    /// The max number of results to return. Defaults to 20.
    pub max_count: Option<i64>,
}

impl<M> Request<'_, SearchForStopListParams, M> {
    /// The max number of results to return. Defaults to 20.
    pub fn max_count(mut self, max_count: i64) -> Self {
        self.params_mut().max_count = Some(max_count);
        self
    }
}

impl Endpoint for SearchForStopListParams {
    type Output = SearchForStopListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/search/stop.json".to_owned(),
        );
        spec.query("input", &self.input)?;
        spec.query("maxCount", &self.max_count)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchForStopListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<SearchForStopListResponseData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchForStopListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<SearchForStopListResponseDataList>,
    #[serde(rename = "outOfRange")]
    pub out_of_range: bool,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchForStopListResponseDataList {
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
