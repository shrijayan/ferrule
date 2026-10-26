//! Pages: each `pagination` scheme of the API, implemented once over the page's JSON.
//!
//! A list method returns a [`Page`]; [`Request::paginate`] walks every page and yields the
//! items one by one.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::pin::Pin;

use futures_util::Stream;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::error::{Error, Result};
use super::http::Core;
use super::request::{Async, Endpoint, Request, RequestOptions, RequestSpec, call};

/// The `type` of a `pagination` scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// The response gives the next cursor; the request sends it back.
    Cursor,
    /// The request sends the id of the last item (or the first, going backwards).
    CursorId,
    /// The response gives the URL of the next page.
    CursorUrl,
    /// The request sends how many items to skip.
    Offset,
    /// The request sends the page number.
    PageNumber,
    /// One page holds every item.
    FakePage,
}

/// Where a scheme's request params go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    /// In the query string.
    Query,
    /// In the JSON body.
    Body,
}

/// One `pagination` scheme of the API, as its config declares it.
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct PageConfig {
    pub scheme: Scheme,
    pub location: ParamLocation,
    /// Request params by purpose, such as `("next_cursor_param", "after")`.
    pub params: &'static [(&'static str, &'static str)],
    /// Response fields by purpose, as dotted paths, such as `("items", "data")`.
    pub fields: &'static [(&'static str, &'static str)],
}

impl PageConfig {
    fn param(&self, purpose: &str) -> Option<&'static str> {
        self.params
            .iter()
            .find(|(p, _)| *p == purpose)
            .map(|(_, name)| *name)
    }

    fn field<'v>(&self, raw: &'v Value, purpose: &str) -> Option<&'v Value> {
        let path = self.fields.iter().find(|(p, _)| *p == purpose)?.1;
        path.split('.').try_fold(raw, |v, key| v.get(key))
    }
}

/// One page of a list.
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// The items of this page.
    pub items: Vec<T>,
    raw: Value,
    next: Option<RequestSpec>,
}

impl<T> Page<T> {
    /// The page as the server sent it.
    #[must_use]
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    /// Whether another page follows.
    #[must_use]
    pub fn has_next_page(&self) -> bool {
        self.next.is_some()
    }
}

/// Reads a page of `config` returned for `spec`.
///
/// # Errors
///
/// [`Error::Connection`] when the body cannot be read, [`Error::Decode`] when it is not a page.
#[doc(hidden)]
pub async fn decode_page<T: DeserializeOwned>(
    config: &PageConfig,
    response: reqwest::Response,
    spec: RequestSpec,
) -> Result<Page<T>> {
    let raw: Value = super::request::decode_json(response).await?;
    let items = config
        .field(&raw, "items")
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    let decoded: Vec<T> =
        serde_json::from_value(items.clone()).map_err(|source| Error::Decode {
            source,
            body: items.to_string(),
        })?;
    let item_values = items.as_array().map(Vec::as_slice).unwrap_or_default();
    let next = next_request(config, &raw, item_values, spec);
    Ok(Page {
        items: decoded,
        raw,
        next,
    })
}

/// The request of the page after `raw`, sent for `spec`; `None` on the last page.
fn next_request(
    config: &PageConfig,
    raw: &Value,
    items: &[Value],
    mut spec: RequestSpec,
) -> Option<RequestSpec> {
    let has_more = config
        .field(raw, "has_next_page")
        .or_else(|| raw.get("has_more"));
    if items.is_empty() || has_more.and_then(Value::as_bool) == Some(false) {
        return None;
    }
    match config.scheme {
        Scheme::FakePage => None,
        Scheme::Cursor => {
            let cursor = config
                .field(raw, "next_cursor_field")
                .and_then(Value::as_str)?;
            let param = config.param("next_cursor_param")?;
            (!cursor.is_empty()).then(|| set(&mut spec, config.location, param, cursor.into()))?;
            Some(spec)
        }
        Scheme::CursorId => {
            let after = config.param("next_cursor_id_param")?;
            let before = config.param("previous_cursor_id_param");
            let id_field = config
                .fields
                .iter()
                .find(|(p, _)| *p == "cursor_item_id")
                .map(|(_, f)| *f)?;
            let id_field = id_field.rsplit('.').next().unwrap_or(id_field);
            let backwards = before.is_some_and(|b| get(&spec, config.location, b).is_some());
            let (item, param) = if backwards {
                (items.first()?, before?)
            } else {
                (items.last()?, after)
            };
            let id = item.get(id_field).filter(|v| !v.is_null())?.clone();
            remove(
                &mut spec,
                config.location,
                if backwards {
                    after
                } else {
                    before.unwrap_or(after)
                },
            );
            set(&mut spec, config.location, param, id);
            Some(spec)
        }
        Scheme::CursorUrl => {
            let url = config
                .field(raw, "cursor_url_field")
                .and_then(Value::as_str)?;
            (!url.is_empty()).then(|| follow_url(&mut spec, url))?;
            Some(spec)
        }
        Scheme::Offset => {
            let param = config.param("offset_count_param")?;
            let current = get(&spec, config.location, param)
                .as_ref()
                .and_then(as_u64)
                .unwrap_or(0);
            let next = current + items.len() as u64;
            let total = config
                .field(raw, "offset_total_count_field")
                .and_then(as_u64);
            if total.is_some_and(|t| next >= t) {
                return None;
            }
            set(&mut spec, config.location, param, next.into());
            Some(spec)
        }
        Scheme::PageNumber => {
            let param = config.param("page_number_param")?;
            let current = config
                .field(raw, "current_page_number_field")
                .and_then(as_u64)
                .or_else(|| get(&spec, config.location, param).as_ref().and_then(as_u64))
                .unwrap_or(1);
            let total = config.field(raw, "total_page_count_field").and_then(as_u64);
            if total.is_some_and(|t| current >= t) {
                return None;
            }
            set(&mut spec, config.location, param, (current + 1).into());
            Some(spec)
        }
    }
}

fn as_u64(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str()?.parse().ok())
}

fn body(spec: &RequestSpec) -> Option<serde_json::Map<String, Value>> {
    serde_json::from_slice(&spec.body.as_ref()?.bytes).ok()
}

fn get(spec: &RequestSpec, location: ParamLocation, name: &str) -> Option<Value> {
    match location {
        ParamLocation::Query => spec
            .query
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone()),
        ParamLocation::Body => body(spec)?.get(name).filter(|v| !v.is_null()).cloned(),
    }
}

fn remove(spec: &mut RequestSpec, location: ParamLocation, name: &str) {
    match location {
        ParamLocation::Query => spec.query.retain(|(k, _)| k != name),
        ParamLocation::Body => {
            if let Some(mut fields) = body(spec) {
                fields.remove(name);
                set_body(spec, fields);
            }
        }
    }
}

fn set(spec: &mut RequestSpec, location: ParamLocation, name: &str, value: Value) {
    match location {
        ParamLocation::Query => {
            spec.query.retain(|(k, _)| k != name);
            spec.query.push((name.to_owned(), value));
        }
        ParamLocation::Body => {
            let mut fields = body(spec).unwrap_or_default();
            fields.insert(name.to_owned(), value);
            set_body(spec, fields);
        }
    }
}

fn set_body(spec: &mut RequestSpec, fields: serde_json::Map<String, Value>) {
    spec.bytes(
        "application/json",
        serde_json::to_vec(&fields).unwrap_or_default().into(),
    );
}

/// Points `spec` at `url`: its query params replace those of the same name.
fn follow_url(spec: &mut RequestSpec, url: &str) {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    spec.path = path.to_owned();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let (k, v) = (unpercent(k), unpercent(v));
        spec.query.retain(|(name, _)| *name != k);
        spec.query.push((k, Value::String(v)));
    }
}

/// Decodes `%XX` escapes and `+` in a query string part.
fn unpercent(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(b) => {
                        out.push(b);
                        i += 3;
                        continue;
                    }
                    None => out.push(b'%'),
                }
            }
            b'+' => out.push(b' '),
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Every item of every page, in order; an error ends the stream.
pub type PageStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T>> + Send + 'a>>;

struct Walk<'a, P, T> {
    core: &'a Core,
    options: RequestOptions,
    next: Option<Result<RequestSpec>>,
    buffer: VecDeque<T>,
    endpoint: PhantomData<fn() -> P>,
}

fn walk<'a, P, T>(core: &'a Core, params: &P, options: RequestOptions) -> PageStream<'a, T>
where
    P: Endpoint<Output = Page<T>> + 'a,
    T: Send + 'a,
{
    let state = Walk::<P, T> {
        core,
        options,
        next: Some(params.spec()),
        buffer: VecDeque::new(),
        endpoint: PhantomData,
    };
    Box::pin(futures_util::stream::unfold(state, |mut s| async move {
        loop {
            if let Some(item) = s.buffer.pop_front() {
                return Some((Ok(item), s));
            }
            let spec = match s.next.take()? {
                Ok(spec) => spec,
                Err(err) => return Some((Err(err), s)),
            };
            match call::<P>(s.core, spec, &s.options).await {
                Ok(page) => {
                    s.next = page.next.map(Ok);
                    s.buffer = page.items.into();
                }
                Err(err) => return Some((Err(err), s)),
            }
        }
    }))
}

impl<'a, P, T> Request<'a, P, Async>
where
    P: Endpoint<Output = Page<T>> + 'a,
    T: Send + 'a,
{
    /// Every item of every page, fetching the next page as the stream reaches it.
    pub fn paginate(self) -> PageStream<'a, T> {
        let (core, params, options, _) = self.parts();
        walk(core, &params, options)
    }
}

/// Every item of every page, fetching the next page as the iterator reaches it.
#[cfg(feature = "blocking")]
pub struct PageIter<'a, T> {
    runtime: &'a tokio::runtime::Runtime,
    stream: PageStream<'a, T>,
}

#[cfg(feature = "blocking")]
impl<T> Iterator for PageIter<'_, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Result<T>> {
        self.runtime
            .block_on(futures_util::StreamExt::next(&mut self.stream))
    }
}

#[cfg(feature = "blocking")]
impl<'a, P, T> Request<'a, P, super::request::Blocking<'a>>
where
    P: Endpoint<Output = Page<T>> + 'a,
    T: Send + 'a,
{
    /// Every item of every page, fetching the next page as the iterator reaches it.
    pub fn paginate(self) -> PageIter<'a, T> {
        let (core, params, options, super::request::Blocking(runtime)) = self.parts();
        PageIter {
            runtime,
            stream: walk(core, &params, options),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec() -> RequestSpec {
        RequestSpec::new(reqwest::Method::GET, "/items".into())
    }

    fn next(config: &PageConfig, raw: &Value, spec: RequestSpec) -> Option<RequestSpec> {
        let items = config
            .field(raw, "items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        next_request(config, raw, &items, spec)
    }

    fn query(spec: &RequestSpec) -> Vec<(String, Value)> {
        spec.query.clone()
    }

    #[test]
    fn cursor_sends_the_next_cursor_back() {
        let config = PageConfig {
            scheme: Scheme::Cursor,
            location: ParamLocation::Query,
            params: &[("next_cursor_param", "after")],
            fields: &[("items", "data"), ("next_cursor_field", "page_info.after")],
        };
        let raw = json!({"data": [1], "page_info": {"after": "c2"}});
        let n = next(&config, &raw, spec()).expect("a next page");
        assert_eq!(query(&n), [("after".to_owned(), json!("c2"))]);
        assert!(
            next(
                &config,
                &json!({"data": [1], "page_info": {"after": null}}),
                spec()
            )
            .is_none()
        );
        assert!(
            next(
                &config,
                &json!({"data": [], "page_info": {"after": "c3"}}),
                spec()
            )
            .is_none()
        );
    }

    #[test]
    fn cursor_id_sends_the_last_id_or_the_first_going_backwards() {
        let config = PageConfig {
            scheme: Scheme::CursorId,
            location: ParamLocation::Query,
            params: &[
                ("next_cursor_id_param", "starting_after"),
                ("previous_cursor_id_param", "ending_before"),
            ],
            fields: &[("items", "data"), ("cursor_item_id", "data.[].id")],
        };
        let raw = json!({"data": [{"id": "a"}, {"id": "b"}]});
        let forwards = next(&config, &raw, spec()).expect("a next page");
        assert_eq!(
            query(&forwards),
            [("starting_after".to_owned(), json!("b"))]
        );
        let mut back = spec();
        back.query("ending_before", &"z").expect("encodes");
        let backwards = next(&config, &raw, back).expect("a previous page");
        assert_eq!(
            query(&backwards),
            [("ending_before".to_owned(), json!("a"))]
        );
        assert!(
            next(
                &config,
                &json!({"data": [{"id": "a"}], "has_more": false}),
                spec()
            )
            .is_none()
        );
    }

    #[test]
    fn cursor_url_follows_the_url() {
        let config = PageConfig {
            scheme: Scheme::CursorUrl,
            location: ParamLocation::Query,
            params: &[],
            fields: &[("items", "items"), ("cursor_url_field", "next")],
        };
        let raw = json!({"items": [1], "next": "https://api.test/items?page=2&q=a%20b"});
        let n = next(&config, &raw, spec()).expect("a next page");
        assert_eq!(n.path, "https://api.test/items");
        assert_eq!(
            query(&n),
            [
                ("page".to_owned(), json!("2")),
                ("q".to_owned(), json!("a b"))
            ]
        );
    }

    #[test]
    fn offset_skips_what_was_read_until_the_total() {
        let config = PageConfig {
            scheme: Scheme::Offset,
            location: ParamLocation::Body,
            params: &[("offset_count_param", "offset")],
            fields: &[("items", "data"), ("offset_total_count_field", "total")],
        };
        let mut first = RequestSpec::new(reqwest::Method::POST, "/search".into());
        first.json(&json!({"q": "x"})).expect("encodes");
        let n = next(&config, &json!({"data": [1, 2], "total": 5}), first).expect("a next page");
        assert_eq!(
            body(&n).expect("a body"),
            *json!({"q": "x", "offset": 2}).as_object().expect("object")
        );
        assert!(next(&config, &json!({"data": [3, 4, 5], "total": 5}), n).is_none());
    }

    #[test]
    fn page_number_counts_up_to_the_total() {
        let config = PageConfig {
            scheme: Scheme::PageNumber,
            location: ParamLocation::Query,
            params: &[("page_number_param", "page")],
            fields: &[
                ("items", "data"),
                ("current_page_number_field", "page"),
                ("total_page_count_field", "pages"),
            ],
        };
        let n = next(
            &config,
            &json!({"data": [1], "page": 1, "pages": 2}),
            spec(),
        )
        .expect("a next page");
        assert_eq!(query(&n), [("page".to_owned(), json!(2))]);
        assert!(next(&config, &json!({"data": [1], "page": 2, "pages": 2}), n).is_none());
    }

    const CURSOR: PageConfig = PageConfig {
        scheme: Scheme::Cursor,
        location: ParamLocation::Query,
        params: &[("next_cursor_param", "after")],
        fields: &[("items", "data"), ("next_cursor_field", "next")],
    };

    struct List;

    impl Endpoint for List {
        type Output = Page<i64>;

        fn spec(&self) -> Result<RequestSpec> {
            Ok(RequestSpec::new(reqwest::Method::GET, "/items".into()))
        }

        async fn decode(response: reqwest::Response, spec: RequestSpec) -> Result<Page<i64>> {
            decode_page(&CURSOR, response, spec).await
        }
    }

    #[tokio::test]
    async fn paginate_walks_every_page() {
        use crate::testing::{FakeServer, Reply};
        use futures_util::StreamExt;
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply::json(200, &json!({"data": [1, 2], "next": "c2"})));
        server.enqueue(Reply::json(200, &json!({"data": [3], "next": null})));
        let core = crate::rt::http::tests::core(&server, 0);
        let items: Vec<Result<i64>> = Request::new(&core, List, Async).paginate().collect().await;
        let items: Vec<i64> = items.into_iter().map(|i| i.expect("an item")).collect();
        assert_eq!(items, [1, 2, 3]);
        let queries: Vec<String> = server.requests().into_iter().map(|r| r.query).collect();
        assert_eq!(queries, ["", "after=c2"]);
    }

    #[tokio::test]
    async fn send_returns_the_first_page() {
        use crate::testing::{FakeServer, Reply};
        let server = FakeServer::with_routes(&[]);
        server.enqueue(Reply::json(200, &json!({"data": [1], "next": "c2"})));
        let core = crate::rt::http::tests::core(&server, 0);
        let page = Request::new(&core, List, Async).await.expect("a page");
        assert_eq!(page.items, [1]);
        assert!(page.has_next_page());
        assert_eq!(page.raw()["next"], "c2");
    }

    #[test]
    fn fake_page_has_one_page() {
        let config = PageConfig {
            scheme: Scheme::FakePage,
            location: ParamLocation::Query,
            params: &[],
            fields: &[("items", "data")],
        };
        assert!(next(&config, &json!({"data": [1]}), spec()).is_none());
    }
}
