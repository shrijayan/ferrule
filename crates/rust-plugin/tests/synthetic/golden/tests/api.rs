//! Every method of the API, called against the fake server (`testing` feature).
#![cfg(feature = "testing")]

use acme_notes::AcmeNotes;
use acme_notes::testing::FakeServer;
use futures_util::StreamExt;

fn client(server: &FakeServer) -> AcmeNotes {
    AcmeNotes::builder()
        .api_key("My API Key")
        .base_url(server.base_url())
        .max_retries(0)
        .build()
        .expect("a client")
}

#[tokio::test]
async fn health() {
    let server = FakeServer::start();
    let client = client(&server);
    let response = client.health().await;
    assert!(response.is_ok(), "{response:?}");
    let request = &server.requests()[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/health");
}

mod notes {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let page = client.notes().list().await;
        assert!(page.is_ok(), "{page:?}");
        let items: Vec<_> = client.notes().list().paginate().collect().await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/notes");
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn update() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client
            .notes()
            .update(
                "string",
                acme_notes::shared::NoteContent::String("string".to_owned()),
                "2019-12-27T18:11:19.117Z",
                "string",
                "string",
            )
            .await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "PUT");
        assert_eq!(request.path, "/v1/notes/string");
    }

    #[tokio::test]
    async fn delete() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.notes().delete("string").await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "DELETE");
        assert_eq!(request.path, "/v1/notes/string");
    }

    mod comments {
        use super::*;

        #[tokio::test]
        async fn list() {
            let server = FakeServer::start();
            let client = client(&server);
            let response = client.notes().comments().list("string").await;
            assert!(response.is_ok(), "{response:?}");
            let request = &server.requests()[0];
            assert_eq!(request.method, "GET");
            assert_eq!(request.path, "/v1/notes/string/comments");
        }
    }
}

mod completions {
    use super::*;

    #[tokio::test]
    async fn create() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client
            .completions()
            .create(
                acme_notes::resources::completions::CompletionsCreateModel::Small,
                "string",
            )
            .await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/completions");
    }

    #[tokio::test]
    async fn create_streaming() {
        let server = FakeServer::start();
        let client = client(&server);
        let events = client
            .completions()
            .create_streaming(
                acme_notes::resources::completions::CompletionsCreateModel::Small,
                "string",
            )
            .stream()
            .await;
        let events: Vec<_> = events.expect("a stream").collect().await;
        assert!(events.iter().all(Result::is_ok), "{events:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/completions");
    }
}

mod files {
    use super::*;

    #[tokio::test]
    async fn upload() {
        let server = FakeServer::start();
        let client = client(&server);
        let response = client.files().upload(0, bytes::Bytes::new()).await;
        assert!(response.is_ok(), "{response:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/files/0/content");
    }
}

mod tags {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let page = client.tags().list().await;
        assert!(page.is_ok(), "{page:?}");
        let items: Vec<_> = client.tags().list().paginate().collect().await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/tags");
    }
}

mod events {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let page = client.events().list().await;
        assert!(page.is_ok(), "{page:?}");
        let items: Vec<_> = client.events().list().paginate().collect().await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/events");
    }
}

mod users {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let page = client.users().list().await;
        assert!(page.is_ok(), "{page:?}");
        let items: Vec<_> = client.users().list().paginate().collect().await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/users");
    }
}

mod logs {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let page = client.logs().list().await;
        assert!(page.is_ok(), "{page:?}");
        let items: Vec<_> = client.logs().list().paginate().collect().await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/logs");
    }
}

mod labels {
    use super::*;

    #[tokio::test]
    async fn list() {
        let server = FakeServer::start();
        let client = client(&server);
        let page = client.labels().list().await;
        assert!(page.is_ok(), "{page:?}");
        let items: Vec<_> = client.labels().list().paginate().collect().await;
        assert!(items.iter().all(Result::is_ok), "{items:?}");
        let request = &server.requests()[0];
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/v1/labels");
    }
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_client() {
    let server = FakeServer::start();
    let client =
        acme_notes::blocking::AcmeNotes::from_async(client(&server)).expect("a blocking client");
    let response = client.health().send();
    assert!(response.is_ok(), "{response:?}");
    let items: Vec<_> = client.notes().list().paginate().collect();
    assert!(items.iter().all(Result::is_ok), "{items:?}");
    let events: Vec<_> = client
        .completions()
        .create_streaming(
            acme_notes::resources::completions::CompletionsCreateModel::Small,
            "string",
        )
        .stream()
        .expect("a stream")
        .collect();
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert_eq!(server.requests().len(), 3);
}
