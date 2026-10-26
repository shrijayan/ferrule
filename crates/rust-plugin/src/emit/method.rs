//! One method: its params struct, the setters of its optional params, its [`Endpoint`] impl and
//! the function that starts a call, plus the test that calls it.
//!
//! A method with a streaming discriminator (`stream: true` in the body) gives two calls from
//! one emitter pass: `<name>` returns the response, `<name>_streaming` the events.
//!
//! [`Endpoint`]: the runtime's `rt::Endpoint`

use std::fmt::Write as _;

use ferrule_sdk_model::model::{
    Body, HttpMethod, Method, Pagination, PaginationScheme, Param, Scalar, StreamFormat, Type,
};
use ferrule_sdk_model::{FerCode, FerruleError};

use crate::doc::doc;
use crate::fake;
use crate::naming::{Taken, field as field_name, item, pascal, screaming};
use crate::types::{Cx, Rendered, render};

/// Where a call starts: a resource (generic over its mode) or a client.
#[derive(Debug, Clone, Copy)]
pub struct Site<'s> {
    /// `self.core` or `&self.core`.
    pub core: &'s str,
    /// `self.mode`, `Async` or `Blocking(&self.runtime)`.
    pub mode: &'s str,
    /// The `Request` type's arguments after the params: `'a, …, M`.
    pub lifetime: &'s str,
    pub mode_ty: &'s str,
    /// The path to the params types: empty, or `crate::client::` from the blocking client.
    pub params_path: &'s str,
}

/// Request names a setter may not take.
const RESERVED: [&str; 11] = [
    "send",
    "stream",
    "paginate",
    "params",
    "params_mut",
    "request_timeout",
    "request_max_retries",
    "extra_header",
    "extra_query",
    "into_future",
    "parts",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Location {
    Path,
    Query,
    Header,
    /// One field of a JSON object body.
    BodyField,
    /// The whole JSON body.
    Json,
    /// The whole body as bytes, with its media type.
    Bytes,
}

#[derive(Debug, Clone)]
struct ParamField {
    ident: String,
    wire: String,
    location: Location,
    rendered: Rendered,
    /// A positional argument rather than a setter.
    required: bool,
    /// Its value may be `None` though it is required (nullable).
    nullable: bool,
    description: String,
}

impl ParamField {
    /// The field's type in the params struct.
    fn field_ty(&self) -> String {
        if self.required && !self.nullable {
            self.rendered.ty.clone()
        } else {
            format!("Option<{}>", self.rendered.ty)
        }
    }
}

/// What a method emits into its module, and how to call it.
#[derive(Debug, Clone, Default)]
pub struct Emitted {
    /// The functions of the resource (or client) impl block, one per call.
    pub functions: Vec<String>,
    /// The tests, one per call.
    pub tests: Vec<TestCase>,
    /// The fake server's route.
    pub route: Option<Route>,
    /// The page schemes the method uses.
    pub pages: Vec<Pagination>,
}

/// One generated test: a call against the fake server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    /// The test function's name.
    pub name: String,
    /// The call after `client.`, with example arguments, before `.send()`.
    pub call: String,
    pub kind: CallKind,
    /// `GET`.
    pub http_method: String,
    /// The path the request is sent to, when every path param has a known example.
    pub path: Option<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Plain,
    Page,
    Stream,
}

/// One route of the fake server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
    /// The body param that makes it stream, and the reply then.
    pub streaming: Option<(String, &'static str, String)>,
}

/// Emits `m` of the resource `prefix` names (`Agency`) into `cx`'s module.
///
/// # Errors
///
/// FER201 for a body the Rust target cannot send yet (`multipart/form-data`).
pub fn emit(
    cx: &mut Cx<'_, '_>,
    prefix: &str,
    m: &Method,
    site: Site<'_>,
    call_prefix: &str,
) -> Result<Emitted, FerruleError> {
    let base = format!("{prefix}{}", pascal(&m.name.words, "M", "Call"));
    let mut out = Emitted::default();
    // The method's own items go ahead of the types they use, which rendering declares first.
    let start = cx.module.items.len();
    let mut own = Vec::new();
    let mut fields = params(cx, m, &base)?;
    let streams = m.streaming.as_ref();
    let discriminator = streams.and_then(|s| s.param_discriminator.clone());
    if let Some(d) = &discriminator {
        fields.retain(|f| !(f.location == Location::BodyField && f.wire == *d));
    }
    let fn_name = item(&m.name.words);
    let variants: Vec<Variant> = match (streams, &discriminator) {
        (Some(_), Some(_)) => vec![Variant::Plain, Variant::Streaming],
        (Some(_), None) => vec![Variant::Streaming],
        (None, _) => vec![Variant::Plain],
    };
    for variant in variants {
        let (params_name, call_name) = match (variant, variants_len(streams, discriminator.as_ref())) {
            (Variant::Streaming, 2) => (format!("{base}StreamingParams"), format!("{fn_name}_streaming")),
            _ => (format!("{base}Params"), fn_name.clone()),
        };
        let params_name = cx.module.taken.claim(&params_name);
        let output = output(cx, m, &base, variant);
        own.push(params_struct(m, &params_name, &call_name, &fields));
        own.extend(setters(cx, &params_name, &fields));
        own.push(endpoint(
            cx,
            m,
            &params_name,
            &fields,
            &output,
            variant,
            discriminator.as_deref(),
        ));
        out.functions
            .push(function(cx, m, &params_name, &call_name, &fields, site));
        out.tests
            .push(test_case(m, &fields, call_prefix, &call_name, output.kind));
    }
    let types: Vec<String> = cx.module.items.drain(start..).collect();
    cx.module.items.extend(own);
    cx.module.items.extend(types);
    out.pages.extend(m.pagination.clone());
    out.route = Some(route(cx, m));
    Ok(out)
}

fn variants_len(streams: Option<&ferrule_sdk_model::model::Streaming>, d: Option<&String>) -> usize {
    match (streams, d) {
        (Some(_), Some(_)) => 2,
        _ => 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Variant {
    Plain,
    Streaming,
}

fn params(cx: &mut Cx<'_, '_>, m: &Method, base: &str) -> Result<Vec<ParamField>, FerruleError> {
    let mut idents = Taken::with(&RESERVED);
    let mut out = Vec::new();
    let groups = [
        (&m.path_params, Location::Path),
        (&m.query_params, Location::Query),
        (&m.header_params, Location::Header),
    ];
    for (list, location) in groups {
        for p in list {
            out.push(param_field(cx, p, location, base, &mut idents));
        }
    }
    if let Some(body) = &m.body {
        out.extend(body_fields(cx, m, body, base, &mut idents)?);
    }
    Ok(out)
}

fn param_field(cx: &mut Cx<'_, '_>, p: &Param, location: Location, base: &str, idents: &mut Taken) -> ParamField {
    param_field_with(cx, p, location, base, idents, &[])
}

/// A param; a field of one of the `declared` models takes the type that model's struct has.
fn param_field_with(
    cx: &mut Cx<'_, '_>,
    p: &Param,
    location: Location,
    base: &str,
    idents: &mut Taken,
    declared: &[String],
) -> ParamField {
    let f = &p.field;
    let (ty, nullable) = strip_nullable(&f.ty);
    let known = {
        let fields = cx.g.fields.borrow();
        declared
            .iter()
            .rev()
            .filter_map(|schema| fields.get(schema))
            .find_map(|list| list.iter().find(|(wire, _)| *wire == f.wire_name))
            .map(|(_, r)| r.clone())
    };
    let rendered = known.map_or_else(
        || {
            let child = format!("{base}{}", pascal(&f.name.words, "P", "Param"));
            render(
                cx,
                ty,
                &child,
                Some(&f.description).filter(|d| !d.is_empty()).map(String::as_str),
            )
        },
        |r| Rendered {
            ty: r.path.clone(),
            ..r
        },
    );
    ParamField {
        ident: idents.claim(&field_name(&f.name.words)),
        wire: f.wire_name.clone(),
        location,
        rendered,
        required: f.required || location == Location::Path,
        nullable,
        description: f.description.clone(),
    }
}

fn strip_nullable(ty: &Type) -> (&Type, bool) {
    match ty {
        Type::Nullable { inner } => (inner.as_ref(), true),
        ty => (ty, false),
    }
}

fn body_fields(
    cx: &mut Cx<'_, '_>,
    m: &Method,
    body: &Body,
    base: &str,
    idents: &mut Taken,
) -> Result<Vec<ParamField>, FerruleError> {
    if body.media_type.starts_with("multipart/") {
        return Err(FerruleError::new(
            FerCode::Fer201,
            m.pointer.clone(),
            format!("the Rust target cannot send a {} body yet", body.media_type),
            "leave the method out of the Rust SDK, or send the body as JSON (ADR 0019)",
        ));
    }
    let binary = matches!(body.ty, Type::Scalar { scalar: Scalar::Binary }) || !body.media_type.contains("json");
    let description = if body.description.is_empty() {
        &body.schema_description
    } else {
        &body.description
    };
    if binary {
        return Ok(vec![ParamField {
            ident: idents.claim("body"),
            wire: String::new(),
            location: Location::Bytes,
            rendered: Rendered {
                ty: "bytes::Bytes".to_owned(),
                path: "bytes::Bytes".to_owned(),
                example: "bytes::Bytes::new()".to_owned(),
                string: false,
            },
            required: body.required,
            nullable: false,
            description: description.clone(),
        }]);
    }
    let object = cx.g.client.object_of(&body.ty).filter(|o| o.extra.is_none()).cloned();
    let Some(object) = object else {
        let (ty, nullable) = strip_nullable(&body.ty);
        let rendered = render(
            cx,
            ty,
            &format!("{base}Body"),
            Some(description).filter(|d| !d.is_empty()).map(String::as_str),
        );
        return Ok(vec![ParamField {
            ident: idents.claim("body"),
            wire: String::new(),
            location: Location::Json,
            rendered,
            required: body.required,
            nullable,
            description: description.clone(),
        }]);
    };
    let mut inherited = Vec::new();
    // The models whose declared fields the body's fields are, to reuse their types.
    let mut declared: Vec<String> = Vec::new();
    for b in &object.bases {
        if let Some(o) = cx.g.client.model(b).and_then(|d| cx.g.client.object_of(&d.ty)) {
            inherited.extend(o.fields.clone());
            declared.push(b.schema.clone());
        }
    }
    if let Type::Ref(r) = strip_nullable(&body.ty).0 {
        declared.push(r.schema.clone());
    }
    Ok(inherited
        .iter()
        .chain(&object.fields)
        .filter(|f| !f.read_only)
        .map(|f| {
            let p = Param {
                field: f.clone(),
                exploded: None,
            };
            let mut field = param_field_with(cx, &p, Location::BodyField, base, idents, &declared);
            field.required = f.required;
            field
        })
        .collect())
}

/// The output type of a call and how its response is read.
struct Output {
    ty: String,
    decode: String,
    kind: CallKind,
    uses_spec: bool,
}

fn output(cx: &mut Cx<'_, '_>, m: &Method, base: &str, variant: Variant) -> Output {
    if variant == Variant::Streaming
        && let Some(s) = &m.streaming
    {
        let event = s.event.as_ref().or(m.response.as_ref().map(|r| &r.ty));
        let event = event.map_or_else(
            || "serde_json::Value".to_owned(),
            |ty| render(cx, ty, &format!("{base}Event"), None).ty,
        );
        let (decode, rt) = match s.format {
            StreamFormat::Sse => ("decode_sse(response).await", "decode_sse"),
            StreamFormat::Jsonl => ("decode_jsonl(response).await", "decode_jsonl"),
        };
        cx.module.rt.extend(["EventStream", rt]);
        return Output {
            ty: format!("EventStream<{event}>"),
            decode: decode.to_owned(),
            kind: CallKind::Stream,
            uses_spec: false,
        };
    }
    if let Some(p) = &m.pagination {
        let item = render(cx, &p.item, &format!("{base}Response"), None).ty;
        cx.module.rt.extend(["Page", "decode_page"]);
        return Output {
            ty: format!("Page<{item}>"),
            decode: format!(
                "decode_page(&crate::pages::{}, response, spec).await",
                screaming(&p.page.words)
            ),
            kind: CallKind::Page,
            uses_spec: true,
        };
    }
    let Some(response) = &m.response else {
        cx.module.rt.insert("decode_bytes");
        return Output {
            ty: "()".to_owned(),
            decode: "decode_bytes(response).await.map(drop)".to_owned(),
            kind: CallKind::Plain,
            uses_spec: false,
        };
    };
    let name = if matches!(response.ty, Type::Array { .. }) {
        format!("{base}ResponseItem")
    } else {
        format!("{base}Response")
    };
    let ty = match &response.ty {
        Type::Array { items } => format!("Vec<{}>", render(cx, items, &name, None).ty),
        ty => render(cx, ty, &format!("{base}Response"), None).ty,
    };
    cx.module.rt.insert("decode_json");
    Output {
        ty,
        decode: "decode_json(response).await".to_owned(),
        kind: CallKind::Plain,
        uses_spec: false,
    }
}

fn params_struct(m: &Method, name: &str, call: &str, fields: &[ParamField]) -> String {
    let mut item = format!(
        "/// The params of `{call}` ({} `{}`).\n",
        m.http_method.as_str().to_uppercase(),
        crate::doc::inline(&m.path)
    );
    item.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    let _ = writeln!(item, "pub struct {name} {{");
    for f in fields {
        item.push_str(&doc(&f.description, "    "));
        let _ = writeln!(item, "    pub {}: {},", f.ident, f.field_ty());
    }
    item.push_str("}\n");
    item
}

fn setters(cx: &mut Cx<'_, '_>, params: &str, fields: &[ParamField]) -> Option<String> {
    let optional: Vec<&ParamField> = fields.iter().filter(|f| !f.required).collect();
    if optional.is_empty() {
        return None;
    }
    cx.module.rt.insert("Request");
    let mut item = format!("impl<M> Request<'_, {params}, M> {{\n");
    let mut first = true;
    for f in optional {
        if !first {
            item.push('\n');
        }
        first = false;
        item.push_str(&doc(&f.description, "    "));
        let arg = f.ident.trim_start_matches("r#");
        let (ty, value) = if f.rendered.string {
            ("impl Into<String>".to_owned(), format!("{arg}.into()"))
        } else {
            (f.rendered.ty.clone(), arg.to_owned())
        };
        let _ = writeln!(
            item,
            "    pub fn {}(mut self, {arg}: {ty}) -> Self {{\n        self.params_mut().{} = Some({value});\n        self\n    }}",
            f.ident, f.ident
        );
    }
    item.push_str("}\n");
    Some(item)
}

fn http_method(m: HttpMethod) -> String {
    match m {
        HttpMethod::Query => "reqwest::Method::from_bytes(b\"QUERY\").unwrap_or_default()".to_owned(),
        other => format!("reqwest::Method::{}", other.as_str().to_uppercase()),
    }
}

fn endpoint(
    cx: &mut Cx<'_, '_>,
    m: &Method,
    params: &str,
    fields: &[ParamField],
    output: &Output,
    variant: Variant,
    discriminator: Option<&str>,
) -> String {
    cx.module.rt.extend(["Endpoint", "RequestSpec", "Result"]);
    let mut statements = Vec::new();
    if variant == Variant::Streaming && m.streaming.as_ref().is_some_and(|s| s.format == StreamFormat::Sse) {
        statements.push("spec.accept = \"text/event-stream\";".to_owned());
    }
    let has_body_fields = fields.iter().any(|f| f.location == Location::BodyField);
    if has_body_fields || discriminator.is_some() {
        statements.push("spec.json(&serde_json::Map::new())?;".to_owned());
    }
    for f in fields {
        let ident = &f.ident;
        match f.location {
            Location::Path => {}
            Location::Query => statements.push(format!("spec.query({:?}, &self.{ident})?;", f.wire)),
            Location::Header => statements.push(format!("spec.header({:?}, &self.{ident})?;", f.wire)),
            Location::BodyField => statements.push(format!(
                "spec.body_field({:?}, &self.{ident}, {})?;",
                f.wire, f.required
            )),
            Location::Json if f.required && !f.nullable => statements.push(format!("spec.json(&self.{ident})?;")),
            Location::Json => statements.push(format!(
                "if let Some(body) = &self.{ident} {{\n            spec.json(body)?;\n        }}"
            )),
            Location::Bytes => {
                let media = m
                    .body
                    .as_ref()
                    .map_or("application/octet-stream", |b| b.media_type.as_str());
                if f.required {
                    statements.push(format!("spec.bytes({media:?}, self.{ident}.clone());"));
                } else {
                    statements.push(format!(
                        "if let Some(body) = &self.{ident} {{\n            spec.bytes({media:?}, body.clone());\n        }}"
                    ));
                }
            }
        }
    }
    if let Some(d) = discriminator {
        let on = variant == Variant::Streaming;
        if on {
            statements.push(format!("spec.body_field({d:?}, &true, true)?;"));
        }
    }
    let path = path_expr(cx, m, fields);
    let new = format!("RequestSpec::new({}, {path})", http_method(m.http_method));
    let body = if statements.is_empty() {
        format!("        Ok({new})\n")
    } else {
        format!(
            "        let mut spec = {new};\n        {}\n        Ok(spec)\n",
            statements.join("\n        ")
        )
    };
    let spec_arg = if output.uses_spec { "spec" } else { "_spec" };
    format!(
        "impl Endpoint for {params} {{\n    type Output = {};\n\n    fn spec(&self) -> Result<RequestSpec> {{\n{body}    }}\n\n    async fn decode(response: reqwest::Response, {spec_arg}: RequestSpec) -> Result<Self::Output> {{\n        {}\n    }}\n}}\n",
        output.ty, output.decode
    )
}

/// `text` as literal text of a `format!` string.
fn braces(text: &str) -> String {
    text.replace('{', "{{").replace('}', "}}")
}

/// The path with its params filled: a `format!` when it has any.
fn path_expr(cx: &mut Cx<'_, '_>, m: &Method, fields: &[ParamField]) -> String {
    let mut template = String::new();
    let mut args = Vec::new();
    let mut rest = m.path.as_str();
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else { break };
        template.push_str(&braces(&rest[..start]));
        let wire = &rest[start + 1..start + len];
        match fields.iter().find(|f| f.location == Location::Path && f.wire == wire) {
            Some(f) => {
                template.push_str("{}");
                args.push(format!("path_param(&self.{})", f.ident));
            }
            None => {
                let _ = write!(template, "{{{{{wire}}}}}");
            }
        }
        rest = &rest[start + len + 1..];
    }
    template.push_str(&braces(rest));
    if args.is_empty() {
        format!("{:?}.to_owned()", m.path)
    } else {
        cx.module.rt.insert("path_param");
        format!("format!({template:?}, {})", args.join(", "))
    }
}

fn function(
    cx: &mut Cx<'_, '_>,
    m: &Method,
    params: &str,
    call: &str,
    fields: &[ParamField],
    site: Site<'_>,
) -> String {
    cx.module.rt.insert("Request");
    let mut out = String::new();
    let text = match (m.summary.is_empty(), m.description.is_empty()) {
        (false, false) if m.description.starts_with(&m.summary) => m.description.clone(),
        (false, false) => format!("{}\n\n{}", m.summary, m.description),
        (false, true) => m.summary.clone(),
        _ => m.description.clone(),
    };
    out.push_str(&doc(&text, "    "));
    if m.deprecated {
        out.push_str("    #[deprecated]\n");
    }
    let required: Vec<&ParamField> = fields.iter().filter(|f| f.required).collect();
    if required.len() > 6 {
        out.push_str("    #[allow(clippy::too_many_arguments)]\n");
    }
    let args: Vec<String> = required
        .iter()
        .map(|f| {
            let ty = if f.rendered.string && !f.nullable {
                "impl Into<String>".to_owned()
            } else {
                f.field_ty()
            };
            format!(", {}: {ty}", f.ident)
        })
        .collect();
    let values: Vec<String> = fields
        .iter()
        .map(|f| {
            if !f.required {
                format!("{}: None", f.ident)
            } else if f.rendered.string && !f.nullable {
                format!("{}: {}.into()", f.ident, f.ident)
            } else {
                f.ident.clone()
            }
        })
        .collect();
    let params = format!("{}{params}", site.params_path);
    let _ = writeln!(
        out,
        "    pub fn {call}(&self{}) -> Request<{}, {params}{}> {{",
        args.concat(),
        site.lifetime,
        site.mode_ty
    );
    let literal = if values.is_empty() {
        format!("{params} {{}}")
    } else {
        format!("{params} {{ {} }}", values.join(", "))
    };
    let _ = writeln!(
        out,
        "        let params = {literal};\n        Request::new({}, params, {})\n    }}",
        site.core, site.mode
    );
    out
}

fn test_case(m: &Method, fields: &[ParamField], call_prefix: &str, call: &str, kind: CallKind) -> TestCase {
    let args: Vec<String> = fields
        .iter()
        .filter(|f| f.required)
        .map(|f| {
            if f.nullable {
                "None".to_owned()
            } else if f.rendered.string {
                format!("{:?}", example_text(&f.rendered))
            } else {
                f.rendered.example.clone()
            }
        })
        .collect();
    let mut path = Some(String::new());
    let mut rest = m.path.as_str();
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else { break };
        let wire = &rest[start + 1..start + len];
        let value = fields
            .iter()
            .find(|f| f.location == Location::Path && f.wire == wire)
            .and_then(path_example);
        path = match (path, value) {
            (Some(mut p), Some(v)) => {
                p.push_str(&rest[..start]);
                p.push_str(&v);
                Some(p)
            }
            _ => None,
        };
        rest = &rest[start + len + 1..];
    }
    let path = path.map(|mut p| {
        p.push_str(rest);
        p
    });
    TestCase {
        name: call.to_owned(),
        call: format!("{call_prefix}{call}({})", args.join(", ")),
        kind,
        http_method: m.http_method.as_str().to_uppercase(),
        path,
        deprecated: m.deprecated,
    }
}

/// The text of a string example: `"string".to_owned()` -> `string`.
fn example_text(r: &Rendered) -> String {
    r.example
        .strip_suffix(".to_owned()")
        .and_then(|s| s.strip_prefix('"'))
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or("string")
        .to_owned()
}

/// A path param's example as it is sent, when it is a string or a number.
fn path_example(f: &ParamField) -> Option<String> {
    if f.rendered.string {
        Some(example_text(&f.rendered))
    } else if f.rendered.example.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some(f.rendered.example.clone())
    } else {
        None
    }
}

fn route(cx: &Cx<'_, '_>, m: &Method) -> Route {
    let client = cx.g.client;
    let status = m.response.as_ref().and_then(|r| r.status.parse().ok()).unwrap_or(200);
    let json = m
        .response
        .as_ref()
        .map_or_else(String::new, |r| fake::value(client, &r.ty).to_string());
    let stream = m.streaming.as_ref().map(|s| {
        let event = s.event.as_ref().or(m.response.as_ref().map(|r| &r.ty));
        let value = event
            .map_or(serde_json::Value::Null, |ty| fake::value(client, ty))
            .to_string();
        match s.format {
            StreamFormat::Sse => ("text/event-stream", format!("data: {value}\n\ndata: [DONE]\n\n")),
            StreamFormat::Jsonl => ("application/x-ndjson", format!("{value}\n")),
        }
    });
    let discriminator = m.streaming.as_ref().and_then(|s| s.param_discriminator.clone());
    let (content_type, body, streaming) = match (stream, discriminator) {
        (Some((content_type, body)), None) => (content_type, body, None),
        (Some((content_type, body)), Some(param)) => ("application/json", json, Some((param, content_type, body))),
        (None, _) => ("application/json", json, None),
    };
    Route {
        method: m.http_method.as_str().to_uppercase(),
        path: m.path.clone(),
        status,
        content_type,
        body,
        streaming,
    }
}

/// The page schemes' `PageConfig` constants, once each, in the order first used.
#[must_use]
pub fn pages_module(pages: &[Pagination]) -> String {
    let mut out =
        String::from("//! The API's `pagination` schemes.\n\nuse crate::rt::{PageConfig, ParamLocation, Scheme};\n");
    let mut seen = Vec::new();
    for p in pages {
        let name = screaming(&p.page.words);
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());
        let scheme = match p.scheme {
            PaginationScheme::Cursor => "Cursor",
            PaginationScheme::CursorId => "CursorId",
            PaginationScheme::CursorUrl => "CursorUrl",
            PaginationScheme::Offset => "Offset",
            PaginationScheme::PageNumber => "PageNumber",
            PaginationScheme::FakePage => "FakePage",
        };
        let location = match p.param_location {
            ferrule_sdk_model::model::ParamLocation::Query => "Query",
            ferrule_sdk_model::model::ParamLocation::Body => "Body",
        };
        let pairs = |list: &[ferrule_sdk_model::model::Purpose]| {
            list.iter()
                .map(|x| format!("({:?}, {:?})", x.purpose, x.field))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = write!(
            out,
            "\n/// The `{}` scheme.\npub const {name}: PageConfig = PageConfig {{\n    scheme: Scheme::{scheme},\n    location: ParamLocation::{location},\n    params: &[{}],\n    fields: &[{}],\n}};\n",
            crate::doc::inline(&p.page.written),
            pairs(&p.request),
            pairs(&p.response)
        );
    }
    out
}
