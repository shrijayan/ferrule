#![doc = include_str!("../README.md")]

/// The client and its builder.
pub mod client;
/// The API's resources.
pub mod resources;
/// The runtime the generated code is built on.
pub mod rt;
/// Models several resources share.
pub mod shared;

#[cfg(feature = "blocking")]
pub mod blocking;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use client::{Environment, OnebusawaySdk, OnebusawaySdkBuilder};
pub use rt::{ApiError, Error, EventStream, Page, PageStream, Request, RequestOptions, Result};
