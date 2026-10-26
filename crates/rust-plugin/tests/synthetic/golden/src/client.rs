//! The `AcmeNotes` client and its builder.

use crate::rt::{
    ArrayFormat, Async, Config, Core, Endpoint, Error, NestedFormat, Request, RequestSpec, Result,
    decode_json, from_env,
};
use serde::{Deserialize, Serialize};

/// The servers the API runs on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Environment {
    /// `https://api.acme-notes.test`
    #[default]
    Production,
    /// `https://sandbox.acme-notes.test`
    Sandbox,
}

impl Environment {
    /// The base URL of the environment.
    #[must_use]
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Production => "https://api.acme-notes.test",
            Self::Sandbox => "https://sandbox.acme-notes.test",
        }
    }
}

/// The params of `health` (GET `/health`).
#[derive(Debug, Clone, PartialEq)]
pub struct AcmeNotesHealthParams {}

impl Endpoint for AcmeNotesHealthParams {
    type Output = AcmeNotesHealthResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(reqwest::Method::GET, "/health".to_owned()))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcmeNotesHealthResponse {
    pub ok: bool,
}

/// A client for the API. Cloning it is cheap: clones share one connection pool.
#[derive(Debug, Clone)]
pub struct AcmeNotes {
    pub(crate) core: std::sync::Arc<Core>,
}

impl AcmeNotes {
    /// A client with every option read from the environment.
    ///
    /// # Errors
    ///
    /// [`Error::Config`] when a required option is not set.
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// A builder, to set options before the environment is read.
    pub fn builder() -> AcmeNotesBuilder {
        AcmeNotesBuilder::default()
    }

    /// The `notes` resource.
    pub fn notes(&self) -> crate::resources::notes::NotesResource<'_> {
        crate::resources::notes::NotesResource::new(&self.core, Async)
    }

    /// The `completions` resource.
    pub fn completions(&self) -> crate::resources::completions::CompletionsResource<'_> {
        crate::resources::completions::CompletionsResource::new(&self.core, Async)
    }

    /// The `files` resource.
    pub fn files(&self) -> crate::resources::files::FilesResource<'_> {
        crate::resources::files::FilesResource::new(&self.core, Async)
    }

    /// The `tags` resource.
    pub fn tags(&self) -> crate::resources::tags::TagsResource<'_> {
        crate::resources::tags::TagsResource::new(&self.core, Async)
    }

    /// The `events` resource.
    pub fn events(&self) -> crate::resources::events::EventsResource<'_> {
        crate::resources::events::EventsResource::new(&self.core, Async)
    }

    /// The `users` resource.
    pub fn users(&self) -> crate::resources::users::UsersResource<'_> {
        crate::resources::users::UsersResource::new(&self.core, Async)
    }

    /// The `logs` resource.
    pub fn logs(&self) -> crate::resources::logs::LogsResource<'_> {
        crate::resources::logs::LogsResource::new(&self.core, Async)
    }

    /// The `labels` resource.
    pub fn labels(&self) -> crate::resources::labels::LabelsResource<'_> {
        crate::resources::labels::LabelsResource::new(&self.core, Async)
    }

    /// Health check
    pub fn health(&self) -> Request<'_, AcmeNotesHealthParams> {
        let params = AcmeNotesHealthParams {};
        Request::new(&self.core, params, Async)
    }
}

/// Options of a [`AcmeNotes`]; each option not set is read from the environment.
#[derive(Debug, Clone, Default)]
pub struct AcmeNotesBuilder {
    api_key: Option<String>,
    organization: Option<String>,
    base_url: Option<String>,
    environment: Option<Environment>,
    timeout: Option<std::time::Duration>,
    max_retries: Option<u32>,
    headers: Vec<(String, String)>,
    http_client: Option<reqwest::Client>,
}

impl AcmeNotesBuilder {
    /// Sets `api_key`.
    ///
    /// Read from `ACME_NOTES_API_KEY` when not set.
    pub fn api_key(mut self, value: impl Into<String>) -> Self {
        self.api_key = Some(value.into());
        self
    }

    /// Sets `organization`.
    ///
    /// Read from `ACME_NOTES_ORG` when not set.
    pub fn organization(mut self, value: impl Into<String>) -> Self {
        self.organization = Some(value.into());
        self
    }

    /// The URL every request goes to, instead of the environment's.
    ///
    /// Read from `ACME_NOTES_BASE_URL` when not set.
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
    pub fn build(self) -> Result<AcmeNotes> {
        Ok(AcmeNotes {
            core: std::sync::Arc::new(self.core()?),
        })
    }

    /// The blocking client.
    ///
    /// # Errors
    ///
    /// As [`Self::build`], and [`Error::Config`] when its runtime cannot be started.
    #[cfg(feature = "blocking")]
    pub fn build_blocking(self) -> Result<crate::blocking::AcmeNotes> {
        crate::blocking::AcmeNotes::from_async(self.build()?)
    }

    fn core(self) -> Result<Core> {
        let mut headers: Vec<(String, String)> = Vec::new();
        let query: Vec<(String, String)> = Vec::new();
        let api_key = from_env(self.api_key, "ACME_NOTES_API_KEY").ok_or_else(|| {
            Error::Config("missing api_key: call `api_key` or set ACME_NOTES_API_KEY".to_owned())
        })?;
        let organization = from_env(self.organization, "ACME_NOTES_ORG");
        let base_url = match (
            from_env(self.base_url, "ACME_NOTES_BASE_URL"),
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
        headers.push(("authorization".to_owned(), format!("Bearer {api_key}")));
        if let Some(organization) = organization {
            headers.push(("X-Organization".to_owned(), organization));
        }
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
            array_format: ArrayFormat::Comma,
            nested_format: NestedFormat::Brackets,
        };
        Core::new(config, self.http_client)
    }
}
