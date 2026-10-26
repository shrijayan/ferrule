//! The `labels` resource.

use crate::rt::{Async, Core, Endpoint, Page, Request, RequestSpec, Result, decode_page};

/// The `labels` resource.
#[derive(Debug, Clone, Copy)]
pub struct LabelsResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> LabelsResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn list(&self) -> Request<'a, LabelsListParams, M> {
        let params = LabelsListParams {};
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/labels`).
#[derive(Debug, Clone, PartialEq)]
pub struct LabelsListParams {}

impl Endpoint for LabelsListParams {
    type Output = Page<String>;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            "/v1/labels".to_owned(),
        ))
    }

    async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Self::Output> {
        decode_page(&crate::pages::SINGLE_PAGE, response, spec).await
    }
}
