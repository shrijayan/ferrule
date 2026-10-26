//! The `arrival_and_departure` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `arrival_and_departure` resource.
#[derive(Debug, Clone, Copy)]
pub struct ArrivalAndDepartureResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ArrivalAndDepartureResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// arrivals-and-departures-for-stop
    pub fn list(
        &self,
        stop_id: impl Into<String>,
    ) -> Request<'a, ArrivalAndDepartureListParams, M> {
        let params = ArrivalAndDepartureListParams {
            stop_id: stop_id.into(),
            minutes_before: None,
            minutes_after: None,
            time: None,
        };
        Request::new(self.core, params, self.mode)
    }

    /// arrival-and-departure-for-stop
    pub fn retrieve(
        &self,
        stop_id: impl Into<String>,
        trip_id: impl Into<String>,
        service_date: i64,
    ) -> Request<'a, ArrivalAndDepartureRetrieveParams, M> {
        let params = ArrivalAndDepartureRetrieveParams {
            stop_id: stop_id.into(),
            trip_id: trip_id.into(),
            service_date,
            vehicle_id: None,
            stop_sequence: None,
            time: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/arrivals-and-departures-for-stop/{stopID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrivalAndDepartureListParams {
    pub stop_id: String,
    /// Include vehicles having arrived or departed in the previous n minutes.
    pub minutes_before: Option<i64>,
    /// Include vehicles arriving or departing in the next n minutes.
    pub minutes_after: Option<i64>,
    /// The specific time for querying the system status.
    pub time: Option<String>,
}

impl<M> Request<'_, ArrivalAndDepartureListParams, M> {
    /// Include vehicles having arrived or departed in the previous n minutes.
    pub fn minutes_before(mut self, minutes_before: i64) -> Self {
        self.params_mut().minutes_before = Some(minutes_before);
        self
    }

    /// Include vehicles arriving or departing in the next n minutes.
    pub fn minutes_after(mut self, minutes_after: i64) -> Self {
        self.params_mut().minutes_after = Some(minutes_after);
        self
    }

    /// The specific time for querying the system status.
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.params_mut().time = Some(time.into());
        self
    }
}

impl Endpoint for ArrivalAndDepartureListParams {
    type Output = ArrivalAndDepartureListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/arrivals-and-departures-for-stop/{}.json",
                path_param(&self.stop_id)?
            ),
        );
        spec.query("minutesBefore", &self.minutes_before)?;
        spec.query("minutesAfter", &self.minutes_after)?;
        spec.query("time", &self.time)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ArrivalAndDepartureListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureListResponseData {
    pub entry: ArrivalAndDepartureListResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureListResponseDataEntry {
    #[serde(rename = "arrivalsAndDepartures")]
    pub arrivals_and_departures: Vec<ArrivalAndDepartureListResponseDataEntryArrivalsAndDepartures>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureListResponseDataEntryArrivalsAndDepartures {
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
    pub trip_status:
        Option<ArrivalAndDepartureListResponseDataEntryArrivalsAndDeparturesTripStatus>,
    /// ID of the transit vehicle serving this trip.
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

/// Trip-specific status for the arriving transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureListResponseDataEntryArrivalsAndDeparturesTripStatus {
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
    pub last_known_location: Option<
        ArrivalAndDepartureListResponseDataEntryArrivalsAndDeparturesTripStatusLastKnownLocation,
    >,
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
    pub position:
        Option<ArrivalAndDepartureListResponseDataEntryArrivalsAndDeparturesTripStatusPosition>,
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
pub struct ArrivalAndDepartureListResponseDataEntryArrivalsAndDeparturesTripStatusLastKnownLocation
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
pub struct ArrivalAndDepartureListResponseDataEntryArrivalsAndDeparturesTripStatusPosition {
    /// Latitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// The params of `retrieve` (GET `/api/where/arrival-and-departure-for-stop/{stopID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrivalAndDepartureRetrieveParams {
    pub stop_id: String,
    pub trip_id: String,
    pub service_date: i64,
    pub vehicle_id: Option<String>,
    pub stop_sequence: Option<i64>,
    pub time: Option<i64>,
}

impl<M> Request<'_, ArrivalAndDepartureRetrieveParams, M> {
    pub fn vehicle_id(mut self, vehicle_id: impl Into<String>) -> Self {
        self.params_mut().vehicle_id = Some(vehicle_id.into());
        self
    }

    pub fn stop_sequence(mut self, stop_sequence: i64) -> Self {
        self.params_mut().stop_sequence = Some(stop_sequence);
        self
    }

    pub fn time(mut self, time: i64) -> Self {
        self.params_mut().time = Some(time);
        self
    }
}

impl Endpoint for ArrivalAndDepartureRetrieveParams {
    type Output = ArrivalAndDepartureRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/arrival-and-departure-for-stop/{}.json",
                path_param(&self.stop_id)?
            ),
        );
        spec.query("tripId", &self.trip_id)?;
        spec.query("serviceDate", &self.service_date)?;
        spec.query("vehicleId", &self.vehicle_id)?;
        spec.query("stopSequence", &self.stop_sequence)?;
        spec.query("time", &self.time)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ArrivalAndDepartureRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureRetrieveResponseData {
    pub entry: ArrivalAndDepartureRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureRetrieveResponseDataEntry {
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
    pub trip_status: Option<ArrivalAndDepartureRetrieveResponseDataEntryTripStatus>,
    /// ID of the transit vehicle serving this trip.
    #[serde(rename = "vehicleId")]
    pub vehicle_id: String,
}

/// Trip-specific status for the arriving transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureRetrieveResponseDataEntryTripStatus {
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
        Option<ArrivalAndDepartureRetrieveResponseDataEntryTripStatusLastKnownLocation>,
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
    pub position: Option<ArrivalAndDepartureRetrieveResponseDataEntryTripStatusPosition>,
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
pub struct ArrivalAndDepartureRetrieveResponseDataEntryTripStatusLastKnownLocation {
    /// Latitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the last known location of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// Current position of the transit vehicle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalAndDepartureRetrieveResponseDataEntryTripStatusPosition {
    /// Latitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// Longitude of the current position of the transit vehicle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}
