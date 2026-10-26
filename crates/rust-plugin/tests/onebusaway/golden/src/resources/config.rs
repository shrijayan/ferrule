//! The `config` resource.

use crate::rt::{Async, Core, Endpoint, Request, RequestSpec, Result, decode_json};
use serde::{Deserialize, Serialize};

/// The `config` resource.
#[derive(Debug, Clone, Copy)]
pub struct ConfigResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> ConfigResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// config
    pub fn retrieve(&self) -> Request<'a, ConfigRetrieveParams, M> {
        let params = ConfigRetrieveParams {};
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `retrieve` (GET `/api/where/config.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigRetrieveParams {}

impl Endpoint for ConfigRetrieveParams {
    type Output = ConfigRetrieveResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::GET,
            "/api/where/config.json".to_owned(),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigRetrieveResponse {
    #[serde(flatten)]
    pub response_wrapper: crate::shared::ResponseWrapper,
    pub data: ConfigRetrieveResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigRetrieveResponseData {
    pub entry: ConfigRetrieveResponseDataEntry,
    pub references: crate::shared::References,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigRetrieveResponseDataEntry {
    #[serde(rename = "gitProperties")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_properties: Option<ConfigRetrieveResponseDataEntryGitProperties>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "serviceDateFrom")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_date_from: Option<String>,
    #[serde(rename = "serviceDateTo")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_date_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigRetrieveResponseDataEntryGitProperties {
    #[serde(rename = "git.branch")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(rename = "git.build.host")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_build_host: Option<String>,
    #[serde(rename = "git.build.time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_build_time: Option<String>,
    #[serde(rename = "git.build.user.email")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_build_user_email: Option<String>,
    #[serde(rename = "git.build.user.name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_build_user_name: Option<String>,
    #[serde(rename = "git.build.version")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_build_version: Option<String>,
    #[serde(rename = "git.closest.tag.commit.count")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_closest_tag_commit_count: Option<String>,
    #[serde(rename = "git.closest.tag.name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_closest_tag_name: Option<String>,
    #[serde(rename = "git.commit.id")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_id: Option<String>,
    #[serde(rename = "git.commit.id.abbrev")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_id_abbrev: Option<String>,
    #[serde(rename = "git.commit.id.describe")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_id_describe: Option<String>,
    #[serde(rename = "git.commit.id.describe-short")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_id_describe_short: Option<String>,
    #[serde(rename = "git.commit.message.full")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_message_full: Option<String>,
    #[serde(rename = "git.commit.message.short")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_message_short: Option<String>,
    #[serde(rename = "git.commit.time")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_time: Option<String>,
    #[serde(rename = "git.commit.user.email")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_user_email: Option<String>,
    #[serde(rename = "git.commit.user.name")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit_user_name: Option<String>,
    #[serde(rename = "git.dirty")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_dirty: Option<String>,
    #[serde(rename = "git.remote.origin.url")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_remote_origin_url: Option<String>,
    #[serde(rename = "git.tags")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_tags: Option<String>,
}
