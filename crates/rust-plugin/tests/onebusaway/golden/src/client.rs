//! The `OnebusawaySdk` client and its builder.

use crate::rt::{ArrayFormat, Async, Config, Core, Error, NestedFormat, Result, from_env};

/// The servers the API runs on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Environment {
    /// `https://api.pugetsound.onebusaway.org`
    #[default]
    Production,
}

impl Environment {
    /// The base URL of the environment.
    #[must_use]
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Production => "https://api.pugetsound.onebusaway.org",
        }
    }
}

/// A client for the API. Cloning it is cheap: clones share one connection pool.
#[derive(Debug, Clone)]
pub struct OnebusawaySdk {
    pub(crate) core: std::sync::Arc<Core>,
}

impl OnebusawaySdk {
    /// A client with every option read from the environment.
    ///
    /// # Errors
    ///
    /// [`Error::Config`] when a required option is not set.
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// A builder, to set options before the environment is read.
    pub fn builder() -> OnebusawaySdkBuilder {
        OnebusawaySdkBuilder::default()
    }

    /// The `agencies_with_coverage` resource.
    pub fn agencies_with_coverage(
        &self,
    ) -> crate::resources::agencies_with_coverage::AgenciesWithCoverageResource<'_> {
        crate::resources::agencies_with_coverage::AgenciesWithCoverageResource::new(
            &self.core, Async,
        )
    }

    /// The `agency` resource.
    pub fn agency(&self) -> crate::resources::agency::AgencyResource<'_> {
        crate::resources::agency::AgencyResource::new(&self.core, Async)
    }

    /// The `vehicles_for_agency` resource.
    pub fn vehicles_for_agency(
        &self,
    ) -> crate::resources::vehicles_for_agency::VehiclesForAgencyResource<'_> {
        crate::resources::vehicles_for_agency::VehiclesForAgencyResource::new(&self.core, Async)
    }

    /// The `config` resource.
    pub fn config(&self) -> crate::resources::config::ConfigResource<'_> {
        crate::resources::config::ConfigResource::new(&self.core, Async)
    }

    /// The `current_time` resource.
    pub fn current_time(&self) -> crate::resources::current_time::CurrentTimeResource<'_> {
        crate::resources::current_time::CurrentTimeResource::new(&self.core, Async)
    }

    /// The `stops_for_location` resource.
    pub fn stops_for_location(
        &self,
    ) -> crate::resources::stops_for_location::StopsForLocationResource<'_> {
        crate::resources::stops_for_location::StopsForLocationResource::new(&self.core, Async)
    }

    /// The `stops_for_route` resource.
    pub fn stops_for_route(&self) -> crate::resources::stops_for_route::StopsForRouteResource<'_> {
        crate::resources::stops_for_route::StopsForRouteResource::new(&self.core, Async)
    }

    /// The `stops_for_agency` resource.
    pub fn stops_for_agency(
        &self,
    ) -> crate::resources::stops_for_agency::StopsForAgencyResource<'_> {
        crate::resources::stops_for_agency::StopsForAgencyResource::new(&self.core, Async)
    }

    /// The `stop` resource.
    pub fn stop(&self) -> crate::resources::stop::StopResource<'_> {
        crate::resources::stop::StopResource::new(&self.core, Async)
    }

    /// The `stop_ids_for_agency` resource.
    pub fn stop_ids_for_agency(
        &self,
    ) -> crate::resources::stop_ids_for_agency::StopIdsForAgencyResource<'_> {
        crate::resources::stop_ids_for_agency::StopIdsForAgencyResource::new(&self.core, Async)
    }

    /// The `schedule_for_stop` resource.
    pub fn schedule_for_stop(
        &self,
    ) -> crate::resources::schedule_for_stop::ScheduleForStopResource<'_> {
        crate::resources::schedule_for_stop::ScheduleForStopResource::new(&self.core, Async)
    }

    /// The `route` resource.
    pub fn route(&self) -> crate::resources::route::RouteResource<'_> {
        crate::resources::route::RouteResource::new(&self.core, Async)
    }

    /// The `route_ids_for_agency` resource.
    pub fn route_ids_for_agency(
        &self,
    ) -> crate::resources::route_ids_for_agency::RouteIdsForAgencyResource<'_> {
        crate::resources::route_ids_for_agency::RouteIdsForAgencyResource::new(&self.core, Async)
    }

    /// The `routes_for_location` resource.
    pub fn routes_for_location(
        &self,
    ) -> crate::resources::routes_for_location::RoutesForLocationResource<'_> {
        crate::resources::routes_for_location::RoutesForLocationResource::new(&self.core, Async)
    }

    /// The `routes_for_agency` resource.
    pub fn routes_for_agency(
        &self,
    ) -> crate::resources::routes_for_agency::RoutesForAgencyResource<'_> {
        crate::resources::routes_for_agency::RoutesForAgencyResource::new(&self.core, Async)
    }

    /// The `schedule_for_route` resource.
    pub fn schedule_for_route(
        &self,
    ) -> crate::resources::schedule_for_route::ScheduleForRouteResource<'_> {
        crate::resources::schedule_for_route::ScheduleForRouteResource::new(&self.core, Async)
    }

    /// The `arrivals_and_departures_for_location` resource.
    pub fn arrivals_and_departures_for_location(&self) -> crate::resources::arrivals_and_departures_for_location::ArrivalsAndDeparturesForLocationResource<'_>{
        crate::resources::arrivals_and_departures_for_location::ArrivalsAndDeparturesForLocationResource::new(&self.core, Async)
    }

    /// The `arrival_and_departure` resource.
    pub fn arrival_and_departure(
        &self,
    ) -> crate::resources::arrival_and_departure::ArrivalAndDepartureResource<'_> {
        crate::resources::arrival_and_departure::ArrivalAndDepartureResource::new(&self.core, Async)
    }

    /// The `trip` resource.
    pub fn trip(&self) -> crate::resources::trip::TripResource<'_> {
        crate::resources::trip::TripResource::new(&self.core, Async)
    }

    /// The `trips_for_location` resource.
    pub fn trips_for_location(
        &self,
    ) -> crate::resources::trips_for_location::TripsForLocationResource<'_> {
        crate::resources::trips_for_location::TripsForLocationResource::new(&self.core, Async)
    }

    /// The `trip-details` resource.
    pub fn trip_details(&self) -> crate::resources::trip_details::TripDetailsResource<'_> {
        crate::resources::trip_details::TripDetailsResource::new(&self.core, Async)
    }

    /// The `trip_for_vehicle` resource.
    pub fn trip_for_vehicle(
        &self,
    ) -> crate::resources::trip_for_vehicle::TripForVehicleResource<'_> {
        crate::resources::trip_for_vehicle::TripForVehicleResource::new(&self.core, Async)
    }

    /// The `trips_for_route` resource.
    pub fn trips_for_route(&self) -> crate::resources::trips_for_route::TripsForRouteResource<'_> {
        crate::resources::trips_for_route::TripsForRouteResource::new(&self.core, Async)
    }

    /// The `report_problem_with_stop` resource.
    pub fn report_problem_with_stop(
        &self,
    ) -> crate::resources::report_problem_with_stop::ReportProblemWithStopResource<'_> {
        crate::resources::report_problem_with_stop::ReportProblemWithStopResource::new(
            &self.core, Async,
        )
    }

    /// The `report_problem_with_trip` resource.
    pub fn report_problem_with_trip(
        &self,
    ) -> crate::resources::report_problem_with_trip::ReportProblemWithTripResource<'_> {
        crate::resources::report_problem_with_trip::ReportProblemWithTripResource::new(
            &self.core, Async,
        )
    }

    /// The `search_for_stop` resource.
    pub fn search_for_stop(&self) -> crate::resources::search_for_stop::SearchForStopResource<'_> {
        crate::resources::search_for_stop::SearchForStopResource::new(&self.core, Async)
    }

    /// The `search_for_route` resource.
    pub fn search_for_route(
        &self,
    ) -> crate::resources::search_for_route::SearchForRouteResource<'_> {
        crate::resources::search_for_route::SearchForRouteResource::new(&self.core, Async)
    }

    /// The `block` resource.
    pub fn block(&self) -> crate::resources::block::BlockResource<'_> {
        crate::resources::block::BlockResource::new(&self.core, Async)
    }

    /// The `shape` resource.
    pub fn shape(&self) -> crate::resources::shape::ShapeResource<'_> {
        crate::resources::shape::ShapeResource::new(&self.core, Async)
    }
}

/// Options of a [`OnebusawaySdk`]; each option not set is read from the environment.
#[derive(Debug, Clone, Default)]
pub struct OnebusawaySdkBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    environment: Option<Environment>,
    timeout: Option<std::time::Duration>,
    max_retries: Option<u32>,
    headers: Vec<(String, String)>,
    http_client: Option<reqwest::Client>,
}

impl OnebusawaySdkBuilder {
    /// Sets `api_key`.
    ///
    /// Read from `ONEBUSAWAY_API_KEY` when not set.
    pub fn api_key(mut self, value: impl Into<String>) -> Self {
        self.api_key = Some(value.into());
        self
    }

    /// The URL every request goes to, instead of the environment's.
    ///
    /// Read from `ONEBUSAWAY_SDK_BASE_URL` when not set.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// The environment to send requests to.
    pub fn environment(mut self, environment: Environment) -> Self {
        self.environment = Some(environment);
        self
    }

    /// How long to wait for a response; 60 seconds when not set.
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// How many times to retry a failed request; 2 when not set.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// A header to send with every request.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// The HTTP client to send requests with, instead of a new one.
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// The client.
    ///
    /// # Errors
    ///
    /// [`Error::Config`] when a required option is not set, or both `base_url` and
    /// `environment` are.
    pub fn build(self) -> Result<OnebusawaySdk> {
        Ok(OnebusawaySdk {
            core: std::sync::Arc::new(self.core()?),
        })
    }

    /// The blocking client.
    ///
    /// # Errors
    ///
    /// As [`Self::build`], and [`Error::Config`] when its runtime cannot be started.
    #[cfg(feature = "blocking")]
    pub fn build_blocking(self) -> Result<crate::blocking::OnebusawaySdk> {
        crate::blocking::OnebusawaySdk::from_async(self.build()?)
    }

    fn core(self) -> Result<Core> {
        let mut headers: Vec<(String, String)> = Vec::new();
        let mut query: Vec<(String, String)> = Vec::new();
        let api_key = from_env(self.api_key, "ONEBUSAWAY_API_KEY").ok_or_else(|| {
            Error::Config("missing api_key: call `api_key` or set ONEBUSAWAY_API_KEY".to_owned())
        })?;
        let base_url = match (
            from_env(self.base_url, "ONEBUSAWAY_SDK_BASE_URL"),
            self.environment,
        ) {
            (Some(_), Some(_)) => {
                return Err(Error::Config(
                    "set base_url or environment, not both".to_owned(),
                ));
            }
            (Some(url), None) => url,
            (None, environment) => environment.unwrap_or_default().base_url().to_owned(),
        };
        query.push(("key".to_owned(), api_key));
        headers.extend(self.headers);
        let config = Config {
            base_url,
            timeout: self
                .timeout
                .unwrap_or(std::time::Duration::from_secs_f64(60.0)),
            max_retries: self.max_retries.unwrap_or(2),
            initial_retry_delay: std::time::Duration::from_secs_f64(0.5),
            max_retry_delay: std::time::Duration::from_secs_f64(8.0),
            headers,
            query,
            idempotency_header: None,
            user_agent: concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")).to_owned(),
            array_format: ArrayFormat::Repeat,
            nested_format: NestedFormat::Brackets,
        };
        Core::new(config, self.http_client)
    }
}
