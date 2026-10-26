//! The `notes` resource.

/// The `comments` resource.
pub mod comments;

use crate::rt::{
    Async, Core, Endpoint, Page, Request, RequestSpec, Result, decode_bytes, decode_json,
    decode_page, path_param,
};
use serde::{Deserialize, Serialize};

/// The `notes` resource.
#[derive(Debug, Clone, Copy)]
pub struct NotesResource<'a, M = Async> {
    core: &'a Core,
    mode: M,
}

impl<'a, M: Copy> NotesResource<'a, M> {
    #[doc(hidden)]
    pub fn new(core: &'a Core, mode: M) -> Self {
        Self { core, mode }
    }

    /// The `comments` resource.
    pub fn comments(&self) -> comments::CommentsResource<'a, M> {
        comments::CommentsResource::new(self.core, self.mode)
    }

    /// List notes
    pub fn list(&self) -> Request<'a, NotesListParams, M> {
        let params = NotesListParams {
            after: None,
            tag: None,
        };
        Request::new(self.core, params, self.mode)
    }

    /// Replace a note
    #[deprecated]
    pub fn update(
        &self,
        note_id: impl Into<String>,
        content: crate::shared::NoteContent,
        created_at: impl Into<String>,
        id: impl Into<String>,
        title: impl Into<String>,
    ) -> Request<'a, NotesUpdateParams, M> {
        let params = NotesUpdateParams {
            note_id: note_id.into(),
            x_request_source: None,
            attributes: None,
            content,
            created_at: created_at.into(),
            id: id.into(),
            parent: None,
            score: None,
            tags: None,
            title: title.into(),
        };
        Request::new(self.core, params, self.mode)
    }

    /// Delete a note
    pub fn delete(&self, note_id: impl Into<String>) -> Request<'a, NotesDeleteParams, M> {
        let params = NotesDeleteParams {
            note_id: note_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }

    /// Archive every note
    pub fn archive(&self) -> Request<'a, NotesArchiveParams, M> {
        let params = NotesArchiveParams {};
        Request::new(self.core, params, self.mode)
    }

    /// Pin a note
    pub fn pin(&self, note_id: impl Into<String>) -> Request<'a, NotesPinParams, M> {
        let params = NotesPinParams {
            note_id: note_id.into(),
        };
        Request::new(self.core, params, self.mode)
    }
}

/// The params of `list` (GET `/v1/notes`).
#[derive(Debug, Clone, PartialEq)]
pub struct NotesListParams {
    pub after: Option<String>,
    /// Only notes with every tag.
    pub tag: Option<Vec<String>>,
}

impl<M> Request<'_, NotesListParams, M> {
    pub fn after(mut self, after: impl Into<String>) -> Self {
        self.params_mut().after = Some(after.into());
        self
    }

    /// Only notes with every tag.
    pub fn tag(mut self, tag: Vec<String>) -> Self {
        self.params_mut().tag = Some(tag);
        self
    }
}

impl Endpoint for NotesListParams {
    type Output = Page<crate::shared::Note>;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(reqwest::Method::GET, "/v1/notes".to_owned());
        spec.query("after", &self.after)?;
        spec.query("tag", &self.tag)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Self::Output> {
        decode_page(&crate::pages::ITEMS_CURSOR, response, spec).await
    }
}

/// The params of `update` (PUT `/v1/notes/{note_id}`).
#[derive(Debug, Clone, PartialEq)]
pub struct NotesUpdateParams {
    pub note_id: String,
    pub x_request_source: Option<String>,
    pub attributes: Option<std::collections::BTreeMap<String, String>>,
    /// Plain text or one block.
    pub content: crate::shared::NoteContent,
    pub created_at: String,
    pub id: String,
    /// The note this one answers.
    pub parent: Option<Box<crate::shared::Note>>,
    pub score: Option<f32>,
    pub tags: Option<Vec<String>>,
    pub title: String,
}

impl<M> Request<'_, NotesUpdateParams, M> {
    pub fn x_request_source(mut self, x_request_source: impl Into<String>) -> Self {
        self.params_mut().x_request_source = Some(x_request_source.into());
        self
    }

    pub fn attributes(mut self, attributes: std::collections::BTreeMap<String, String>) -> Self {
        self.params_mut().attributes = Some(attributes);
        self
    }

    /// The note this one answers.
    pub fn parent(mut self, parent: Box<crate::shared::Note>) -> Self {
        self.params_mut().parent = Some(parent);
        self
    }

    pub fn score(mut self, score: f32) -> Self {
        self.params_mut().score = Some(score);
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.params_mut().tags = Some(tags);
        self
    }
}

impl Endpoint for NotesUpdateParams {
    type Output = crate::shared::Note;

    fn spec(&self) -> Result<RequestSpec> {
        let mut spec = RequestSpec::new(
            reqwest::Method::PUT,
            format!("/v1/notes/{}", path_param(&self.note_id)?),
        );
        spec.json(&serde_json::Map::new())?;
        spec.header("X-Request-Source", &self.x_request_source)?;
        spec.body_field("attributes", &self.attributes, false)?;
        spec.body_field("content", &self.content, true)?;
        spec.body_field("created_at", &self.created_at, true)?;
        spec.body_field("id", &self.id, true)?;
        spec.body_field("parent", &self.parent, false)?;
        spec.body_field("score", &self.score, false)?;
        spec.body_field("tags", &self.tags, false)?;
        spec.body_field("title", &self.title, true)?;
        Ok(spec)
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

/// The params of `delete` (DELETE `/v1/notes/{note_id}`).
#[derive(Debug, Clone, PartialEq)]
pub struct NotesDeleteParams {
    pub note_id: String,
}

impl Endpoint for NotesDeleteParams {
    type Output = ();

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::DELETE,
            format!("/v1/notes/{}", path_param(&self.note_id)?),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_bytes(response).await.map(drop)
    }
}

/// The params of `archive` (POST `/v1/notes/archive`).
#[derive(Debug, Clone, PartialEq)]
pub struct NotesArchiveParams {}

impl Endpoint for NotesArchiveParams {
    type Output = ();

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::POST,
            "/v1/notes/archive".to_owned(),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_bytes(response).await.map(drop)
    }
}

/// The params of `pin` (POST `/v1/notes/{note_id}`).
#[derive(Debug, Clone, PartialEq)]
pub struct NotesPinParams {
    pub note_id: String,
}

impl Endpoint for NotesPinParams {
    type Output = NotesPinResponse;

    fn spec(&self) -> Result<RequestSpec> {
        Ok(RequestSpec::new(
            reqwest::Method::POST,
            format!("/v1/notes/{}", path_param(&self.note_id)?),
        ))
    }

    async fn decode(response: reqwest::Response, _spec: RequestSpec) -> Result<Self::Output> {
        decode_json(response).await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotesPinResponse {
    pub pinned: bool,
}
