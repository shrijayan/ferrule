//! The `arrivals_and_departures_for_location` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `arrivals_and_departures_for_location` resource.
#[derive(Debug, Clone, Copy)]
pub struct ArrivalsAndDeparturesForLocationResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ArrivalsAndDeparturesForLocationResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// arrivals-and-departures-for-location
    ///
    /// Returns real-time arrival and departure data for stops within a bounding box or radius centered on a specific location.
    pub fn list(
        &self,
        lat: f64,
        lon: f64,
    ) -> Request<'a, ArrivalsAndDeparturesForLocationListParams, M> {
        let params = ArrivalsAndDeparturesForLocationListParams {
            lat,
            lon,
            radius: None,
            lat_span: None,
            lon_span: None,
            time: None,
            minutes_before: None,
            minutes_after: None,
            max_count: None,
            route_type: None,
            empty_returns_not_found: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/arrivals-and-departures-for-location.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrivalsAndDeparturesForLocationListParams {
    /// The latitude coordinate of the search center.
    pub lat: f64,
    /// The longitude coordinate of the search center.
    pub lon: f64,
    /// The search radius in meters.
    pub radius: Option<f64>,
    /// Sets the latitude limits of the search bounding box.
    pub lat_span: Option<f64>,
    /// Sets the longitude limits of the search bounding box.
    pub lon_span: Option<f64>,
    /// By default, returns the status right now. Can be queried at a specific time (milliseconds since epoch) for testing.
    pub time: Option<i64>,
    /// Include arrivals and departures this many minutes before the query time.
    pub minutes_before: Option<i64>,
    /// Include arrivals and departures this many minutes after the query time.
    pub minutes_after: Option<i64>,
    /// The max size of the list of nearby stops and arrivals to return. Defaults to 250, max 1000.
    pub max_count: Option<i64>,
    /// Optional list of GTFS routeTypes to filter by (comma delimited) e.g. "1,2,3".
    pub route_type: Option<String>,
    /// If true, returns a 404 Not Found error instead of an empty result.
    pub empty_returns_not_found: Option<bool>,
}

impl<M> Request<'_, ArrivalsAndDeparturesForLocationListParams, M> {
    /// The search radius in meters.
    pub fn radius(mut self, radius: f64) -> Self {
        self.params_mut().radius = Some(radius);
        self
    }

    /// Sets the latitude limits of the search bounding box.
    pub fn lat_span(mut self, lat_span: f64) -> Self {
        self.params_mut().lat_span = Some(lat_span);
        self
    }

    /// Sets the longitude limits of the search bounding box.
    pub fn lon_span(mut self, lon_span: f64) -> Self {
        self.params_mut().lon_span = Some(lon_span);
        self
    }

    /// By default, returns the status right now. Can be queried at a specific time (milliseconds since epoch) for testing.
    pub fn time(mut self, time: i64) -> Self {
        self.params_mut().time = Some(time);
        self
    }

    /// Include arrivals and departures this many minutes before the query time.
    pub fn minutes_before(mut self, minutes_before: i64) -> Self {
        self.params_mut().minutes_before = Some(minutes_before);
        self
    }

    /// Include arrivals and departures this many minutes after the query time.
    pub fn minutes_after(mut self, minutes_after: i64) -> Self {
        self.params_mut().minutes_after = Some(minutes_after);
        self
    }

    /// The max size of the list of nearby stops and arrivals to return. Defaults to 250, max 1000.
    pub fn max_count(mut self, max_count: i64) -> Self {
        self.params_mut().max_count = Some(max_count);
        self
    }

    /// Optional list of GTFS routeTypes to filter by (comma delimited) e.g. "1,2,3".
    pub fn route_type(mut self, route_type: impl Into<String>) -> Self {
        self.params_mut().route_type = Some(route_type.into());
        self
    }

    /// If true, returns a 404 Not Found error instead of an empty result.
    pub fn empty_returns_not_found(mut self, empty_returns_not_found: bool) -> Self {
        self.params_mut().empty_returns_not_found = Some(empty_returns_not_found);
        self
    }
}

impl Endpoint for ArrivalsAndDeparturesForLocationListParams {
    type Output = ArrivalsAndDeparturesForLocationListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/arrivals-and-departures-for-location.json".to_owned(),
        );
        spec.query("lat", &self.lat)?;
        spec.query("lon", &self.lon)?;
        spec.query("radius", &self.radius)?;
        spec.query("latSpan", &self.lat_span)?;
        spec.query("lonSpan", &self.lon_span)?;
        spec.query("time", &self.time)?;
        spec.query("minutesBefore", &self.minutes_before)?;
        spec.query("minutesAfter", &self.minutes_after)?;
        spec.query("maxCount", &self.max_count)?;
        spec.query("routeType", &self.route_type)?;
        spec.query("emptyReturnsNotFound", &self.empty_returns_not_found)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ArrivalsAndDeparturesForLocationListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponseData {
    pub entry: ArrivalsAndDeparturesForLocationListResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponseDataEntry {
    #[serde(rename = "arrivalsAndDepartures")]
    pub arrivals_and_departures:
        Vec<ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDepartures>,
    #[serde(rename = "limitExceeded")]
    pub limit_exceeded: bool,
    #[serde(rename = "nearbyStopIds")]
    pub nearby_stop_ids: Vec<ArrivalsAndDeparturesForLocationListResponseDataEntryNearbyStopIds>,
    #[serde(rename = "situationIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub situation_ids: Option<Vec<String>>,
    #[serde(rename = "stopIds")]
    pub stop_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDepartures {
    /// The actual track information of the arriving transit vehicle.
    #[serde(rename = "actualTrack")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_track: Option<String>,
    /// Indicates if riders can arrive on this transit vehicle.
    #[serde(rename = "arrivalEnabled")]
    pub arrival_enabled: bool,
    /// Index of this arrival’s trip into the sequence of trips for the active block.
    #[serde(rename = "blockTripSequence")]
    pub block_trip_sequence: i64,
    /// Indicates if riders can depart from this transit vehicle.
    #[serde(rename = "departureEnabled")]
    pub departure_enabled: bool,
    /// Distance of the arriving transit vehicle from the stop, in meters.
    #[serde(rename = "distanceFromStop")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_from_stop: Option<f64>,
    /// Information about frequency-based scheduling, if applicable to the trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    /// Historical occupancy information of the transit vehicle.
    #[serde(rename = "historicalOccupancy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_occupancy: Option<String>,
    /// Timestamp of the last update time for this arrival.
    #[serde(rename = "lastUpdateTime")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update_time: Option<i64>,
    /// Number of stops between the arriving transit vehicle and the current stop (excluding the current stop).
    #[serde(rename = "numberOfStopsAway")]
    pub number_of_stops_away: i64,
    /// Current occupancy status of the transit vehicle.
    #[serde(rename = "occupancyStatus")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy_status: Option<String>,
    /// Indicates if real-time arrival info is available for this trip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted: Option<bool>,
    /// Interval for predicted arrival time, if available.
    #[serde(rename = "predictedArrivalInterval")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_arrival_interval: Option<String>,
    /// Predicted arrival time, in milliseconds since Unix epoch (zero if no real-time available).
    #[serde(rename = "predictedArrivalTime")]
    pub predicted_arrival_time: i64,
    /// Interval for predicted departure time, if available.
    #[serde(rename = "predictedDepartureInterval")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_departure_interval: Option<String>,
    /// Predicted departure time, in milliseconds since Unix epoch (zero if no real-time available).
    #[serde(rename = "predictedDepartureTime")]
    pub predicted_departure_time: i64,
    /// Predicted occupancy status of the transit vehicle.
    #[serde(rename = "predictedOccupancy")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_occupancy: Option<String>,
    /// The ID of the route for the arriving vehicle.
    #[serde(rename = "routeId")]
    pub route_id: String,
    /// Optional route long name that potentially overrides the route long name in the referenced route element.
    #[serde(rename = "routeLongName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_long_name: Option<String>,
    /// Optional route short name that potentially overrides the route short name in the referenced route element.
    #[serde(rename = "routeShortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_short_name: Option<String>,
    /// Interval for scheduled arrival time.
    #[serde(rename = "scheduledArrivalInterval")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_arrival_interval: Option<String>,
    /// Scheduled arrival time, in milliseconds since Unix epoch.
    #[serde(rename = "scheduledArrivalTime")]
    pub scheduled_arrival_time: i64,
    /// Interval for scheduled departure time.
    #[serde(rename = "scheduledDepartureInterval")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_departure_interval: Option<String>,
    /// Scheduled departure time, in milliseconds since Unix epoch.
    #[serde(rename = "scheduledDepartureTime")]
    pub scheduled_departure_time: i64,
    /// Scheduled track information of the arriving transit vehicle.
    #[serde(rename = "scheduledTrack")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_track: Option<String>,
    /// Time, in milliseconds since the Unix epoch, of midnight for the start of the service date for the trip.
    #[serde(rename = "serviceDate")]
    pub service_date: i64,
    /// References to situation elements (if any) applicable to this arrival.
    #[serde(rename = "situationIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub situation_ids: Option<Vec<String>>,
    /// Current status of the arrival.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// The ID of the stop the vehicle is arriving at.
    #[serde(rename = "stopId")]
    pub stop_id: String,
    /// Index of the stop into the sequence of stops that make up the trip for this arrival.
    #[serde(rename = "stopSequence")]
    pub stop_sequence: i64,
    /// Total number of stops visited on the trip for this arrival.
    #[serde(rename = "totalStopsInTrip")]
    pub total_stops_in_trip: i64,
    /// Optional trip headsign that potentially overrides the trip headsign in the referenced trip element.
    #[serde(rename = "tripHeadsign")]
    pub trip_headsign: String,
    /// The ID of the trip for the arriving vehicle.
    #[serde(rename = "tripId")]
    pub trip_id: String,
    /// Trip-specific status for the arriving transit vehicle.
    #[serde(rename = "tripStatus")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_status: Option<
        ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDeparturesTripStatus,
    >,
    /// ID of the transit vehicle serving this trip.
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

/// Trip-specific status for the arriving transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDeparturesTripStatus {
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
    pub last_known_location: Option<ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDeparturesTripStatusLastKnownLocation>,
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
    pub position: Option<ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDeparturesTripStatusPosition>,
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
pub struct ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDeparturesTripStatusLastKnownLocation
{
    /// Latitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// Current position of the transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponseDataEntryArrivalsAndDeparturesTripStatusPosition
{
    /// Latitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalsAndDeparturesForLocationListResponseDataEntryNearbyStopIds {
    #[serde(rename = "distanceFromQuery")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_from_query: Option<f64>,
    #[serde(rename = "stopId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
}
