//! The `trip` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `trip` resource.
#[derive(Debug, Clone, Copy)]
pub struct TripResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> TripResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get details of a specific trip
    pub fn retrieve(&self, trip_id: impl Into<String>) -> Request<'a, TripRetrieveParams, M> {
        let params = TripRetrieveParams {
            trip_id: trip_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/trip/{tripID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct TripRetrieveParams {
    /// ID of the trip
    pub trip_id: String,
}

impl Endpoint for TripRetrieveParams {
    type Output = TripRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/api/where/trip/{}.json", path_param(&self.trip_id)?),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: TripRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripRetrieveResponseData {
    pub entry: TripRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripRetrieveResponseDataEntry {
    #[serde(rename = "blockId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(rename = "directionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction_id: Option<String>,
    pub id: String,
    #[serde(rename = "peakOffpeak")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak_offpeak: Option<i64>,
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "routeShortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_short_name: Option<String>,
    #[serde(rename = "serviceId")]
    pub service_id: String,
    #[serde(rename = "shapeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape_id: Option<String>,
    #[serde(rename = "timeZone")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(rename = "tripHeadsign")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_headsign: Option<String>,
    #[serde(rename = "tripShortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_short_name: Option<String>,
}
