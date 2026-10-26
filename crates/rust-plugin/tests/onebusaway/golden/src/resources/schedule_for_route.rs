//! The `schedule_for_route` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `schedule_for_route` resource.
#[derive(Debug, Clone, Copy)]
pub struct ScheduleForRouteResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ScheduleForRouteResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Retrieve the full schedule for a route on a particular day
    pub fn retrieve(
        &self,
        route_id: impl Into<String>,
    ) -> Request<'a, ScheduleForRouteRetrieveParams, M> {
        let params = ScheduleForRouteRetrieveParams {
            route_id: route_id.into(),
            date: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/schedule-for-route/{routeID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleForRouteRetrieveParams {
    /// The route id to request the schedule for
    pub route_id: String,
    /// The date for which you want to request a schedule in the format YYYY-MM-DD (optional, defaults to current date)
    pub date: Option<String>,
}

impl<M> Request<'_, ScheduleForRouteRetrieveParams, M> {
    /// The date for which you want to request a schedule in the format YYYY-MM-DD (optional, defaults to current date)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.params_mut().date = Some(date.into());
        self
    }
}

impl Endpoint for ScheduleForRouteRetrieveParams {
    type Output = ScheduleForRouteRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/schedule-for-route/{}.json",
                path_param(&self.route_id)
            ),
        );
        spec.query("date", &self.date)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForRouteRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ScheduleForRouteRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForRouteRetrieveResponseData {
    pub entry: ScheduleForRouteRetrieveResponseDataEntry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForRouteRetrieveResponseDataEntry {
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "scheduleDate")]
    pub schedule_date: i64,
    #[serde(rename = "serviceIds")]
    pub service_ids: Vec<String>,
    #[serde(rename = "stopTripGroupings")]
    pub stop_trip_groupings: Vec<ScheduleForRouteRetrieveResponseDataEntryStopTripGroupings>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForRouteRetrieveResponseDataEntryStopTripGroupings {
    #[serde(rename = "directionId")]
    pub direction_id: String,
    #[serde(rename = "stopIds")]
    pub stop_ids: Vec<String>,
    #[serde(rename = "tripHeadsigns")]
    pub trip_headsigns: Vec<String>,
    #[serde(rename = "tripIds")]
    pub trip_ids: Vec<String>,
    #[serde(rename = "tripsWithStopTimes")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trips_with_stop_times:
        Option<Vec<ScheduleForRouteRetrieveResponseDataEntryStopTripGroupingsTripsWithStopTimes>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForRouteRetrieveResponseDataEntryStopTripGroupingsTripsWithStopTimes {
    #[serde(rename = "stopTimes")]
    pub stop_times:
        Vec<ScheduleForRouteRetrieveResponseDataEntryStopTripGroupingsTripsWithStopTimesStopTimes>,
    #[serde(rename = "tripId")]
    pub trip_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForRouteRetrieveResponseDataEntryStopTripGroupingsTripsWithStopTimesStopTimes {
    #[serde(rename = "arrivalEnabled")]
    pub arrival_enabled: bool,
    #[serde(rename = "arrivalTime")]
    pub arrival_time: i64,
    #[serde(rename = "departureEnabled")]
    pub departure_enabled: bool,
    #[serde(rename = "departureTime")]
    pub departure_time: i64,
    #[serde(rename = "serviceId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "stopHeadsign")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_headsign: Option<String>,
    #[serde(rename = "stopId")]
    pub stop_id: String,
    #[serde(rename = "tripId")]
    pub trip_id: String,
}
