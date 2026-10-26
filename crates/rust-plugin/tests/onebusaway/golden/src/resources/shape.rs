//! The `shape` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json, path_param};
use serde::{Deserialize, Serialize};

/// The `shape` resource.
#[derive(Debug, Clone, Copy)]
pub struct ShapeResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ShapeResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// Retrieve a shape by ID
    ///
    /// Retrieve a shape (the path traveled by a transit vehicle) by ID.
    pub fn retrieve(&self, shape_id: impl Into<String>) -> Request<'a, ShapeRetrieveParams, M> {
        let params = ShapeRetrieveParams {
            shape_id: shape_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/shape/{shapeID}.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeRetrieveParams {
    /// The shape ID, encoded directly in the URL
    pub shape_id: String,
}

impl Endpoint for ShapeRetrieveParams {
    type Output = ShapeRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            format!("/api/where/shape/{}.json", path_param(&self.shape_id)),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ShapeRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeRetrieveResponseData {
    pub entry: ShapeRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeRetrieveResponseDataEntry {
    pub length: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<String>,
    /// Encoded polyline format representing the shape of the path
    pub points: String,
}
