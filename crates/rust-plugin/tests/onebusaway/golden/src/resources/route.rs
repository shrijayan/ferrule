//! The `route` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `route` resource.
#[derive(Debug, Clone, Copy)]
pub struct RouteResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> RouteResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Retrieve info for a specific route identified by ID
    ///
    /// Retrieve information for a specific route identified by its unique ID.
    pub fn retrieve(&self, route_id: impl Into<String>) -> Request<'a, RouteRetrieveParams, M> {
        let params = RouteRetrieveParams {
            route_id: route_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/route/{routeID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct RouteRetrieveParams {
    /// The ID of the route.
    pub route_id: String,
}

impl Endpoint for RouteRetrieveParams {
    type Output = RouteRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/api/where/route/{}.json", path_param(&self.route_id)),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: RouteRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteRetrieveResponseData {
    pub entry: RouteRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteRetrieveResponseDataEntry {
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
