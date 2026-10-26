//! The `search_for_route` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `search_for_route` resource.
#[derive(Debug, Clone, Copy)]
pub struct SearchForRouteResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> SearchForRouteResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Search for a route based on its name.
    pub fn list(&self, input: impl Into<String>) -> Request<'a, SearchForRouteListParams, M> {
        let params = SearchForRouteListParams {
            input: input.into(),
            max_count: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/search/route.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct SearchForRouteListParams {
    /// The string to search for.
    pub input: String,
    /// The max number of results to return. Defaults to 20.
    pub max_count: Option<i64>,
}

impl<M> Request<'_, SearchForRouteListParams, M> {
    /// The max number of results to return. Defaults to 20.
    pub fn max_count(mut self, max_count: i64) -> Self {
        self.params_mut().max_count = Some(max_count);
        self
    }
}

impl Endpoint for SearchForRouteListParams {
    type Output = SearchForRouteListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/search/route.json".to_owned(),
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
pub struct SearchForRouteListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<SearchForRouteListResponseData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchForRouteListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<SearchForRouteListResponseDataList>,
    #[serde(rename = "outOfRange")]
    pub out_of_range: bool,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchForRouteListResponseDataList {
    #[serde(rename = "agencyId")]
    pub agency_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub id: String,
    #[serde(rename = "longName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
    #[serde(rename = "nullSafeShortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_safe_short_name: Option<String>,
    #[serde(rename = "shortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(rename = "textColor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    pub r#type: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
