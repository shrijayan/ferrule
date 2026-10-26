//! The blocking client (`blocking` feature): the same resources and requests, whose `send`
//! blocks on a runtime the client owns.

use crate::rt::{Blocking, Result, blocking_runtime};

/// A client whose calls block until they are done.
#[derive(Debug)]
pub struct AcmeNotes {
    inner: crate::AcmeNotes,
    runtime: tokio::runtime::Runtime,
}

impl AcmeNotes {
    /// A client with every option read from the environment.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Config`] when a required option is not set or the runtime cannot be
    /// started.
    pub fn new() -> Result<Self> {
        crate::AcmeNotes::builder().build_blocking()
    }

    /// A blocking client that shares `inner`'s settings and connection pool.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Config`] when the runtime cannot be started.
    pub fn from_async(inner: crate::AcmeNotes) -> Result<Self> {
        Ok(Self {
            inner,
            runtime: blocking_runtime()?,
        })
    }

    /// The `notes` resource.
    pub fn notes(&self) -> crate::resources::notes::NotesResource<'_, Blocking<'_>> {
        crate::resources::notes::NotesResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `completions` resource.
    pub fn completions(
        &self,
    ) -> crate::resources::completions::CompletionsResource<'_, Blocking<'_>> {
        crate::resources::completions::CompletionsResource::new(
            &self.inner.core,
            Blocking(&self.runtime),
        )
    }

    /// The `files` resource.
    pub fn files(&self) -> crate::resources::files::FilesResource<'_, Blocking<'_>> {
        crate::resources::files::FilesResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `tags` resource.
    pub fn tags(&self) -> crate::resources::tags::TagsResource<'_, Blocking<'_>> {
        crate::resources::tags::TagsResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `events` resource.
    pub fn events(&self) -> crate::resources::events::EventsResource<'_, Blocking<'_>> {
        crate::resources::events::EventsResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `users` resource.
    pub fn users(&self) -> crate::resources::users::UsersResource<'_, Blocking<'_>> {
        crate::resources::users::UsersResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `logs` resource.
    pub fn logs(&self) -> crate::resources::logs::LogsResource<'_, Blocking<'_>> {
        crate::resources::logs::LogsResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// The `labels` resource.
    pub fn labels(&self) -> crate::resources::labels::LabelsResource<'_, Blocking<'_>> {
        crate::resources::labels::LabelsResource::new(&self.inner.core, Blocking(&self.runtime))
    }

    /// Health check
    pub fn health(
        &self,
    ) -> crate::rt::Request<'_, crate::client::AcmeNotesHealthParams, Blocking<'_>> {
        let params = crate::client::AcmeNotesHealthParams {};
        crate::rt::Request::new(&self.inner.core, params, Blocking(&self.runtime))
    }
}
