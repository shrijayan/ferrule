//! Every method of the API, called against the fake server (`testing` feature).
#![cfg(feature = "testing")]

use onebusaway_sdk::OnebusawaySdk;
use onebusaway_sdk::testing::FakeServer;

fn client(server: &FakeServer) -> OnebusawaySdk {
    OnebusawaySdk::builder()
        .api_key("My API Key")
        .base_url(server.base_url())
        .max_retries(0)
        .build()
        .expect("a client")
}

mod agencies_with_coverage {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.agencies_with_coverage().list().await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/agencies-with-coverage.json");
    }
}

mod agency {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.agency().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/agency/string.json");
    }
}

mod vehicles_for_agency {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.vehicles_for_agency().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/vehicles-for-agency/string.json");
    }
}

mod config {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.config().retrieve().await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/config.json");
    }
}

mod current_time {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.current_time().retrieve().await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/current-time.json");
    }
}

mod stops_for_location {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.stops_for_location().list().await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/stops-for-location.json");
    }
}

mod stops_for_route {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.stops_for_route().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/stops-for-route/string.json");
    }
}

mod stops_for_agency {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.stops_for_agency().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/stops-for-agency/string.json");
    }
}

mod stop {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.stop().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/stop/string.json");
    }
}

mod stop_ids_for_agency {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.stop_ids_for_agency().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/stop-ids-for-agency/string.json");
    }
}

mod schedule_for_stop {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.schedule_for_stop().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/schedule-for-stop/string.json");
    }
}

mod route {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.route().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/route/string.json");
    }
}

mod route_ids_for_agency {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.route_ids_for_agency().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/route-ids-for-agency/string.json");
    }
}

mod routes_for_location {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.routes_for_location().list().await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/routes-for-location.json");
    }
}

mod routes_for_agency {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.routes_for_agency().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/routes-for-agency/string.json");
    }
}

mod schedule_for_route {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.schedule_for_route().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/schedule-for-route/string.json");
    }
}

mod arrivals_and_departures_for_location {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client
            .arrivals_and_departures_for_location()
            .list(0.0, 0.0)
            .await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/api/where/arrivals-and-departures-for-location.json"
        );
    }
}

mod arrival_and_departure {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.arrival_and_departure().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/api/where/arrivals-and-departures-for-stop/string.json"
        );
    }

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client
            .arrival_and_departure()
            .retrieve("string", "string", 0)
            .await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/api/where/arrival-and-departure-for-stop/string.json"
        );
    }
}

mod trip {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.trip().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/trip/string.json");
    }
}

mod trips_for_location {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.trips_for_location().list(0.0, 0.0).await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/trips-for-location.json");
    }
}

mod trip_details {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.trip_details().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/trip-details/string.json");
    }
}

mod trip_for_vehicle {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.trip_for_vehicle().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/trip-for-vehicle/string.json");
    }
}

mod trips_for_route {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.trips_for_route().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/trips-for-route/string.json");
    }
}

mod report_problem_with_stop {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.report_problem_with_stop().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/api/where/report-problem-with-stop/string.json"
        );
    }
}

mod report_problem_with_trip {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.report_problem_with_trip().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.path,
            "/api/where/report-problem-with-trip/string.json"
        );
    }
}

mod search_for_stop {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.search_for_stop().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/search/stop.json");
    }
}

mod search_for_route {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.search_for_route().list("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/search/route.json");
    }
}

mod block {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.block().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/block/string.json");
    }
}

mod shape {
    use super::*;

    #[tokio::test]
    async fn retrieve() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.shape().retrieve("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/where/shape/string.json");
    }
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_client() {
    let server = FakeServer::start();
    let client = onebusaway_sdk::blocking::OnebusawaySdk::from_async(client(&server))
        .expect("a blocking client");
    let response = client.agencies_with_coverage().list().send();
    assert!(response.is_ok(), "{response:?}");
    assert_eq!(server.requests().len(), 1);
}
