//! The `vehicles_for_agency` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `vehicles_for_agency` resource.
#[derive(Debug, Clone, Copy)]
pub struct VehiclesForAgencyResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> VehiclesForAgencyResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get vehicles for a specific agency
    pub fn list(
        &self,
        agency_id: impl Into<String>,
    ) -> Request<'a, VehiclesForAgencyListParams, M> {
        let params = VehiclesForAgencyListParams {
            agency_id: agency_id.into(),
            time: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/vehicles-for-agency/{agencyID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct VehiclesForAgencyListParams {
    /// ID of the agency
    pub agency_id: String,
    /// Specific time for querying the status (timestamp format)
    pub time: Option<String>,
}

impl<M> Request<'_, VehiclesForAgencyListParams, M> {
    /// Specific time for querying the status (timestamp format)
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.params_mut().time = Some(time.into());
        self
    }
}

impl Endpoint for VehiclesForAgencyListParams {
    type Output = VehiclesForAgencyListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/vehicles-for-agency/{}.json",
                path_param(&self.agency_id)?
            ),
        );
        spec.query("time", &self.time)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: VehiclesForAgencyListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<VehiclesForAgencyListResponseDataList>,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponseDataList {
    #[serde(rename = "lastLocationUpdateTime")]
    pub last_location_update_time: i64,
    #[serde(rename = "lastUpdateTime")]
    pub last_update_time: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<VehiclesForAgencyListResponseDataListLocation>,
    #[serde(rename = "occupancyCapacity")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy_capacity: Option<i64>,
    #[serde(rename = "occupancyCount")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy_count: Option<i64>,
    #[serde(rename = "occupancyStatus")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "tripId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<String>,
    /// Trip-specific status for the arriving transit vehicle.
    #[serde(rename = "tripStatus")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_status: Option<VehiclesForAgencyListResponseDataListTripStatus>,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponseDataListLocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// Trip-specific status for the arriving transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponseDataListTripStatus {
    /// Trip ID of the trip the vehicle is actively serving.
    #[serde(rename = "activeTripId")]
    pub active_trip_id: String,
    /// Index of the active trip into the sequence of trips for the active block.
    #[serde(rename = "blockTripSequence")]
    pub block_trip_sequence: i64,
    /// ID of the closest stop to the current location of the transit vehicle.
    #[serde(rename = "closestStop")]
    pub closest_stop: String,
    /// Time offset from the closest stop to the current position of the transit vehicle (in seconds).
    #[serde(rename = "closestStopTimeOffset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closest_stop_time_offset: Option<i64>,
    /// Distance, in meters, the transit vehicle has progressed along the active trip.
    #[serde(rename = "distanceAlongTrip")]
    pub distance_along_trip: f64,
    /// Information about frequency-based scheduling, if applicable to the trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    /// Last known distance along the trip received in real-time from the transit vehicle.
    #[serde(rename = "lastKnownDistanceAlongTrip")]
    pub last_known_distance_along_trip: f64,
    /// Last known location of the transit vehicle (optional).
    #[serde(rename = "lastKnownLocation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_location:
        Option<VehiclesForAgencyListResponseDataListTripStatusLastKnownLocation>,
    /// Last known orientation value received in real-time from the transit vehicle.
    #[serde(rename = "lastKnownOrientation")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_orientation: Option<f64>,
    /// Timestamp of the last known real-time location update from the transit vehicle.
    #[serde(rename = "lastLocationUpdateTime")]
    pub last_location_update_time: i64,
    /// Timestamp of the last known real-time update from the transit vehicle.
    #[serde(rename = "lastUpdateTime")]
    pub last_update_time: i64,
    /// ID of the next stop the transit vehicle is scheduled to arrive at.
    #[serde(rename = "nextStop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_stop: Option<String>,
    /// Time offset from the next stop to the current position of the transit vehicle (in seconds).
    #[serde(rename = "nextStopTimeOffset")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_stop_time_offset: Option<i64>,
    /// Capacity of the transit vehicle in terms of occupancy.
    #[serde(rename = "occupancyCapacity")]
    pub occupancy_capacity: i64,
    /// Current count of occupants in the transit vehicle.
    #[serde(rename = "occupancyCount")]
    pub occupancy_count: i64,
    /// Current occupancy status of the transit vehicle.
    #[serde(rename = "occupancyStatus")]
    pub occupancy_status: String,
    /// Orientation of the transit vehicle, represented as an angle in degrees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<f64>,
    /// Current journey phase of the trip.
    pub phase: String,
    /// Current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<VehiclesForAgencyListResponseDataListTripStatusPosition>,
    /// Indicates if real-time arrival info is available for this trip.
    pub predicted: bool,
    /// Deviation from the schedule in seconds (positive for late, negative for early).
    #[serde(rename = "scheduleDeviation")]
    pub schedule_deviation: i64,
    /// Distance, in meters, the transit vehicle is scheduled to have progressed along the active trip.
    #[serde(rename = "scheduledDistanceAlongTrip")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_distance_along_trip: Option<f64>,
    /// Time, in milliseconds since the Unix epoch, of midnight for the start of the service date for the trip.
    #[serde(rename = "serviceDate")]
    pub service_date: i64,
    /// References to situation elements (if any) applicable to this trip.
    #[serde(rename = "situationIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub situation_ids: Option<Vec<String>>,
    /// Current status modifiers for the trip.
    pub status: String,
    /// Total length of the trip, in meters.
    #[serde(rename = "totalDistanceAlongTrip")]
    pub total_distance_along_trip: f64,
    /// ID of the transit vehicle currently serving the trip.
    #[serde(rename = "vehicleId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle_id: Option<String>,
}

/// Last known location of the transit vehicle (optional).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponseDataListTripStatusLastKnownLocation {
    /// Latitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// Current position of the transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehiclesForAgencyListResponseDataListTripStatusPosition {
    /// Latitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}
