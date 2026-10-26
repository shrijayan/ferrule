//! The `routes_for_location` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `routes_for_location` resource.
#[derive(Debug, Clone, Copy)]
pub struct RoutesForLocationResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> RoutesForLocationResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// routes-for-location
    pub fn list(&self) -> Request<'a, RoutesForLocationListParams, M> {
        let params = RoutesForLocationListParams {
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

/// The params of `list` (GET `/api/where/routes-for-location.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct RoutesForLocationListParams {
    /// If omitted, defaults to 0.0.
    pub lat: Option<f64>,
    /// If omitted, defaults to 0.0.
    pub lon: Option<f64>,
    pub radius: Option<f64>,
    pub lat_span: Option<f64>,
    pub lon_span: Option<f64>,
    pub query: Option<String>,
}

impl<M> Request<'_, RoutesForLocationListParams, M> {
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

    pub fn radius(mut self, radius: f64) -> Self {
        self.params_mut().radius = Some(radius);
        self
    }

    pub fn lat_span(mut self, lat_span: f64) -> Self {
        self.params_mut().lat_span = Some(lat_span);
        self
    }

    pub fn lon_span(mut self, lon_span: f64) -> Self {
        self.params_mut().lon_span = Some(lon_span);
        self
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.params_mut().query = Some(query.into());
        self
    }
}

impl Endpoint for RoutesForLocationListParams {
    type Output = RoutesForLocationListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/routes-for-location.json".to_owned(),
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
pub struct RoutesForLocationListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: RoutesForLocationListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutesForLocationListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<RoutesForLocationListResponseDataList>,
    #[serde(rename = "outOfRange")]
    pub out_of_range: bool,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutesForLocationListResponseDataList {
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
