//! The runtime every generated SDK shares: sending requests with retries and timeouts,
//! errors, pagination and streaming. The API's own code (resources, types, the client) is
//! generated around it.

pub mod error;
pub mod http;
pub mod multipart;
pub mod pagination;
pub mod query;
pub mod request;
pub mod streaming;

pub use error::{ApiError, Error, Result};
pub use http::{Config, Core};
pub use multipart::FileUpload;
pub use pagination::{Page, PageConfig, PageStream, ParamLocation, Scheme, decode_page};
pub use query::{ArrayFormat, NestedFormat};
#[cfg(feature = "blocking")]
pub use request::Blocking;
pub use request::{
    Async, Endpoint, Request, RequestOptions, RequestSpec, decode_bytes, decode_json, path_param,
};
pub use streaming::{EventStream, decode_jsonl, decode_sse};

#[cfg(feature = "blocking")]
pub use pagination::PageIter;
#[cfg(feature = "blocking")]
pub use streaming::EventIter;

/// The tokio runtime of a blocking client.
///
/// # Errors
///
/// [`Error::Config`] when the runtime cannot be started.
#[cfg(feature = "blocking")]
#[doc(hidden)]
pub fn blocking_runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| Error::Config(format!("could not start the blocking runtime: {err}")))
}

/// The value of client option `name`: the one given, else environment variable `env`.
#[doc(hidden)]
#[must_use]
pub fn from_env(given: Option<String>, env: &str) -> Option<String> {
    given.or_else(|| std::env::var(env).ok().filter(|v| !v.is_empty()))
}

/// `Basic` credentials: `user:password` in base64.
#[doc(hidden)]
#[must_use]
pub fn basic_auth(user: &str, password: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = format!("{user}:{password}");
    let mut out = String::from("Basic ");
    for chunk in input.as_bytes().chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_basic_credentials() {
        assert_eq!(
            super::basic_auth("Aladdin", "open sesame"),
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
        assert_eq!(super::basic_auth("a", ""), "Basic YTo=");
    }
}
