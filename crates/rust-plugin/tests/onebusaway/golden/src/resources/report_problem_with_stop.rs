//! The `report_problem_with_stop` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `report_problem_with_stop` resource.
#[derive(Debug, Clone, Copy)]
pub struct ReportProblemWithStopResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ReportProblemWithStopResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Submit a user-generated problem report for a stop
    pub fn retrieve(
        &self,
        stop_id: impl Into<String>,
    ) -> Request<'a, ReportProblemWithStopRetrieveParams, M> {
        let params = ReportProblemWithStopRetrieveParams {
            stop_id: stop_id.into(),
            code: None,
            user_comment: None,
            user_lat: None,
            user_lon: None,
            user_location_accuracy: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/report-problem-with-stop/{stopID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReportProblemWithStopRetrieveParams {
    /// The ID of the stop
    pub stop_id: String,
    /// A string code identifying the nature of the problem
    pub code: Option<ReportProblemWithStopRetrieveCode>,
    /// Additional comment text supplied by the user describing the problem
    pub user_comment: Option<String>,
    /// The reporting user’s current latitude
    pub user_lat: Option<f32>,
    /// The reporting user’s current longitude
    pub user_lon: Option<f32>,
    /// The reporting user’s location accuracy, in meters
    pub user_location_accuracy: Option<f32>,
}

impl<M> Request<'_, ReportProblemWithStopRetrieveParams, M> {
    /// A string code identifying the nature of the problem
    pub fn code(mut self, code: ReportProblemWithStopRetrieveCode) -> Self {
        self.params_mut().code = Some(code);
        self
    }

    /// Additional comment text supplied by the user describing the problem
    pub fn user_comment(mut self, user_comment: impl Into<String>) -> Self {
        self.params_mut().user_comment = Some(user_comment.into());
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

impl Endpoint for ReportProblemWithStopRetrieveParams {
    type Output = crate::shared::ResponseWrapper;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/report-problem-with-stop/{}.json",
                path_param(&self.stop_id)?
            ),
        );
        spec.query("code", &self.code)?;
        spec.query("userComment", &self.user_comment)?;
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
pub enum ReportProblemWithStopRetrieveCode {
    #[serde(rename = "stop_name_wrong")]
    StopNameWrong,
    #[serde(rename = "stop_number_wrong")]
    StopNumberWrong,
    #[serde(rename = "stop_location_wrong")]
    StopLocationWrong,
    #[serde(rename = "route_or_trip_missing")]
    RouteOrTripMissing,
    #[serde(rename = "other")]
    Other,
    /// A value this version of the SDK does not know.
    #[serde(other, skip_serializing)]
    Unknown,
}
