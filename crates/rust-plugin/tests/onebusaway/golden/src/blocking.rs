//! The blocking client (`blocking` feature): the same resources and requests, whose `send`
//! blocks on a runtime the client owns.

use crate::rt::{Blocking, Result, blocking_runtime};

/// A client whose calls block until they are done.
#[derive(Debug)]
pub struct OnebusawaySdk {
    inner: crate::OnebusawaySdk,
    runtime: tokio::runtime::Runtime,
}

impl OnebusawaySdk {
    /// A client with every option read from the environment.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Config`] when a required option is not set or the runtime cannot be
    /// started.
    pub fn new() -> Result<Self> {
        crate::OnebusawaySdk::builder().build_blocking()
    }

    /// A blocking client that shares `inner`'s settings and connection pool.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Config`] when the runtime cannot be started.
    pub fn from_async(inner: crate::OnebusawaySdk) -> Result<Self> {
        Ok(Self {
            inner,
            runtime: blocking_runtime()?,
        })
    }

    /// The async client this one wraps, with the same settings.
    #[must_use]
    pub fn as_async(&self) -> &crate::OnebusawaySdk {
        &self.inner
    }

    /// The `agencies_with_coverage` resource.
    pub fn agencies_with_coverage(
        &self,
    ) -> crate::resources::agencies_with_coverage::AgenciesWithCoverageResource<'_, Blocking<'_>>
    {
        crate::resources::agencies_with_coverage::AgenciesWithCoverageResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `agency` resource.
    pub fn agency(&self) -> crate::resources::agency::AgencyResource<'_, Blocking<'_>> {
        crate::resources::agency::AgencyResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `vehicles_for_agency` resource.
    pub fn vehicles_for_agency(
        &self,
    ) -> crate::resources::vehicles_for_agency::VehiclesForAgencyResource<'_, Blocking<'_>> {
        crate::resources::vehicles_for_agency::VehiclesForAgencyResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `config` resource.
    pub fn config(&self) -> crate::resources::config::ConfigResource<'_, Blocking<'_>> {
        crate::resources::config::ConfigResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `current_time` resource.
    pub fn current_time(
        &self,
    ) -> crate::resources::current_time::CurrentTimeResource<'_, Blocking<'_>> {
        crate::resources::current_time::CurrentTimeResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `stops_for_location` resource.
    pub fn stops_for_location(
        &self,
    ) -> crate::resources::stops_for_location::StopsForLocationResource<'_, Blocking<'_>> {
        crate::resources::stops_for_location::StopsForLocationResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `stops_for_route` resource.
    pub fn stops_for_route(
        &self,
    ) -> crate::resources::stops_for_route::StopsForRouteResource<'_, Blocking<'_>> {
        crate::resources::stops_for_route::StopsForRouteResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `stops_for_agency` resource.
    pub fn stops_for_agency(
        &self,
    ) -> crate::resources::stops_for_agency::StopsForAgencyResource<'_, Blocking<'_>> {
        crate::resources::stops_for_agency::StopsForAgencyResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `stop` resource.
    pub fn stop(&self) -> crate::resources::stop::StopResource<'_, Blocking<'_>> {
        crate::resources::stop::StopResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `stop_ids_for_agency` resource.
    pub fn stop_ids_for_agency(
        &self,
    ) -> crate::resources::stop_ids_for_agency::StopIdsForAgencyResource<'_, Blocking<'_>> {
        crate::resources::stop_ids_for_agency::StopIdsForAgencyResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `schedule_for_stop` resource.
    pub fn schedule_for_stop(
        &self,
    ) -> crate::resources::schedule_for_stop::ScheduleForStopResource<'_, Blocking<'_>> {
        crate::resources::schedule_for_stop::ScheduleForStopResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `route` resource.
    pub fn route(&self) -> crate::resources::route::RouteResource<'_, Blocking<'_>> {
        crate::resources::route::RouteResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `route_ids_for_agency` resource.
    pub fn route_ids_for_agency(
        &self,
    ) -> crate::resources::route_ids_for_agency::RouteIdsForAgencyResource<'_, Blocking<'_>> {
        crate::resources::route_ids_for_agency::RouteIdsForAgencyResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `routes_for_location` resource.
    pub fn routes_for_location(
        &self,
    ) -> crate::resources::routes_for_location::RoutesForLocationResource<'_, Blocking<'_>> {
        crate::resources::routes_for_location::RoutesForLocationResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `routes_for_agency` resource.
    pub fn routes_for_agency(
        &self,
    ) -> crate::resources::routes_for_agency::RoutesForAgencyResource<'_, Blocking<'_>> {
        crate::resources::routes_for_agency::RoutesForAgencyResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `schedule_for_route` resource.
    pub fn schedule_for_route(
        &self,
    ) -> crate::resources::schedule_for_route::ScheduleForRouteResource<'_, Blocking<'_>> {
        crate::resources::schedule_for_route::ScheduleForRouteResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `arrivals_and_departures_for_location` resource.
    pub fn arrivals_and_departures_for_location(&self) -> crate::resources::arrivals_and_departures_for_location::ArrivalsAndDeparturesForLocationResource<'_, Blocking<'_>>{
        crate::resources::arrivals_and_departures_for_location::ArrivalsAndDeparturesForLocationResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `arrival_and_departure` resource.
    pub fn arrival_and_departure(
        &self,
    ) -> crate::resources::arrival_and_departure::ArrivalAndDepartureResource<'_, Blocking<'_>>
    {
        crate::resources::arrival_and_departure::ArrivalAndDepartureResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `trip` resource.
    pub fn trip(&self) -> crate::resources::trip::TripResource<'_, Blocking<'_>> {
        crate::resources::trip::TripResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `trips_for_location` resource.
    pub fn trips_for_location(
        &self,
    ) -> crate::resources::trips_for_location::TripsForLocationResource<'_, Blocking<'_>> {
        crate::resources::trips_for_location::TripsForLocationResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `trip-details` resource.
    pub fn trip_details(
        &self,
    ) -> crate::resources::trip_details::TripDetailsResource<'_, Blocking<'_>> {
        crate::resources::trip_details::TripDetailsResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `trip_for_vehicle` resource.
    pub fn trip_for_vehicle(
        &self,
    ) -> crate::resources::trip_for_vehicle::TripForVehicleResource<'_, Blocking<'_>> {
        crate::resources::trip_for_vehicle::TripForVehicleResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `trips_for_route` resource.
    pub fn trips_for_route(
        &self,
    ) -> crate::resources::trips_for_route::TripsForRouteResource<'_, Blocking<'_>> {
        crate::resources::trips_for_route::TripsForRouteResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `report_problem_with_stop` resource.
    pub fn report_problem_with_stop(
        &self,
    ) -> crate::resources::report_problem_with_stop::ReportProblemWithStopResource<'_, Blocking<'_>>
    {
        crate::resources::report_problem_with_stop::ReportProblemWithStopResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `report_problem_with_trip` resource.
    pub fn report_problem_with_trip(
        &self,
    ) -> crate::resources::report_problem_with_trip::ReportProblemWithTripResource<'_, Blocking<'_>>
    {
        crate::resources::report_problem_with_trip::ReportProblemWithTripResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `search_for_stop` resource.
    pub fn search_for_stop(
        &self,
    ) -> crate::resources::search_for_stop::SearchForStopResource<'_, Blocking<'_>> {
        crate::resources::search_for_stop::SearchForStopResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `search_for_route` resource.
    pub fn search_for_route(
        &self,
    ) -> crate::resources::search_for_route::SearchForRouteResource<'_, Blocking<'_>> {
        crate::resources::search_for_route::SearchForRouteResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `block` resource.
    pub fn block(&self) -> crate::resources::block::BlockResource<'_, Blocking<'_>> {
        crate::resources::block::BlockResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `shape` resource.
    pub fn shape(&self) -> crate::resources::shape::ShapeResource<'_, Blocking<'_>> {
        crate::resources::shape::ShapeResource::new(&self.inner.core, Blocking(&self.runtime))
    }
}
