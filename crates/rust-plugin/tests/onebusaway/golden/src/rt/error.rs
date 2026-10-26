//! The errors every call can return.

use reqwest::header::HeaderMap;
use serde_json::Value;

/// A `Result` whose error is [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A response with a status that is not 2xx.
#[derive(Debug, Clone)]
pub struct ApiError {
    /// The HTTP status code.
    pub status: u16,
    /// The response headers.
    pub headers: HeaderMap,
    /// The response body as JSON, or `None` when it is empty or not JSON.
    pub body: Option<Value>,
    /// The `message` (or `error.message`) of the body, else the body as text.
    pub message: String,
}

impl ApiError {
    pub(crate) fn new(status: u16, headers: HeaderMap, text: &str) -> Self {
        let body: Option<Value> = serde_json::from_str(text).ok();
        let message = body
            .as_ref()
            .and_then(message_of)
            .map_or_else(|| text.trim().to_owned(), str::to_owned);
        Self {
            status,
            headers,
            body,
            message,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "status {}", self.status)
        } else {
            write!(f, "status {}: {}", self.status, self.message)
        }
    }
}

/// The `message` of an error body: `{"message": ...}` or `{"error": {"message": ...}}`.
fn message_of(body: &Value) -> Option<&str> {
    body.get("message")
        .or_else(|| body.get("error").and_then(|e| e.get("message")))
        .and_then(Value::as_str)
}

/// Everything a call can fail with. The status variants match the HTTP status of the response
/// and hold it boxed, so that a `Result` stays small.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The request could not be sent or the response could not be read.
    #[error("connection error: {0}")]
    Connection(#[source] reqwest::Error),
    /// No response came within the timeout.
    #[error("request timed out")]
    Timeout,
    /// 400.
    #[error("bad request ({0})")]
    BadRequest(Box<ApiError>),
    /// 401.
    #[error("authentication failed ({0})")]
    Authentication(Box<ApiError>),
    /// 403.
    #[error("permission denied ({0})")]
    PermissionDenied(Box<ApiError>),
    /// 404.
    #[error("not found ({0})")]
    NotFound(Box<ApiError>),
    /// 409.
    #[error("conflict ({0})")]
    Conflict(Box<ApiError>),
    /// 422.
    #[error("unprocessable entity ({0})")]
    UnprocessableEntity(Box<ApiError>),
    /// 429.
    #[error("rate limited ({0})")]
    RateLimit(Box<ApiError>),
    /// 500 and above.
    #[error("internal server error ({0})")]
    InternalServer(Box<ApiError>),
    /// Any other status that is not 2xx.
    #[error("unexpected status ({0})")]
    Status(Box<ApiError>),
    /// The response body did not have the type the API describes.
    #[error("could not decode the response: {source}")]
    Decode {
        /// Why decoding failed.
        #[source]
        source: serde_json::Error,
        /// The body that failed to decode.
        body: String,
    },
    /// A request parameter could not be encoded.
    #[error("could not encode the request: {0}")]
    Encode(#[source] serde_json::Error),
    /// A stream sent an `error` event.
    #[error("the stream sent an error: {0}")]
    Stream(Value),
    /// The client is not configured correctly, such as a missing API key.
    #[error("{0}")]
    Config(String),
}

impl Error {
    /// The error for a response with a status that is not 2xx.
    pub(crate) fn from_status(err: ApiError) -> Self {
        let status = err.status;
        let err = Box::new(err);
        match status {
            400 => Self::BadRequest(err),
            401 => Self::Authentication(err),
            403 => Self::PermissionDenied(err),
            404 => Self::NotFound(err),
            409 => Self::Conflict(err),
            422 => Self::UnprocessableEntity(err),
            429 => Self::RateLimit(err),
            500.. => Self::InternalServer(err),
            _ => Self::Status(err),
        }
    }

    /// The response of a status error, else `None`.
    #[must_use]
    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            Self::BadRequest(e)
            | Self::Authentication(e)
            | Self::PermissionDenied(e)
            | Self::NotFound(e)
            | Self::Conflict(e)
            | Self::UnprocessableEntity(e)
            | Self::RateLimit(e)
            | Self::InternalServer(e)
            | Self::Status(e) => Some(e),
            _ => None,
        }
    }

    /// The HTTP status of a status error, else `None`.
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        self.api_error().map(|e| e.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_statuses_to_variants() {
        let err = |status| Error::from_status(ApiError::new(status, HeaderMap::new(), ""));
        assert!(matches!(err(400), Error::BadRequest(_)));
        assert!(matches!(err(401), Error::Authentication(_)));
        assert!(matches!(err(403), Error::PermissionDenied(_)));
        assert!(matches!(err(404), Error::NotFound(_)));
        assert!(matches!(err(409), Error::Conflict(_)));
        assert!(matches!(err(422), Error::UnprocessableEntity(_)));
        assert!(matches!(err(429), Error::RateLimit(_)));
        assert!(matches!(err(503), Error::InternalServer(_)));
        assert!(matches!(err(418), Error::Status(_)));
        assert_eq!(err(418).status(), Some(418));
    }

    #[test]
    fn reads_the_message_of_the_body() {
        let nested = ApiError::new(400, HeaderMap::new(), r#"{"error": {"message": "bad id"}}"#);
        assert_eq!(nested.message, "bad id");
        let flat = ApiError::new(400, HeaderMap::new(), r#"{"message": "no"}"#);
        assert_eq!(flat.to_string(), "status 400: no");
        let text = ApiError::new(502, HeaderMap::new(), "upstream down\n");
        assert_eq!((text.body, text.message.as_str()), (None, "upstream down"));
    }
}
