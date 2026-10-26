//! The `trips_for_location` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `trips_for_location` resource.
#[derive(Debug, Clone, Copy)]
pub struct TripsForLocationResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> TripsForLocationResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Retrieve trips for a given location
    pub fn list(&self, lat_span: f32, lon_span: f32) -> Request<'a, TripsForLocationListParams, M> {
        let params = TripsForLocationListParams {
            lat: None,
            lon: None,
            lat_span,
            lon_span,
            include_trip: None,
            include_schedule: None,
            time: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/trips-for-location.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct TripsForLocationListParams {
    /// The latitude coordinate of the search center. If omitted, defaults to 0.0.
    pub lat: Option<f32>,
    /// The longitude coordinate of the search center. If omitted, defaults to 0.0.
    pub lon: Option<f32>,
    /// Latitude span of the search bounding box
    pub lat_span: f32,
    /// Longitude span of the search bounding box
    pub lon_span: f32,
    /// Whether to include full trip elements in the references section. Defaults to true.
    pub include_trip: Option<bool>,
    /// Whether to include full schedule elements in the tripDetails section. Defaults to false.
    pub include_schedule: Option<bool>,
    /// Specific time for the query. Defaults to the current time.
    pub time: Option<i64>,
}

impl<M> Request<'_, TripsForLocationListParams, M> {
    /// The latitude coordinate of the search center. If omitted, defaults to 0.0.
    pub fn lat(mut self, lat: f32) -> Self {
        self.params_mut().lat = Some(lat);
        self
    }

    /// The longitude coordinate of the search center. If omitted, defaults to 0.0.
    pub fn lon(mut self, lon: f32) -> Self {
        self.params_mut().lon = Some(lon);
        self
    }

    /// Whether to include full trip elements in the references section. Defaults to true.
    pub fn include_trip(mut self, include_trip: bool) -> Self {
        self.params_mut().include_trip = Some(include_trip);
        self
    }

    /// Whether to include full schedule elements in the tripDetails section. Defaults to false.
    pub fn include_schedule(mut self, include_schedule: bool) -> Self {
        self.params_mut().include_schedule = Some(include_schedule);
        self
    }

    /// Specific time for the query. Defaults to the current time.
    pub fn time(mut self, time: i64) -> Self {
        self.params_mut().time = Some(time);
        self
    }
}

impl Endpoint for TripsForLocationListParams {
    type Output = TripsForLocationListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/trips-for-location.json".to_owned(),
        );
        spec.query("lat", &self.lat)?;
        spec.query("lon", &self.lon)?;
        spec.query("latSpan", &self.lat_span)?;
        spec.query("lonSpan", &self.lon_span)?;
        spec.query("includeTrip", &self.include_trip)?;
        spec.query("includeSchedule", &self.include_schedule)?;
        spec.query("time", &self.time)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: TripsForLocationListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponseData {
    /// Indicates if the limit of trips has been exceeded
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<TripsForLocationListResponseDataList>,
    /// Indicates if the search location is out of range
    #[serde(rename = "outOfRange")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_range: Option<bool>,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponseDataList {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<TripsForLocationListResponseDataListSchedule>,
    #[serde(rename = "serviceDate")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_date: Option<i64>,
    #[serde(rename = "situationIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub situation_ids: Option<Vec<String>>,
    /// Trip-specific status for the arriving transit vehicle.
    pub status: TripsForLocationListResponseDataListStatus,
    #[serde(rename = "tripId")]
    pub trip_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponseDataListSchedule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    #[serde(rename = "nextTripId")]
    pub next_trip_id: String,
    #[serde(rename = "previousTripId")]
    pub previous_trip_id: String,
    #[serde(rename = "stopTimes")]
    pub stop_times: Vec<TripsForLocationListResponseDataListScheduleStopTimes>,
    #[serde(rename = "timeZone")]
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponseDataListScheduleStopTimes {
    #[serde(rename = "arrivalTime")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival_time: Option<i64>,
    #[serde(rename = "departureTime")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure_time: Option<i64>,
    #[serde(rename = "distanceAlongTrip")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_along_trip: Option<f64>,
    #[serde(rename = "historicalOccupancy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_occupancy: Option<String>,
    #[serde(rename = "stopHeadsign")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_headsign: Option<String>,
    #[serde(rename = "stopId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
}

/// Trip-specific status for the arriving transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponseDataListStatus {
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
    pub last_known_location: Option<TripsForLocationListResponseDataListStatusLastKnownLocation>,
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
    pub position: Option<TripsForLocationListResponseDataListStatusPosition>,
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
pub struct TripsForLocationListResponseDataListStatusLastKnownLocation {
    /// Latitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// Current position of the transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripsForLocationListResponseDataListStatusPosition {
    /// Latitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}
