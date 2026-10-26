//! The `events` resource.

use crate::rt::{Async, Core, Endpoint, Page, Request, RequestSpec, Result, decode_page};
use serde::{Deserialize, Serialize};

/// The `events` resource.
#[derive(Debug, Clone, Copy)]
pub struct EventsResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> EventsResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn list(&self) -> Request<'a, EventsListParams, M> {
        let params = EventsListParams { page: None };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/events`).
#[derive(Debug, Clone, PartialEq)]
pub struct EventsListParams {
    pub page: Option<i64>,
}

impl<M> Request<'_, EventsListParams, M> {
    pub fn page(mut self, page: i64) -> Self {
        self.params_mut().page = Some(page);
        self
    }
}

impl Endpoint for EventsListParams {
    type Output = Page<EventsListResponse>;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::GET, "/v1/events".to_owned());
        spec.query("page", &self.page)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Self::Output> {
        decode_page(&crate::pages::NUMBERED_PAGE, response, spec).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventsListResponse {
    pub kind: String,
}
