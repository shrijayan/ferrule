//! The `block` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `block` resource.
#[derive(Debug, Clone, Copy)]
pub struct BlockResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> BlockResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get details of a specific block by ID
    pub fn retrieve(&self, block_id: impl Into<String>) -> Request<'a, BlockRetrieveParams, M> {
        let params = BlockRetrieveParams {
            block_id: block_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/block/{blockID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockRetrieveParams {
    /// ID of the block
    pub block_id: String,
}

impl Endpoint for BlockRetrieveParams {
    type Output = BlockRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/api/where/block/{}.json", path_param(&self.block_id)?),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: BlockRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponseData {
    pub entry: BlockRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponseDataEntry {
    pub configurations: Vec<BlockRetrieveResponseDataEntryConfigurations>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponseDataEntryConfigurations {
    #[serde(rename = "activeServiceIds")]
    pub active_service_ids: Vec<String>,
    #[serde(rename = "inactiveServiceIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inactive_service_ids: Option<Vec<String>>,
    pub trips: Vec<BlockRetrieveResponseDataEntryConfigurationsTrips>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponseDataEntryConfigurationsTrips {
    #[serde(rename = "accumulatedSlackTime")]
    pub accumulated_slack_time: f32,
    #[serde(rename = "blockStopTimes")]
    pub block_stop_times: Vec<BlockRetrieveResponseDataEntryConfigurationsTripsBlockStopTimes>,
    #[serde(rename = "distanceAlongBlock")]
    pub distance_along_block: f32,
    #[serde(rename = "tripId")]
    pub trip_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponseDataEntryConfigurationsTripsBlockStopTimes {
    #[serde(rename = "accumulatedSlackTime")]
    pub accumulated_slack_time: f32,
    #[serde(rename = "blockSequence")]
    pub block_sequence: i64,
    #[serde(rename = "distanceAlongBlock")]
    pub distance_along_block: f32,
    #[serde(rename = "stopTime")]
    pub stop_time: BlockRetrieveResponseDataEntryConfigurationsTripsBlockStopTimesStopTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRetrieveResponseDataEntryConfigurationsTripsBlockStopTimesStopTime {
    #[serde(rename = "arrivalTime")]
    pub arrival_time: i64,
    #[serde(rename = "departureTime")]
    pub departure_time: i64,
    #[serde(rename = "dropOffType")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_off_type: Option<i64>,
    #[serde(rename = "pickupType")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup_type: Option<i64>,
    #[serde(rename = "stopId")]
    pub stop_id: String,
}
