//! The `agency` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `agency` resource.
#[derive(Debug, Clone, Copy)]
pub struct AgencyResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> AgencyResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Retrieve info for a specific transit agency identified by ID
    ///
    /// Retrieve information for a specific transit agency identified by its unique ID.
    pub fn retrieve(&self, agency_id: impl Into<String>) -> Request<'a, AgencyRetrieveParams, M> {
        let params = AgencyRetrieveParams {
            agency_id: agency_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/agency/{agencyID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct AgencyRetrieveParams {
    /// The ID of the transit agency.
    pub agency_id: String,
}

impl Endpoint for AgencyRetrieveParams {
    type Output = AgencyRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/api/where/agency/{}.json", path_param(&self.agency_id)),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgencyRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: AgencyRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgencyRetrieveResponseData {
    pub entry: AgencyRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgencyRetrieveResponseDataEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclaimer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "fareUrl")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fare_url: Option<String>,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(rename = "privateService")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_service: Option<bool>,
    pub timezone: String,
    pub url: String,
}
