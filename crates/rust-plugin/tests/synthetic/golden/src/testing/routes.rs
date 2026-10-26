//! Every operation of the API, with the reply the fake server gives it.

use super::{Route, StreamReply};

/// The API's routes.
pub static ROUTES: &[Route] = &[
    Route {
        method: "GET",
        path: "/v1/notes",
        status: 200,
        content_type: "application/json",
        body: r#"{"data":[]}"#,
        streaming: None,
    },
    Route {
        method: "PUT",
        path: "/v1/notes/{note_id}",
        status: 200,
        content_type: "application/json",
        body: r#"{"content":"string","created_at":"2019-12-27T18:11:19.117Z","id":"string","title":"string"}"#,
        streaming: None,
    },
    Route {
        method: "DELETE",
        path: "/v1/notes/{note_id}",
        status: 200,
        content_type: "application/json",
        body: r#""#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/v1/notes/{note_id}/comments",
        status: 200,
        content_type: "application/json",
        body: r#"[]"#,
        streaming: None,
    },
    Route {
        method: "POST",
        path: "/v1/completions",
        status: 200,
        content_type: "application/json",
        body: r#"{"id":"string","text":"string"}"#,
        streaming: Some(StreamReply {
            param: "stream",
            content_type: "text/event-stream",
            body: r#"data: {"id":"string","text":"string"}

data: [DONE]

"#,
        }),
    },
    Route {
        method: "POST",
        path: "/v1/files/{file_id}/content",
        status: 200,
        content_type: "application/json",
        body: r#"{"size":0}"#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/v1/tags",
        status: 200,
        content_type: "application/json",
        body: r#"{"results":[]}"#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/v1/events",
        status: 200,
        content_type: "application/json",
        body: r#"{"entries":[]}"#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/v1/users",
        status: 200,
        content_type: "application/json",
        body: r#"{"objects":[]}"#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/v1/logs",
        status: 200,
        content_type: "application/json",
        body: r#"{"logs":[]}"#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/v1/labels",
        status: 200,
        content_type: "application/json",
        body: r#"{"labels":[]}"#,
        streaming: None,
    },
    Route {
        method: "GET",
        path: "/health",
        status: 200,
        content_type: "application/json",
        body: r#"{"ok":true}"#,
        streaming: None,
    },
];
