//! The `stops_for_location` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `stops_for_location` resource.
#[derive(Debug, Clone, Copy)]
pub struct StopsForLocationResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> StopsForLocationResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// stops-for-location
    pub fn list(&self) -> Request<'a, StopsForLocationListParams, M> {
        let params = StopsForLocationListParams {
            lat: None,
            lon: None,
            radius: None,
            lat_span: None,
            lon_span: None,
            query: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/stops-for-location.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct StopsForLocationListParams {
    /// If omitted, defaults to 0.0.
    pub lat: Option<f64>,
    /// If omitted, defaults to 0.0.
    pub lon: Option<f64>,
    /// The radius in meters to search within
    pub radius: Option<f64>,
    /// An alternative to radius to set the search bounding box (optional)
    pub lat_span: Option<f64>,
    /// An alternative to radius to set the search bounding box (optional)
    pub lon_span: Option<f64>,
    /// A search query string to filter the results
    pub query: Option<String>,
}

impl<M> Request<'_, StopsForLocationListParams, M> {
    /// If omitted, defaults to 0.0.
    pub fn lat(mut self, lat: f64) -> Self {
        self.params_mut().lat = Some(lat);
        self
    }

    /// If omitted, defaults to 0.0.
    pub fn lon(mut self, lon: f64) -> Self {
        self.params_mut().lon = Some(lon);
        self
    }

    /// The radius in meters to search within
    pub fn radius(mut self, radius: f64) -> Self {
        self.params_mut().radius = Some(radius);
        self
    }

    /// An alternative to radius to set the search bounding box (optional)
    pub fn lat_span(mut self, lat_span: f64) -> Self {
        self.params_mut().lat_span = Some(lat_span);
        self
    }

    /// An alternative to radius to set the search bounding box (optional)
    pub fn lon_span(mut self, lon_span: f64) -> Self {
        self.params_mut().lon_span = Some(lon_span);
        self
    }

    /// A search query string to filter the results
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.params_mut().query = Some(query.into());
        self
    }
}

impl Endpoint for StopsForLocationListParams {
    type Output = StopsForLocationListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/stops-for-location.json".to_owned(),
        );
        spec.query("lat", &self.lat)?;
        spec.query("lon", &self.lon)?;
        spec.query("radius", &self.radius)?;
        spec.query("latSpan", &self.lat_span)?;
        spec.query("lonSpan", &self.lon_span)?;
        spec.query("query", &self.query)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForLocationListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: StopsForLocationListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForLocationListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<StopsForLocationListResponseDataList>,
    #[serde(rename = "outOfRange")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_range: Option<bool>,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopsForLocationListResponseDataList {
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
