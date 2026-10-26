//! Sending requests: timeouts, retries and status errors.

use std::hash::{BuildHasher, RandomState};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::error::{ApiError, Error, Result};
use super::query::{self, ArrayFormat, NestedFormat};
use super::request::{RequestOptions, RequestSpec};

/// The first retry waits this long, before jitter; each next one twice as long.
pub const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);

/// No retry waits longer than this, unless the server asks for it.
pub const MAX_RETRY_DELAY: Duration = Duration::from_secs(8);

/// A `retry-after` longer than this is ignored.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// The share of a retry delay that jitter can take off.
const MAX_JITTER: f64 = 0.25;

/// The settings of one client, filled by its builder.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct Config {
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub initial_retry_delay: Duration,
    pub max_retry_delay: Duration,
    pub headers: Vec<(String, String)>,
    pub query: Vec<(String, String)>,
    pub idempotency_header: Option<String>,
    pub user_agent: String,
    pub array_format: ArrayFormat,
    pub nested_format: NestedFormat,
}

/// What every resource of a client shares: the HTTP client and the settings.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct Core {
    http: reqwest::Client,
    config: Config,
}

impl Core {
    /// A core over `http`, else a new `reqwest` client.
    ///
    /// # Errors
    ///
    /// [`Error::Connection`] when the HTTP client cannot be built.
    pub fn new(config: Config, http: Option<reqwest::Client>) -> Result<Self> {
        let http = match http {
            Some(http) => http,
            None => reqwest::Client::builder()
                .read_timeout(config.timeout)
                .build()
                .map_err(Error::Connection)?,
        };
        Ok(Self { http, config })
    }

    /// The client's settings.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Sends `spec`, retrying as the settings allow; returns a response with a 2xx status.
    ///
    /// # Errors
    ///
    /// The status error of the last response, [`Error::Timeout`] or [`Error::Connection`].
    pub async fn execute(
        &self,
        spec: &RequestSpec,
        options: &RequestOptions,
    ) -> Result<reqwest::Response> {
        let max_retries = options.max_retries.unwrap_or(self.config.max_retries);
        let timeout = options.timeout.unwrap_or(self.config.timeout);
        let idempotency_key = self.idempotency_key(spec);
        let mut attempt = 0;
        loop {
            let request = self.build(spec, options, idempotency_key.as_deref())?;
            let outcome = match tokio::time::timeout(timeout, self.http.execute(request)).await {
                Err(_) => Err(Error::Timeout),
                Ok(Err(err)) if err.is_timeout() => Err(Error::Timeout),
                Ok(Err(err)) => Err(Error::Connection(err)),
                Ok(Ok(response)) => Ok(response),
            };
            let retry = attempt < max_retries
                && match &outcome {
                    Ok(response) => should_retry(response.status().as_u16(), response.headers()),
                    Err(_) => true,
                };
            if !retry {
                return match outcome {
                    Ok(response) if response.status().is_success() => Ok(response),
                    Ok(response) => Err(status_error(response).await),
                    Err(err) => Err(err),
                };
            }
            let headers = outcome.as_ref().ok().map(reqwest::Response::headers);
            let delay = retry_delay(&self.config, attempt, headers, jitter());
            drop(outcome);
            tokio::time::sleep(delay).await;
            attempt += 1;
        }
    }

    /// A fresh idempotency key for a request that is not a `GET`, when the API names a header.
    fn idempotency_key(&self, spec: &RequestSpec) -> Option<String> {
        let header = self.config.idempotency_header.as_ref()?;
        let has_one = spec
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case(header));
        (spec.method != reqwest::Method::GET && !has_one)
            .then(|| format!("rust-retry-{:016x}{:016x}", random(), random()))
    }

    fn build(
        &self,
        spec: &RequestSpec,
        options: &RequestOptions,
        idempotency_key: Option<&str>,
    ) -> Result<reqwest::Request> {
        let mut params: Vec<(String, serde_json::Value)> = self
            .config
            .query
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();
        params.extend(spec.query.iter().cloned());
        params.extend(
            options
                .query
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))),
        );
        let query = query::encode(&params, self.config.array_format, self.config.nested_format);
        let base = self.config.base_url.trim_end_matches('/');
        // A `cursor_url` page's next URL is absolute; any other path is under the base URL.
        let absolute = ["http://", "https://"].iter().any(|s| {
            spec.path
                .get(..s.len())
                .is_some_and(|p| p.eq_ignore_ascii_case(s))
        });
        let mut url = if absolute {
            spec.path.clone()
        } else {
            format!("{base}{}", spec.path)
        };
        if !query.is_empty() {
            url.push(if url.contains('?') { '&' } else { '?' });
            url.push_str(&query);
        }
        // The query is left out of the message: it may hold an API key.
        let url = reqwest::Url::parse(&url).map_err(|err| {
            let shown = url.split('?').next().unwrap_or_default();
            Error::Config(format!("invalid URL {shown}: {err}"))
        })?;
        // The client's credentials go only to the base URL's origin, whatever the path holds.
        let expected =
            origin(base).ok_or_else(|| Error::Config(format!("invalid base URL {base}")))?;
        if origin(url.as_str()).as_ref() != Some(&expected) {
            return Err(Error::Config(format!(
                "refusing to send a request to {}, which is not on the base URL's origin",
                origin(url.as_str()).unwrap_or_default()
            )));
        }
        let mut request = reqwest::Request::new(spec.method.clone(), url);
        let headers = request.headers_mut();
        insert(headers, "user-agent", &self.config.user_agent)?;
        insert(headers, "accept", spec.accept)?;
        for (k, v) in self
            .config
            .headers
            .iter()
            .chain(&spec.headers)
            .chain(&options.headers)
        {
            insert(headers, k, v)?;
        }
        if let (Some(header), Some(key)) = (&self.config.idempotency_header, idempotency_key) {
            insert(headers, header, key)?;
        }
        if let Some(body) = &spec.body {
            insert(headers, "content-type", body.content_type)?;
            *request.body_mut() = Some(reqwest::Body::from(body.bytes.clone()));
        }
        Ok(request)
    }
}

/// The scheme, host and port of `url`.
fn origin(url: &str) -> Option<String> {
    let url = reqwest::Url::parse(url).ok()?;
    Some(format!(
        "{}://{}:{}",
        url.scheme(),
        url.host_str()?,
        url.port_or_known_default()?
    ))
}

/// Sets header `name`, replacing any value it had.
fn insert(headers: &mut HeaderMap, name: &str, value: &str) -> Result<()> {
    let name = HeaderName::try_from(name)
        .map_err(|err| Error::Config(format!("invalid header name {name}: {err}")))?;
    let value = HeaderValue::try_from(value)
        .map_err(|err| Error::Config(format!("invalid value of header {name}: {err}")))?;
    headers.insert(name, value);
    Ok(())
}

/// The error of a response with a status that is not 2xx, its body read.
async fn status_error(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let text = response.text().await.unwrap_or_default();
    Error::from_status(ApiError::new(status, headers, &text))
}

/// Whether a response with `status` is worth retrying: the server's `x-should-retry` when it
/// sends one, else 408, 409, 429 and 5xx.
#[must_use]
pub fn should_retry(status: u16, headers: &HeaderMap) -> bool {
    match headers.get("x-should-retry").and_then(|v| v.to_str().ok()) {
        Some("true") => true,
        Some("false") => false,
        _ => matches!(status, 408 | 409 | 429 | 500..),
    }
}

/// How long to wait before retry `attempt` (0 for the first): the server's `retry-after-ms` or
/// `retry-after` (seconds) when it is in (0, 60] seconds, else the backoff with `jitter` (0 to 1)
/// of up to a quarter taken off.
#[must_use]
pub fn retry_delay(
    config: &Config,
    attempt: u32,
    headers: Option<&HeaderMap>,
    jitter: f64,
) -> Duration {
    if let Some(server) = headers.and_then(retry_after)
        && server > Duration::ZERO
        && server <= MAX_RETRY_AFTER
    {
        return server;
    }
    let backoff = config
        .initial_retry_delay
        .saturating_mul(2_u32.saturating_pow(attempt.min(31)))
        .min(config.max_retry_delay);
    backoff.mul_f64(1.0 - jitter.clamp(0.0, 1.0) * MAX_JITTER)
}

/// The delay the server asks for: `retry-after-ms`, else `retry-after` in seconds.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let number = |name: &str| headers.get(name)?.to_str().ok()?.trim().parse::<f64>().ok();
    let seconds = number("retry-after-ms")
        .map(|ms| ms / 1000.0)
        .or_else(|| number("retry-after"))?;
    Duration::try_from_secs_f64(seconds).ok()
}

/// A random fraction in [0, 1), for jitter.
fn jitter() -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "53 random bits")]
    let fraction = (random() >> 11) as f64 / (1_u64 << 53) as f64;
    fraction
}

/// Random bits from the standard library's randomly seeded hasher.
fn random() -> u64 {
    RandomState::new().hash_one(std::time::SystemTime::now())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::rt::{Async, Endpoint, Request, decode_json};
    use crate::testing::{FakeServer, Reply};
    use serde_json::{Value, json};

    pub(crate) fn config() -> Config {
        Config {
            base_url: "http://localhost".into(),
            timeout: Duration::from_secs(60),
            max_retries: 2,
            initial_retry_delay: INITIAL_RETRY_DELAY,
            max_retry_delay: MAX_RETRY_DELAY,
            headers: Vec::new(),
            query: Vec::new(),
            idempotency_header: None,
            user_agent: "test".into(),
            array_format: ArrayFormat::Repeat,
            nested_format: NestedFormat::Brackets,
        }
    }

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))
            .collect()
    }

    #[test]
    fn retries_the_statuses_the_python_runtime_retries() {
        let none = HeaderMap::new();
        for status in [408, 409, 429, 500, 503] {
            assert!(should_retry(status, &none), "{status}");
        }
        for status in [400, 401, 404, 422] {
            assert!(!should_retry(status, &none), "{status}");
        }
        assert!(should_retry(400, &headers(&[("x-should-retry", "true")])));
        assert!(!should_retry(503, &headers(&[("x-should-retry", "false")])));
    }

    #[test]
    fn backs_off_exponentially_up_to_the_maximum() {
        let c = config();
        let delays: Vec<f64> = (0..6)
            .map(|n| retry_delay(&c, n, None, 0.0).as_secs_f64())
            .collect();
        assert_eq!(delays, [0.5, 1.0, 2.0, 4.0, 8.0, 8.0]);
        assert!((retry_delay(&c, 0, None, 1.0).as_secs_f64() - 0.375).abs() < 1e-9);
    }

    #[test]
    fn honours_retry_after_up_to_a_minute() {
        let c = config();
        let ms = headers(&[("retry-after-ms", "1500")]);
        assert_eq!(
            retry_delay(&c, 3, Some(&ms), 0.0),
            Duration::from_millis(1500)
        );
        let secs = headers(&[("retry-after", "2")]);
        assert_eq!(retry_delay(&c, 3, Some(&secs), 0.0), Duration::from_secs(2));
        let huge = headers(&[("retry-after", "1e300")]);
        assert_eq!(
            retry_delay(&c, 0, Some(&huge), 0.0),
            Duration::from_millis(500)
        );
        let negative = headers(&[("retry-after-ms", "-5")]);
        assert_eq!(
            retry_delay(&c, 0, Some(&negative), 0.0),
            Duration::from_millis(500)
        );
        let long = headers(&[("retry-after", "120")]);
        assert_eq!(
            retry_delay(&c, 0, Some(&long), 0.0),
            Duration::from_millis(500)
        );
    }

    /// A core for `server` whose retries wait a millisecond.
    pub(crate) fn core(server: &FakeServer, max_retries: u32) -> Core {
        let config = Config {
            base_url: server.base_url(),
            max_retries,
            initial_retry_delay: Duration::from_millis(1),
            idempotency_header: Some("idempotency-key".into()),
            ..config()
        };
        Core::new(config, None).expect("a core")
    }

    struct Create;

    impl Endpoint for Create {
        type Output = Value;

        fn spec(&self) -> Result<RequestSpec> {
            let mut spec = RequestSpec::new(reqwest::Method::POST, "/things".into());
            spec.json(&json!({"name": "x"}))?;
            Ok(spec)
        }

        async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Value> {
            decode_json(response).await
        }
    }

    #[tokio::test]
    async fn retries_until_a_success_with_one_idempotency_key() {
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply::json(503, &json!({"message": "busy"})));
        server.enqueue(Reply::json(429, &json!({})).header("retry-after-ms", "1"));
        server.enqueue(Reply::json(200, &json!({"id": 1})));
        let core = core(&server, 2);
        let created = Request::new(&core, Create, Async)
            .await
            .expect("the third try succeeds");
        assert_eq!(created, json!({"id": 1}));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        let keys: Vec<Option<&str>> = requests
            .iter()
            .map(|r| r.header("idempotency-key"))
            .collect();
        assert!(
            keys[0].is_some() && keys.iter().all(|k| *k == keys[0]),
            "{keys:?}"
        );
        assert_eq!(requests[0].body, br#"{"name":"x"}"#);
        assert_eq!(requests[0].header("user-agent"), Some("test"));
    }

    #[tokio::test]
    async fn gives_up_after_the_last_retry() {
        let server = FakeServer::with_routes(&[]);
        for _ in 0..3 {
            server.enqueue(Reply::json(500, &json!({"error": {"message": "down"}})));
        }
        let core = core(&server, 2);
        let err = Request::new(&core, Create, Async)
            .request_max_retries(2)
            .await
            .expect_err("500 each time");
        assert!(
            matches!(&err, Error::InternalServer(e) if e.message == "down"),
            "{err:?}"
        );
        assert_eq!(server.requests().len(), 3);
    }

    #[tokio::test]
    async fn does_not_retry_a_client_error() {
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply::json(404, &json!({"message": "no thing"})));
        let core = core(&server, 2);
        let err = Request::new(&core, Create, Async).await.expect_err("404");
        assert_eq!(err.status(), Some(404));
        assert!(matches!(err, Error::NotFound(_)));
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn reports_a_body_of_the_wrong_type() {
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply::json(200, &json!("text")));
        let core = core(&server, 0);
        let spec = RequestSpec::new(reqwest::Method::GET, "/n".into());
        let response = core
            .execute(&spec, &RequestOptions::default())
            .await
            .expect("200");
        let err = decode_json::<i64>(response)
            .await
            .expect_err("not a number");
        assert!(
            matches!(err, Error::Decode { ref body, .. } if body == "\"text\""),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn sends_the_settings_and_the_request_options() {
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply::json(200, &json!({})));
        let mut config = config();
        config.base_url = server.base_url();
        config.headers.push(("x-client".into(), "1".into()));
        config.query.push(("key".into(), "k".into()));
        let core = Core::new(config, None).expect("a core");
        let request = Request::new(&core, Create, Async)
            .extra_header("x-call", "2")
            .extra_query("q", "a b");
        request.await.expect("200");
        let sent = &server.requests()[0];
        assert_eq!(
            (sent.header("x-client"), sent.header("x-call")),
            (Some("1"), Some("2"))
        );
        assert_eq!(sent.query, "key=k&q=a%20b");
    }

    #[tokio::test]
    async fn sends_nothing_to_another_origin() {
        let server = FakeServer::with_routes(&[]);
        let core = core(&server, 0);
        let mut spec = RequestSpec::new(reqwest::Method::GET, "https://elsewhere.test/page".into());
        spec.query("cursor", &"c2").expect("encodes");
        let err = core
            .execute(&spec, &RequestOptions::default())
            .await
            .expect_err("refused");
        assert!(
            matches!(&err, Error::Config(m) if m.contains("https://elsewhere.test:443")),
            "{err:?}"
        );
        assert!(server.requests().is_empty());
        let upper = RequestSpec::new(reqwest::Method::GET, "HTTPS://elsewhere.test/page".into());
        assert!(
            core.execute(&upper, &RequestOptions::default())
                .await
                .is_err()
        );
        let userinfo = RequestSpec::new(reqwest::Method::GET, "@elsewhere.test/page".into());
        let err = core
            .execute(&userinfo, &RequestOptions::default())
            .await
            .expect_err("refused");
        assert!(
            matches!(&err, Error::Config(m) if m.contains("elsewhere.test")),
            "{err:?}"
        );
        assert!(server.requests().is_empty());
        let same = format!("{}/page?cursor=c2", server.base_url());
        server.enqueue(Reply::json(200, &json!({})));
        let spec = RequestSpec::new(reqwest::Method::GET, same);
        core.execute(&spec, &RequestOptions::default())
            .await
            .expect("same origin");
    }

    #[test]
    fn keeps_the_query_out_of_url_errors() {
        let mut config = config();
        config.base_url = "http://bad host".into();
        config.query.push(("key".into(), "secret".into()));
        let core = Core::new(config, None).expect("a core");
        let spec = RequestSpec::new(reqwest::Method::GET, "/x".into());
        let err = core
            .build(&spec, &RequestOptions::default(), None)
            .expect_err("invalid");
        assert!(!err.to_string().contains("secret"), "{err}");
    }

    #[test]
    fn jitter_is_a_fraction() {
        assert!((0..100).map(|_| jitter()).all(|j| (0.0..1.0).contains(&j)));
    }
}
