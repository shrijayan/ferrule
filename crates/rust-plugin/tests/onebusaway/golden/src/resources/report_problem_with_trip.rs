//! The `report_problem_with_trip` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `report_problem_with_trip` resource.
#[derive(Debug, Clone, Copy)]
pub struct ReportProblemWithTripResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ReportProblemWithTripResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Submit a user-generated problem report for a particular trip.
    pub fn retrieve(
        &self,
        trip_id: impl Into<String>,
    ) -> Request<'a, ReportProblemWithTripRetrieveParams, M> {
        let params = ReportProblemWithTripRetrieveParams {
            trip_id: trip_id.into(),
            service_date: None,
            vehicle_id: None,
            stop_id: None,
            code: None,
            user_comment: None,
            user_on_vehicle: None,
            user_vehicle_number: None,
            user_lat: None,
            user_lon: None,
            user_location_accuracy: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/report-problem-with-trip/{tripID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReportProblemWithTripRetrieveParams {
    /// The ID of the trip
    pub trip_id: String,
    /// The service date of the trip
    pub service_date: Option<i64>,
    /// The vehicle actively serving the trip
    pub vehicle_id: Option<String>,
    /// A stop ID indicating where the user is experiencing the problem
    pub stop_id: Option<String>,
    /// A string code identifying the nature of the problem
    pub code: Option<ReportProblemWithTripRetrieveCode>,
    /// Additional comment text supplied by the user describing the problem
    pub user_comment: Option<String>,
    /// Indicator if the user is on the transit vehicle experiencing the problem
    pub user_on_vehicle: Option<bool>,
    /// The vehicle number, as reported by the user
    pub user_vehicle_number: Option<String>,
    /// The reporting user’s current latitude
    pub user_lat: Option<f32>,
    /// The reporting user’s current longitude
    pub user_lon: Option<f32>,
    /// The reporting user’s location accuracy, in meters
    pub user_location_accuracy: Option<f32>,
}

impl<M> Request<'_, ReportProblemWithTripRetrieveParams, M> {
    /// The service date of the trip
    pub fn service_date(mut self, service_date: i64) -> Self {
        self.params_mut().service_date = Some(service_date);
        self
    }

    /// The vehicle actively serving the trip
    pub fn vehicle_id(mut self, vehicle_id: impl Into<String>) -> Self {
        self.params_mut().vehicle_id = Some(vehicle_id.into());
        self
    }

    /// A stop ID indicating where the user is experiencing the problem
    pub fn stop_id(mut self, stop_id: impl Into<String>) -> Self {
        self.params_mut().stop_id = Some(stop_id.into());
        self
    }

    /// A string code identifying the nature of the problem
    pub fn code(mut self, code: ReportProblemWithTripRetrieveCode) -> Self {
        self.params_mut().code = Some(code);
        self
    }

    /// Additional comment text supplied by the user describing the problem
    pub fn user_comment(mut self, user_comment: impl Into<String>) -> Self {
        self.params_mut().user_comment = Some(user_comment.into());
        self
    }

    /// Indicator if the user is on the transit vehicle experiencing the problem
    pub fn user_on_vehicle(mut self, user_on_vehicle: bool) -> Self {
        self.params_mut().user_on_vehicle = Some(user_on_vehicle);
        self
    }

    /// The vehicle number, as reported by the user
    pub fn user_vehicle_number(mut self, user_vehicle_number: impl Into<String>) -> Self {
        self.params_mut().user_vehicle_number = Some(user_vehicle_number.into());
        self
    }

    /// The reporting user’s current latitude
    pub fn user_lat(mut self, user_lat: f32) -> Self {
        self.params_mut().user_lat = Some(user_lat);
        self
    }

    /// The reporting user’s current longitude
    pub fn user_lon(mut self, user_lon: f32) -> Self {
        self.params_mut().user_lon = Some(user_lon);
        self
    }

    /// The reporting user’s location accuracy, in meters
    pub fn user_location_accuracy(mut self, user_location_accuracy: f32) -> Self {
        self.params_mut().user_location_accuracy = Some(user_location_accuracy);
        self
    }
}

impl Endpoint for ReportProblemWithTripRetrieveParams {
    type Output = crate::shared::ResponseWrapper;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/report-problem-with-trip/{}.json",
                path_param(&self.trip_id)?
            ),
        );
        spec.query("serviceDate", &self.service_date)?;
        spec.query("vehicleID", &self.vehicle_id)?;
        spec.query("stopID", &self.stop_id)?;
        spec.query("code", &self.code)?;
        spec.query("userComment", &self.user_comment)?;
        spec.query("userOnVehicle", &self.user_on_vehicle)?;
        spec.query("userVehicleNumber", &self.user_vehicle_number)?;
        spec.query("userLat", &self.user_lat)?;
        spec.query("userLon", &self.user_lon)?;
        spec.query("userLocationAccuracy", &self.user_location_accuracy)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

/// A string code identifying the nature of the problem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReportProblemWithTripRetrieveCode {
    #[serde(rename = "vehicle_never_came")]
    VehicleNeverCame,
    #[serde(rename = "vehicle_came_early")]
    VehicleCameEarly,
    #[serde(rename = "vehicle_came_late")]
    VehicleCameLate,
    #[serde(rename = "wrong_headsign")]
    WrongHeadsign,
    #[serde(rename = "vehicle_does_not_stop_here")]
    VehicleDoesNotStopHere,
    #[serde(rename = "other")]
    Other,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}
