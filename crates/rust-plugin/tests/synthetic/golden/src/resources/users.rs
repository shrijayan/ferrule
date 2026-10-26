//! The `users` resource.

use crate::rt::{Async, Core, Endpoint, Page, Request, RequestSpec, Result, decode_page};
use serde::{Deserialize, Serialize};

/// The `users` resource.
#[derive(Debug, Clone, Copy)]
pub struct UsersResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> UsersResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    pub fn list(&self) -> Request<'a, UsersListParams, M> {
        let params = UsersListParams {
            starting_after: None,
            ending_before: None,
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/users`).
#[derive(Debug, Clone, PartialEq)]
pub struct UsersListParams {
    pub starting_after: Option<String>,
    pub ending_before: Option<String>,
}

impl<M> Request<'_, UsersListParams, M> {
    pub fn starting_after(mut self, starting_after: impl Into<String>) -> Self {
        self.params_mut().starting_after = Some(starting_after.into());
        self
    }

    pub fn ending_before(mut self, ending_before: impl Into<String>) -> Self {
        self.params_mut().ending_before = Some(ending_before.into());
        self
    }
}

impl Endpoint for UsersListParams {
    type Output = Page<UsersListResponse>;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::GET, "/v1/users".to_owned());
        spec.query("starting_after", &self.starting_after)?;
        spec.query("ending_before", &self.ending_before)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Self::Output> {
        decode_page(&crate::pages::ID_CURSOR, response, spec).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsersListResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub id: String,
}
