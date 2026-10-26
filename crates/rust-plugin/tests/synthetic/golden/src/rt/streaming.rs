//! Streamed responses: server-sent events and JSON Lines, as a stream of typed events.

use std::collections::VecDeque;
use std::pin::Pin;

use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::error::{Error, Result};
use super::request::{Async, Endpoint, Request};

/// The events of a streamed response, in order; an error ends the stream.
pub type EventStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send>>;

type Body = Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>;

/// One server-sent event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Sse {
    event: Option<String>,
    data: String,
}

/// Splits a byte stream into lines and lines into events (the WHATWG `text/event-stream` rules
/// for `event` and `data`; `id`, `retry` and comments are read and dropped).
#[derive(Debug, Default)]
struct SseDecoder {
    buffer: Vec<u8>,
    event: Option<String>,
    data: Vec<String>,
}

impl SseDecoder {
    fn feed(&mut self, bytes: &[u8], out: &mut Vec<Sse>) {
        self.buffer.extend_from_slice(bytes);
        while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line);
            self.line(line.trim_end_matches(['\n', '\r']), out);
        }
    }

    fn finish(&mut self, out: &mut Vec<Sse>) {
        if !self.buffer.is_empty() {
            let line = String::from_utf8_lossy(&std::mem::take(&mut self.buffer)).into_owned();
            self.line(line.trim_end_matches('\r'), out);
        }
        self.line("", out);
    }

    fn line(&mut self, line: &str, out: &mut Vec<Sse>) {
        if line.is_empty() {
            if self.event.is_some() || !self.data.is_empty() {
                out.push(Sse {
                    event: self.event.take(),
                    data: std::mem::take(&mut self.data).join("\n"),
                });
            }
            return;
        }
        if line.starts_with(':') {
            return;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => self.data.push(value.to_owned()),
            _ => {}
        }
    }
}

/// What one event means to the caller.
fn event<T: DeserializeOwned>(sse: Sse) -> Option<Result<T>> {
    if sse.data.starts_with("[DONE]") {
        return None;
    }
    if sse.event.as_deref() == Some("error") {
        let body = serde_json::from_str(&sse.data).unwrap_or(Value::String(sse.data));
        return Some(Err(Error::Stream(body)));
    }
    Some(
        serde_json::from_str(&sse.data).map_err(|source| Error::Decode {
            source,
            body: sse.data,
        }),
    )
}

#[derive(Clone, Copy)]
enum Format {
    Sse,
    Jsonl,
}

struct State<T> {
    body: Body,
    format: Format,
    sse: SseDecoder,
    lines: Vec<u8>,
    pending: VecDeque<Option<Result<T>>>,
    done: bool,
}

impl<T: DeserializeOwned> State<T> {
    fn push(&mut self, bytes: &[u8], last: bool) {
        match self.format {
            Format::Sse => {
                let mut events = Vec::new();
                self.sse.feed(bytes, &mut events);
                if last {
                    self.sse.finish(&mut events);
                }
                self.pending.extend(
                    events
                        .into_iter()
                        .filter(|e| !e.data.is_empty() || e.event.is_some())
                        .map(event),
                );
            }
            Format::Jsonl => {
                self.lines.extend_from_slice(bytes);
                let mut rest = std::mem::take(&mut self.lines);
                if !last {
                    let keep = rest.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
                    self.lines = rest.split_off(keep);
                }
                for line in rest.split(|b| *b == b'\n') {
                    let text = String::from_utf8_lossy(line);
                    let text = text.trim();
                    if !text.is_empty() {
                        let parsed = serde_json::from_str(text).map_err(|source| Error::Decode {
                            source,
                            body: text.to_owned(),
                        });
                        self.pending.push_back(Some(parsed));
                    }
                }
            }
        }
    }
}

fn events<T: DeserializeOwned + Send + 'static>(
    response: reqwest::Response,
    format: Format,
) -> EventStream<T> {
    let state = State::<T> {
        body: Box::pin(response.bytes_stream()),
        format,
        sse: SseDecoder::default(),
        lines: Vec::new(),
        pending: VecDeque::new(),
        done: false,
    };
    Box::pin(futures_util::stream::unfold(state, |mut s| async move {
        loop {
            match s.pending.pop_front() {
                // `[DONE]`: nothing after it is read.
                Some(None) => return None,
                Some(Some(Err(err))) => {
                    s.done = true;
                    s.pending.clear();
                    return Some((Err(err), s));
                }
                Some(Some(Ok(event))) => return Some((Ok(event), s)),
                None if s.done => return None,
                None => match s.body.next().await {
                    Some(Ok(bytes)) => s.push(&bytes, false),
                    Some(Err(err)) => {
                        s.done = true;
                        return Some((Err(Error::Connection(err)), s));
                    }
                    None => {
                        s.done = true;
                        s.push(&[], true);
                    }
                },
            }
        }
    }))
}

/// Reads a `text/event-stream` body as events of type `T`.
///
/// # Errors
///
/// Never: errors come as items of the stream.
#[doc(hidden)]
pub async fn decode_sse<T: DeserializeOwned + Send + 'static>(
    response: reqwest::Response,
) -> Result<EventStream<T>> {
    Ok(events(response, Format::Sse))
}

/// Reads an `application/x-ndjson` body, one `T` per line.
///
/// # Errors
///
/// Never: errors come as items of the stream.
#[doc(hidden)]
pub async fn decode_jsonl<T: DeserializeOwned + Send + 'static>(
    response: reqwest::Response,
) -> Result<EventStream<T>> {
    Ok(events(response, Format::Jsonl))
}

impl<'a, P, T> Request<'a, P, Async>
where
    P: Endpoint<Output = EventStream<T>> + Send + 'a,
{
    /// Sends the request; the events come as the server sends them.
    ///
    /// # Errors
    ///
    /// Any [`Error`] of sending the request; errors while streaming come as items.
    pub async fn stream(self) -> Result<EventStream<T>> {
        self.send().await
    }
}

/// The events of a streamed response, read as the iterator reaches them.
#[cfg(feature = "blocking")]
pub struct EventIter<'a, T> {
    runtime: &'a tokio::runtime::Runtime,
    stream: EventStream<T>,
}

#[cfg(feature = "blocking")]
impl<T> Iterator for EventIter<'_, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Result<T>> {
        self.runtime.block_on(self.stream.next())
    }
}

#[cfg(feature = "blocking")]
impl<'a, P, T> Request<'a, P, super::request::Blocking<'a>>
where
    P: Endpoint<Output = EventStream<T>>,
{
    /// Sends the request; the events are read as the iterator reaches them.
    ///
    /// # Errors
    ///
    /// Any [`Error`] of sending the request; errors while streaming come as items.
    pub fn stream(self) -> Result<EventIter<'a, T>> {
        let (core, params, options, super::request::Blocking(runtime)) = self.parts();
        let spec = params.spec()?;
        let stream = runtime.block_on(super::request::call::<P>(core, spec, &options))?;
        Ok(EventIter { runtime, stream })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(chunks: &[&str]) -> Vec<Sse> {
        let mut decoder = SseDecoder::default();
        let mut out = Vec::new();
        for chunk in chunks {
            decoder.feed(chunk.as_bytes(), &mut out);
        }
        decoder.finish(&mut out);
        out
    }

    fn sse(event: Option<&str>, data: &str) -> Sse {
        Sse {
            event: event.map(str::to_owned),
            data: data.to_owned(),
        }
    }

    #[test]
    fn splits_events_across_chunks() {
        let events = decode(&[
            "data: {\"a\"",
            ": 1}\n\nevent: ping\r\ndata: x\r\n",
            "data: y\r\n\r\n: comment\n",
        ]);
        assert_eq!(events, [sse(None, "{\"a\": 1}"), sse(Some("ping"), "x\ny")]);
        assert_eq!(decode(&["data: last"]), [sse(None, "last")]);
    }

    struct Events;

    impl Endpoint for Events {
        type Output = EventStream<i64>;

        fn spec(&self) -> Result<super::super::request::RequestSpec> {
            Ok(super::super::request::RequestSpec::new(
                reqwest::Method::GET,
                "/events".into(),
            ))
        }

        async fn decode(
            response: reqwest::Response,
            _spec: super::super::request::RequestSpec,
        ) -> Result<Self::Output> {
            decode_sse(response).await
        }
    }

    #[tokio::test]
    async fn streams_events_until_done() {
        use crate::testing::{FakeServer, Reply};
        let server = FakeServer::with_routes(&[]);
        let body = "data: 1\n\nevent: ping\ndata: 2\n\ndata: [DONE]\n\ndata: 3\n\n";
        server.enqueue(Reply {
            status: 200,
            headers: vec![("content-type".into(), "text/event-stream".into())],
            body: body.into(),
        });
        let core = crate::rt::http::tests::core(&server, 0);
        let stream = Request::new(&core, Events, Async)
            .stream()
            .await
            .expect("a stream");
        let events: Vec<i64> = stream.map(|e| e.expect("an event")).collect().await;
        assert_eq!(events, [1, 2]);
    }

    #[tokio::test]
    async fn ends_the_stream_on_an_error_event() {
        use crate::testing::{FakeServer, Reply};
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply {
            status: 200,
            headers: Vec::new(),
            body: "data: 1\n\nevent: error\ndata: {\"message\": \"overloaded\"}\n\ndata: 2\n\n"
                .into(),
        });
        let core = crate::rt::http::tests::core(&server, 0);
        let events: Vec<Result<i64>> = Request::new(&core, Events, Async)
            .stream()
            .await
            .expect("a stream")
            .collect()
            .await;
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[1], Err(Error::Stream(v)) if v["message"] == "overloaded"));
    }

    #[test]
    fn reads_done_errors_and_values() {
        assert!(event::<Value>(sse(None, "[DONE]")).is_none());
        let err = event::<Value>(sse(Some("error"), r#"{"message":"boom"}"#)).expect("an item");
        assert!(matches!(err, Err(Error::Stream(v)) if v["message"] == "boom"));
        let ok = event::<Value>(sse(None, "[1]"))
            .expect("an item")
            .expect("decodes");
        assert_eq!(ok, serde_json::json!([1]));
        assert!(matches!(
            event::<i64>(sse(None, "nope")),
            Some(Err(Error::Decode { .. }))
        ));
    }
}
