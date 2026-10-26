//! The `schedule_for_stop` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `schedule_for_stop` resource.
#[derive(Debug, Clone, Copy)]
pub struct ScheduleForStopResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ScheduleForStopResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Get schedule for a specific stop
    pub fn retrieve(
        &self,
        stop_id: impl Into<String>,
    ) -> Request<'a, ScheduleForStopRetrieveParams, M> {
        let params = ScheduleForStopRetrieveParams {
            stop_id: stop_id.into(),
            date: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/schedule-for-stop/{stopID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleForStopRetrieveParams {
    /// The stop id to request the schedule for, encoded directly in the URL
    pub stop_id: String,
    /// The date for which you want to request a schedule in the format YYYY-MM-DD (optional, defaults to the current date)
    pub date: Option<String>,
}

impl<M> Request<'_, ScheduleForStopRetrieveParams, M> {
    /// The date for which you want to request a schedule in the format YYYY-MM-DD (optional, defaults to the current date)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.params_mut().date = Some(date.into());
        self
    }
}

impl Endpoint for ScheduleForStopRetrieveParams {
    type Output = ScheduleForStopRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/schedule-for-stop/{}.json",
                path_param(&self.stop_id)
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
pub struct ScheduleForStopRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ScheduleForStopRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForStopRetrieveResponseData {
    pub entry: ScheduleForStopRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForStopRetrieveResponseDataEntry {
    pub date: i64,
    #[serde(rename = "stopId")]
    pub stop_id: String,
    #[serde(rename = "stopRouteSchedules")]
    pub stop_route_schedules: Vec<ScheduleForStopRetrieveResponseDataEntryStopRouteSchedules>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForStopRetrieveResponseDataEntryStopRouteSchedules {
    #[serde(rename = "routeId")]
    pub route_id: String,
    #[serde(rename = "stopRouteDirectionSchedules")]
    pub stop_route_direction_schedules:
        Vec<ScheduleForStopRetrieveResponseDataEntryStopRouteSchedulesStopRouteDirectionSchedules>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForStopRetrieveResponseDataEntryStopRouteSchedulesStopRouteDirectionSchedules {
    #[serde(rename = "scheduleFrequencies")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_frequencies: Option<Vec<ScheduleForStopRetrieveResponseDataEntryStopRouteSchedulesStopRouteDirectionSchedulesScheduleFrequencies>>,
    #[serde(rename = "scheduleStopTimes")]
    pub schedule_stop_times: Vec<ScheduleForStopRetrieveResponseDataEntryStopRouteSchedulesStopRouteDirectionSchedulesScheduleStopTimes>,
    #[serde(rename = "tripHeadsign")]
    pub trip_headsign: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForStopRetrieveResponseDataEntryStopRouteSchedulesStopRouteDirectionSchedulesScheduleFrequencies
{
    #[serde(rename = "endTime")]
    pub end_time: i64,
    pub headway: i32,
    #[serde(rename = "serviceDate")]
    pub service_date: i64,
    #[serde(rename = "serviceId")]
    pub service_id: String,
    #[serde(rename = "startTime")]
    pub start_time: i64,
    #[serde(rename = "tripId")]
    pub trip_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleForStopRetrieveResponseDataEntryStopRouteSchedulesStopRouteDirectionSchedulesScheduleStopTimes
{
    #[serde(rename = "arrivalEnabled")]
    pub arrival_enabled: bool,
    #[serde(rename = "arrivalTime")]
    pub arrival_time: i64,
    #[serde(rename = "departureEnabled")]
    pub departure_enabled: bool,
    #[serde(rename = "departureTime")]
    pub departure_time: i64,
    #[serde(rename = "serviceId")]
    pub service_id: String,
    #[serde(rename = "stopHeadsign")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_headsign: Option<String>,
    #[serde(rename = "tripId")]
    pub trip_id: String,
}
