//! A local HTTP server that answers like the API, for tests that must not reach the network.
//!
//! Each route answers with a body built from its response schema; [`FakeServer::enqueue`]
//! puts replies ahead of the routes, and [`FakeServer::requests`] shows what the client sent.
//! It serves one connection at a time on a thread of its own, so async and blocking clients can
//! both use it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

/// One operation of the API and the reply the fake server gives it.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    /// Such as `GET`.
    pub method: &'static str,
    /// The path template, such as `/agency/{agencyID}.json`.
    pub path: &'static str,
    pub status: u16,
    pub content_type: &'static str,
    pub body: &'static str,
    /// For a method that streams when a body param is `true`: the reply then.
    pub streaming: Option<StreamReply>,
}

/// The reply of a method that streams when body param `param` is `true`.
#[derive(Debug, Clone, Copy)]
pub struct StreamReply {
    pub param: &'static str,
    pub content_type: &'static str,
    pub body: &'static str,
}

/// A reply given once, before the routes are tried.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Reply {
    /// A JSON reply.
    #[must_use]
    pub fn json(status: u16, body: &serde_json::Value) -> Self {
        Self {
            status,
            headers: vec![("content-type".to_owned(), "application/json".to_owned())],
            body: body.to_string(),
        }
    }

    /// The reply with one more header.
    #[must_use]
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// A request the fake server received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub method: String,
    /// The path, without the query string.
    pub path: String,
    /// The query string, without `?`.
    pub query: String,
    /// Header names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Recorded {
    /// The value of header `name`.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Default)]
struct State {
    queue: std::collections::VecDeque<Reply>,
    requests: Vec<Recorded>,
}

/// The fake server; it stops when dropped.
#[derive(Debug)]
pub struct FakeServer {
    addr: SocketAddr,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeServer {
    /// A server for the API's routes.
    ///
    /// # Panics
    ///
    /// When no local port can be bound.
    #[must_use]
    pub fn start() -> Self {
        Self::with_routes(super::ROUTES)
    }

    /// A server for `routes`.
    ///
    /// # Panics
    ///
    /// When no local port can be bound.
    #[must_use]
    pub fn with_routes(routes: &'static [Route]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port for the fake server");
        let addr = listener.local_addr().expect("the fake server's address");
        let state = Arc::new(Mutex::new(State::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (state, stop) = (Arc::clone(&state), Arc::clone(&stop));
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(stream) = stream {
                        // A client that hangs up early is not the server's failure.
                        let _ = serve(stream, routes, &state);
                    }
                }
            })
        };
        Self {
            addr,
            state,
            stop,
            thread: Some(thread),
        }
    }

    /// The URL to point a client's `base_url` at.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Answers the next request with `reply`, whatever its route.
    pub fn enqueue(&self, reply: Reply) {
        self.lock().queue.push_back(reply);
    }

    /// Every request received so far, in order.
    #[must_use]
    pub fn requests(&self) -> Vec<Recorded> {
        self.lock().requests.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the accept loop so that it sees the flag.
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(stream: TcpStream, routes: &[Route], state: &Mutex<State>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
        return Ok(());
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = header.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        }
    }
    let length = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let recorded = Recorded {
        method: method.to_owned(),
        path: path.to_owned(),
        query: query.to_owned(),
        headers,
        body: body.clone(),
    };
    let reply = {
        let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
        state.requests.push(recorded);
        state.queue.pop_front()
    };
    let reply = reply.unwrap_or_else(|| route_reply(routes, method, path, &body));
    respond(stream, &reply)
}

fn route_reply(routes: &[Route], method: &str, path: &str, body: &[u8]) -> Reply {
    let Some(route) = routes
        .iter()
        .find(|r| r.method.eq_ignore_ascii_case(method) && matches(r.path, path))
    else {
        return Reply::json(
            404,
            &serde_json::json!({"message": format!("no route for {method} {path}")}),
        );
    };
    let streams = route.streaming.filter(|s| {
        let body: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
        body.get(s.param) == Some(&serde_json::Value::Bool(true))
    });
    let (content_type, text) = streams.map_or((route.content_type, route.body), |s| {
        (s.content_type, s.body)
    });
    Reply {
        status: route.status,
        headers: vec![("content-type".to_owned(), content_type.to_owned())],
        body: text.to_owned(),
    }
}

/// Whether `path` fits `template`, each `{param}` standing for part of one segment.
fn matches(template: &str, path: &str) -> bool {
    let (t, p): (Vec<&str>, Vec<&str>) = (template.split('/').collect(), path.split('/').collect());
    t.len() == p.len() && t.iter().zip(&p).all(|(t, p)| segment(t, p))
}

fn segment(template: &str, segment: &str) -> bool {
    // The literal pieces around the params: each param takes at least one character.
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        pieces.push(&rest[..start]);
        let end = rest[start..]
            .find('}')
            .map_or(rest.len(), |e| start + e + 1);
        rest = &rest[end..];
    }
    if pieces.is_empty() {
        return template == segment;
    }
    let Some(mut left) = segment.strip_prefix(pieces[0]) else {
        return false;
    };
    for piece in &pieces[1..] {
        let Some(at) = left.get(1..).and_then(|l| l.find(piece)) else {
            return false;
        };
        left = &left[1 + at + piece.len()..];
    }
    left.len() > rest.len() && left.ends_with(rest)
}

fn respond(mut stream: TcpStream, reply: &Reply) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {} Fake\r\n", reply.status);
    for (k, v) in &reply.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str(&format!(
        "content-length: {}\r\nconnection: close\r\n\r\n",
        reply.body.len()
    ));
    stream.write_all(head.as_bytes())?;
    stream.write_all(reply.body.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_params_inside_segments() {
        assert!(matches("/agency/{agencyID}.json", "/agency/1_40.json"));
        assert!(matches("/a/{x}/b", "/a/1/b"));
        assert!(matches("/a/{x}-{y}", "/a/1-2"));
        assert!(!matches("/agency/{agencyID}.json", "/agency/.json"));
        assert!(!matches("/agency/{agencyID}.json", "/agency/1/x.json"));
        assert!(!matches("/a/b", "/a/c"));
    }
}
