//! Models several resources share.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct References {
    pub agencies: Vec<ReferencesAgencies>,
    pub routes: Vec<ReferencesRoutes>,
    pub situations: Vec<ReferencesSituations>,
    #[serde(rename = "stopTimes")]
    pub stop_times: Vec<ReferencesStopTimes>,
    pub stops: Vec<ReferencesStops>,
    pub trips: Vec<ReferencesTrips>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesAgencies {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclaimer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "fareUrl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fare_url: Option<String>,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(rename = "privateService")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_service: Option<bool>,
    pub timezone: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesRoutes {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituations {
    #[serde(rename = "activeWindows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_windows: Option<Vec<ReferencesSituationsActiveWindows>>,
    #[serde(rename = "allAffects")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_affects: Option<Vec<ReferencesSituationsAllAffects>>,
    /// Message regarding the consequence of the situation.
    #[serde(rename = "consequenceMessage")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consequence_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consequences: Option<Vec<ReferencesSituationsConsequences>>,
    /// Unix timestamp of when this situation was created.
    #[serde(rename = "creationTime")]
    pub creation_time: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<ReferencesSituationsDescription>,
    /// Unique identifier for the situation.
    pub id: String,
    #[serde(rename = "publicationWindows")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_windows: Option<Vec<ReferencesSituationsPublicationWindows>>,
    /// Reason for the service alert, taken from TPEG codes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Severity of the situation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<ReferencesSituationsSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<ReferencesSituationsUrl>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsActiveWindows {
    /// Start time of the active window as a Unix timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<i64>,
    /// End time of the active window as a Unix timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsAllAffects {
    /// Identifier for the agency.
    #[serde(rename = "agencyId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agency_id: Option<String>,
    /// Identifier for the application.
    #[serde(rename = "applicationId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
    /// Identifier for the direction.
    #[serde(rename = "directionId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction_id: Option<String>,
    /// Identifier for the route.
    #[serde(rename = "routeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    /// Identifier for the stop.
    #[serde(rename = "stopId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_id: Option<String>,
    /// Identifier for the trip.
    #[serde(rename = "tripId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsConsequences {
    /// Condition of the consequence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(rename = "conditionDetails")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition_details: Option<ReferencesSituationsConsequencesConditionDetails>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsConsequencesConditionDetails {
    #[serde(rename = "diversionPath")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diversion_path: Option<ReferencesSituationsConsequencesConditionDetailsDiversionPath>,
    #[serde(rename = "diversionStopIds")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diversion_stop_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsConsequencesConditionDetailsDiversionPath {
    /// Length of the diversion path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
    /// Levels of the diversion path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<String>,
    /// Points of the diversion path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsDescription {
    /// Language of the description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Longer description of the situation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsPublicationWindows {
    /// Start time of the time window as a Unix timestamp.
    pub from: i64,
    /// End time of the time window as a Unix timestamp.
    pub to: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsSummary {
    /// Language of the summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Short summary of the situation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesSituationsUrl {
    /// Language of the URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// URL for more information about the situation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesStopTimes {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesStops {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencesTrips {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseWrapper {
    pub code: i64,
    #[serde(rename = "currentTime")]
    pub current_time: i64,
    pub text: String,
    pub version: i64,
}
