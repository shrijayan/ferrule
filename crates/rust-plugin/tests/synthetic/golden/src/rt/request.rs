//! Requests: what a method sends ([`RequestSpec`]), the builder a method returns ([`Request`])
//! and how its response is read ([`Endpoint`]).

use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::error::{Error, Result};
use super::http::Core;
use super::query;

/// The HTTP request one call sends, before the client's settings are applied.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct RequestSpec {
    pub method: reqwest::Method,
    /// From the base URL, starting with `/`, path params encoded.
    pub path: String,
    pub query: Vec<(String, Value)>,
    pub headers: Vec<(String, String)>,
    pub body: Option<RequestBody>,
    pub accept: &'static str,
}

/// A request body and its content type.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct RequestBody {
    pub content_type: &'static str,
    pub bytes: bytes::Bytes,
}

impl RequestSpec {
    /// A request without params, accepting JSON.
    #[must_use]
    pub fn new(method: reqwest::Method, path: String) -> Self {
        Self {
            method,
            path,
            query: Vec::new(),
            headers: Vec::new(),
            body: None,
            accept: "application/json",
        }
    }

    /// Adds query param `name`; `None` (JSON `null`) adds nothing.
    ///
    /// # Errors
    ///
    /// [`Error::Encode`] when the value cannot be serialized.
    pub fn query(&mut self, name: &str, value: &impl Serialize) -> Result<()> {
        let value = serde_json::to_value(value).map_err(Error::Encode)?;
        if !value.is_null() {
            self.query.push((name.to_owned(), value));
        }
        Ok(())
    }

    /// Sets header `name`; `None` sets nothing.
    ///
    /// # Errors
    ///
    /// [`Error::Encode`] when the value cannot be serialized.
    pub fn header(&mut self, name: &str, value: &impl Serialize) -> Result<()> {
        let value = serde_json::to_value(value).map_err(Error::Encode)?;
        let text = match &value {
            Value::Array(items) => Some(
                items
                    .iter()
                    .filter_map(query::scalar)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            other => query::scalar(other),
        };
        if let Some(text) = text {
            self.headers.push((name.to_owned(), text));
        }
        Ok(())
    }

    /// Sends `body` as JSON.
    ///
    /// # Errors
    ///
    /// [`Error::Encode`] when the body cannot be serialized.
    pub fn json(&mut self, body: &impl Serialize) -> Result<()> {
        let bytes = serde_json::to_vec(body).map_err(Error::Encode)?;
        self.body = Some(RequestBody {
            content_type: "application/json",
            bytes: bytes.into(),
        });
        Ok(())
    }

    /// Sets field `name` of the JSON object body; `None` sets nothing unless `required`.
    ///
    /// # Errors
    ///
    /// [`Error::Encode`] when the value cannot be serialized.
    pub fn body_field(&mut self, name: &str, value: &impl Serialize, required: bool) -> Result<()> {
        let value = serde_json::to_value(value).map_err(Error::Encode)?;
        if value.is_null() && !required {
            return Ok(());
        }
        let mut fields: serde_json::Map<String, Value> = self
            .body
            .as_ref()
            .and_then(|b| serde_json::from_slice(&b.bytes).ok())
            .unwrap_or_default();
        fields.insert(name.to_owned(), value);
        self.json(&fields)
    }

    /// Sends `bytes` as they are.
    pub fn bytes(&mut self, content_type: &'static str, bytes: bytes::Bytes) {
        self.body = Some(RequestBody {
            content_type,
            bytes,
        });
    }
}

/// A path param as text, percent-encoded.
///
/// # Errors
///
/// [`Error::Config`] for `.` or `..`: URL parsers resolve those segments even when they are
/// percent-encoded, so the request would go to another path.
#[doc(hidden)]
pub fn path_param(value: &impl Serialize) -> Result<String> {
    let value = serde_json::to_value(value).map_err(Error::Encode)?;
    match query::scalar(&value).as_deref() {
        Some("." | "..") => Err(Error::Config(
            "a path param cannot be `.` or `..`".to_owned(),
        )),
        Some(text) => Ok(query::percent(text)),
        None => Ok(String::new()),
    }
}

/// Settings of one request that override the client's.
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// How long to wait for the response headers.
    pub timeout: Option<Duration>,
    /// How many times to retry a failed request.
    pub max_retries: Option<u32>,
    /// Extra headers, sent after the client's and the method's.
    pub headers: Vec<(String, String)>,
    /// Extra query params.
    pub query: Vec<(String, String)>,
}

/// The mode of an async client: [`Request`]s are futures.
#[derive(Debug, Clone, Copy, Default)]
pub struct Async;

/// The mode of a blocking client: [`Request::send`] blocks on the client's runtime.
#[cfg(feature = "blocking")]
#[derive(Debug, Clone, Copy)]
pub struct Blocking<'a>(pub(crate) &'a tokio::runtime::Runtime);

/// The params of one method: how to send them and how to read the response.
pub trait Endpoint {
    /// What the call returns.
    type Output;

    /// The request to send.
    ///
    /// # Errors
    ///
    /// [`Error::Encode`] when a param cannot be serialized.
    fn spec(&self) -> Result<RequestSpec>;

    /// Reads a 2xx response of `spec`.
    fn decode(
        response: reqwest::Response,
        spec: RequestSpec,
    ) -> impl Future<Output = Result<Self::Output>> + Send;
}

/// One call, built by a resource method: set optional params and options, then send it (or
/// `.await` it, for an async client).
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it is sent"]
pub struct Request<'a, P, M = Async> {
    core: &'a Core,
    params: P,
    options: RequestOptions,
    mode: M,
}

impl<'a, P, M> Request<'a, P, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, params: P, mode: M) -> Self {
        Self {
            core,
            params,
            options: RequestOptions::default(),
            mode,
        }
    }

    /// The params to send.
    pub fn params(&self) -> &P {
        &self.params
    }

    /// The params to send, to change.
    pub fn params_mut(&mut self) -> &mut P {
        &mut self.params
    }

    /// Waits at most `timeout` for the response headers, instead of the client's timeout.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    /// Retries at most `max_retries` times, instead of the client's number.
    pub fn request_max_retries(mut self, max_retries: u32) -> Self {
        self.options.max_retries = Some(max_retries);
        self
    }

    /// Sends one more header.
    pub fn extra_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.headers.push((name.into(), value.into()));
        self
    }

    /// Sends one more query param.
    pub fn extra_query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.query.push((name.into(), value.into()));
        self
    }

    pub(crate) fn parts(self) -> (&'a Core, P, RequestOptions, M) {
        (self.core, self.params, self.options, self.mode)
    }
}

/// Sends `spec` and reads the response as `P`'s output.
pub(crate) async fn call<P: Endpoint>(
    core: &Core,
    spec: RequestSpec,
    options: &RequestOptions,
) -> Result<P::Output> {
    let response = core.execute(&spec, options).await?;
    P::decode(response, spec).await
}

impl<'a, P> Request<'a, P, Async>
where
    P: Endpoint + Send + 'a,
{
    /// Sends the request and reads the response.
    ///
    /// # Errors
    ///
    /// Any [`Error`]: the request could not be sent, the status was not 2xx after the retries,
    /// or the response did not decode.
    pub async fn send(self) -> Result<P::Output> {
        let (core, params, options, _) = self.parts();
        call::<P>(core, params.spec()?, &options).await
    }
}

impl<'a, P> IntoFuture for Request<'a, P, Async>
where
    P: Endpoint + Send + 'a,
    P::Output: Send,
{
    type Output = Result<P::Output>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<P::Output>> + Send + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

#[cfg(feature = "blocking")]
impl<P: Endpoint> Request<'_, P, Blocking<'_>> {
    /// Sends the request and reads the response, blocking until it is done.
    ///
    /// # Errors
    ///
    /// Any [`Error`]: the request could not be sent, the status was not 2xx after the retries,
    /// or the response did not decode.
    pub fn send(self) -> Result<P::Output> {
        let (core, params, options, Blocking(runtime)) = self.parts();
        let spec = params.spec()?;
        runtime.block_on(call::<P>(core, spec, &options))
    }
}

/// Reads a JSON body; an empty body reads as `null`.
///
/// # Errors
///
/// [`Error::Connection`] when the body cannot be read, [`Error::Decode`] when it is not a `T`.
#[doc(hidden)]
pub async fn decode_json<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let bytes = response.bytes().await.map_err(Error::Connection)?;
    let text = if bytes.iter().all(u8::is_ascii_whitespace) {
        &b"null"[..]
    } else {
        &bytes[..]
    };
    serde_json::from_slice(text).map_err(|source| Error::Decode {
        source,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

/// Reads the body as bytes.
///
/// # Errors
///
/// [`Error::Connection`] when the body cannot be read.
#[doc(hidden)]
pub async fn decode_bytes(response: reqwest::Response) -> Result<bytes::Bytes> {
    response.bytes().await.map_err(Error::Connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn skips_absent_params_and_joins_header_lists() {
        let mut spec = RequestSpec::new(reqwest::Method::GET, "/x".into());
        spec.query("a", &None::<i64>).expect("encodes");
        spec.query("b", &Some(2)).expect("encodes");
        spec.header("x-none", &None::<String>).expect("encodes");
        spec.header("x-list", &["a", "b"]).expect("encodes");
        spec.header("x-flag", &true).expect("encodes");
        assert_eq!(spec.query, [("b".to_owned(), json!(2))]);
        assert_eq!(
            spec.headers,
            [
                ("x-list".to_owned(), "a,b".to_owned()),
                ("x-flag".to_owned(), "true".to_owned())
            ]
        );
    }

    #[test]
    fn encodes_path_params() {
        assert_eq!(path_param(&"1_100 a/b").expect("valid"), "1_100%20a%2Fb");
        assert_eq!(path_param(&42).expect("valid"), "42");
        assert_eq!(path_param(&json!("a")).expect("valid"), "a");
        assert_eq!(path_param(&"..a").expect("valid"), "..a");
    }

    #[test]
    fn rejects_dot_segments() {
        for value in [".", ".."] {
            assert!(
                matches!(path_param(&value), Err(Error::Config(_))),
                "{value}"
            );
        }
    }

    #[test]
    fn builds_object_bodies_field_by_field() {
        let mut spec = RequestSpec::new(reqwest::Method::POST, "/x".into());
        spec.json(&serde_json::Map::new()).expect("encodes");
        spec.body_field("a", &1, true).expect("encodes");
        spec.body_field("b", &None::<i64>, false).expect("encodes");
        spec.body_field("c", &None::<i64>, true).expect("encodes");
        let body = spec.body.expect("a body");
        assert_eq!(body.content_type, "application/json");
        assert_eq!(&body.bytes[..], br#"{"a":1,"c":null}"#);
    }
}
