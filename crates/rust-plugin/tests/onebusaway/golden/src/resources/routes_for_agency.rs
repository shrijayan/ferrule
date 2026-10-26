//! The `routes_for_agency` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `routes_for_agency` resource.
#[derive(Debug, Clone, Copy)]
pub struct RoutesForAgencyResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> RoutesForAgencyResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Retrieve the list of all routes for a particular agency by id
    pub fn list(&self, agency_id: impl Into<String>) -> Request<'a, RoutesForAgencyListParams, M> {
        let params = RoutesForAgencyListParams {
            agency_id: agency_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/api/where/routes-for-agency/{agencyID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct RoutesForAgencyListParams {
    /// The id of the agency
    pub agency_id: String,
}

impl Endpoint for RoutesForAgencyListParams {
    type Output = RoutesForAgencyListResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!(
                "/api/where/routes-for-agency/{}.json",
                path_param(&self.agency_id)
            ),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutesForAgencyListResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: RoutesForAgencyListResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutesForAgencyListResponseData {
    #[serde(rename = "limitExceeded")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_exceeded: Option<bool>,
    pub list: Vec<RoutesForAgencyListResponseDataList>,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutesForAgencyListResponseDataList {
    #[serde(rename = "agencyId")]
    pub agency_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub id: String,
    #[serde(rename = "longName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
    #[serde(rename = "nullSafeShortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub null_safe_short_name: Option<String>,
    #[serde(rename = "shortName")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(rename = "textColor")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    pub r#type: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
