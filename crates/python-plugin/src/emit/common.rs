//! Helpers shared by the emitters.

use crate::js;
use crate::model::{
    SdkClient, SdkField, SdkLiteral, SdkMethod, SdkObject, SdkParam, SdkPrimitive, SdkRef, SdkResource, SdkType,
};
use crate::naming::{class_segment, singular, snake_case, split_words};

/// First line of every generated file (no vendor branding, per the plan).
pub const HEADER: &str = "# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.";

/// The relative import of `target` from module `from`, both dotted paths from the package root:
/// `rel_import("types.foo", "_models")` is `.._models`.
#[must_use]
pub fn rel_import(from: &str, target: &str) -> String {
    let parts: Vec<&str> = from.split('.').collect();
    let pkg = &parts[..parts.len() - 1];
    let tgt: Vec<&str> = target.split('.').collect();
    let mut common = 0;
    while common < pkg.len() && common + 1 < tgt.len() && pkg[common] == tgt[common] {
        common += 1;
    }
    format!(".{}{}", ".".repeat(pkg.len() - common), tgt[common..].join("."))
}

/// A Python string literal for any text. JSON's escaping (quotes, backslashes, control characters
/// as `\uXXXX`) is valid in a Python `"..."` literal, so no text can end it. Same bytes as
/// JavaScript's `JSON.stringify(text)`.
#[must_use]
pub fn py_string(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// An f-string literal holding `text` verbatim: braces are doubled so nothing interpolates.
#[must_use]
pub fn py_fstring(text: &str) -> String {
    format!("f{}", py_string(text).replace('{', "{{").replace('}', "}}"))
}

/// Makes spec text safe inside a `"""` docstring: backslashes doubled (as Stainless does) and
/// quotes that could close the docstring escaped, so a spec can't inject code into the SDK.
#[must_use]
pub fn escape_docstring(text: &str) -> String {
    let out = text.replace('\\', "\\\\").replace("\"\"\"", "\\\"\\\"\\\"");
    match out.strip_suffix('"') {
        Some(head) => format!("{head}\\\""),
        None => out,
    }
}

/// Greedy word wrap at `width` columns (UTF-16 code units, as the TypeScript plugin measures).
#[must_use]
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in js::words(para) {
            if cur.is_empty() {
                word.clone_into(&mut cur);
            } else if js::len(&cur) + 1 + js::len(word) <= width {
                cur.push(' ');
                cur.push_str(word);
            } else {
                lines.push(std::mem::replace(&mut cur, word.to_owned()));
            }
        }
        lines.push(cur);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// What [`sort_fields`] and the namers read of a field or parameter.
pub trait FieldLike {
    fn wire_name(&self) -> &str;
    fn py_name(&self) -> &str;
    fn required(&self) -> bool;
    fn description(&self) -> &str;
    fn ty(&self) -> &SdkType;
}

impl FieldLike for SdkField {
    fn wire_name(&self) -> &str {
        &self.wire_name
    }
    fn py_name(&self) -> &str {
        &self.py_name
    }
    fn required(&self) -> bool {
        self.required
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn ty(&self) -> &SdkType {
        &self.ty
    }
}

impl FieldLike for SdkParam {
    fn wire_name(&self) -> &str {
        &self.wire_name
    }
    fn py_name(&self) -> &str {
        &self.py_name
    }
    fn required(&self) -> bool {
        self.required
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn ty(&self) -> &SdkType {
        &self.ty
    }
}

/// Whether `t` holds an inline object, which would need a class name of its own.
fn has_inline_object(t: &SdkType) -> bool {
    match t {
        SdkType::Object(_) => true,
        SdkType::Array(inner) | SdkType::Map(inner) => has_inline_object(inner),
        SdkType::Union { variants, .. } => variants.iter().any(has_inline_object),
        _ => false,
    }
}

/// The type of the other keys an object with fields takes, when the emitters type them: values
/// that are inline objects are left out, since they would need names Stainless gives them.
#[must_use]
pub fn typed_extra(obj: &SdkObject) -> Option<&SdkType> {
    obj.extra.as_deref().filter(|t| !has_inline_object(t))
}

/// Field order in the oracle SDKs: required before optional; within each, `id` first, then by
/// wire name ignoring case.
pub fn sort_fields<'a, T: FieldLike + 'a>(fields: impl IntoIterator<Item = &'a T>) -> Vec<&'a T> {
    let mut out: Vec<&T> = fields.into_iter().collect();
    out.sort_by(|a, b| {
        b.required()
            .cmp(&a.required())
            .then_with(|| (b.wire_name() == "id").cmp(&(a.wire_name() == "id")))
            .then_with(|| js::cmp(&a.wire_name().to_lowercase(), &b.wire_name().to_lowercase()))
    });
    out
}

/// The last segment of a resource's path, such as `agencies_with_coverage`.
#[must_use]
pub fn leaf(resource: &SdkResource) -> &str {
    resource.path.last().map_or("", String::as_str)
}

/// `AgenciesWithCoverage` for `["agencies_with_coverage"]`, `QA` for `["qa"]` with the `qa`
/// initialism (the adapter spells it).
#[must_use]
pub fn resource_class_base(resource: &SdkResource) -> String {
    resource
        .class_segment
        .clone()
        .unwrap_or_else(|| class_segment(&split_words(leaf(resource))))
}

/// Whether the resource or one below it has methods, so it has a class.
#[must_use]
pub fn has_methods(resource: &SdkResource) -> bool {
    !resource.methods.is_empty() || resource.subresources.iter().any(has_methods)
}

/// Every resource the config declares that the model knows of ([`SdkClient::resource_tree`]),
/// or `resources` for a hand-built model without a tree.
#[must_use]
pub fn resource_tree(client: &SdkClient) -> &[SdkResource] {
    if client.resource_tree.is_empty() {
        &client.resources
    } else {
        &client.resource_tree
    }
}

/// Whether the resource is a package (`resources/<path>/<leaf>.py` and an `__init__.py`): a
/// standalone API, or one with subresources that have classes.
#[must_use]
pub fn is_package(resource: &SdkResource) -> bool {
    resource.standalone || resource.subresources.iter().any(has_methods)
}

/// The dotted module of a resource: `resources.stop`, or `resources.generations.generations` for
/// a package ([`is_package`]).
#[must_use]
pub fn resource_module(resource: &SdkResource) -> String {
    let mut base: Vec<&str> = vec!["resources"];
    base.extend(resource.path.iter().map(String::as_str));
    if is_package(resource) {
        base.push(leaf(resource));
    }
    base.join(".")
}

/// The directory (dotted, under `types`) of a resource's types: its parent's, or its own for a
/// standalone API.
#[must_use]
pub fn types_dir(resource: &SdkResource) -> &[String] {
    if resource.standalone {
        &resource.path
    } else {
        &resource.path[..resource.path.len().saturating_sub(1)]
    }
}

/// Standard methods come first in this order; the rest follow by name (as the oracles order).
const STANDARD: [&str; 5] = ["create", "retrieve", "update", "list", "delete"];

#[must_use]
pub fn stainless_method_order(methods: &[SdkMethod]) -> Vec<&SdkMethod> {
    let rank = |m: &SdkMethod| STANDARD.iter().position(|&s| s == m.name).unwrap_or(STANDARD.len());
    let mut out: Vec<&SdkMethod> = methods.iter().collect();
    out.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| js::cmp(&a.name, &b.name)));
    out
}

/// A synthesized type named after its method, with the resource's last word singular as the
/// oracles have it: `AgenciesWithCoverageListResponse`, `CardProgramListParams`. Types of a
/// subresource sit in its parent's directory: `types/generations/video_create_params.py`, those of
/// a standalone API in its own ([`types_dir`]). `$client` methods (no resource) are named after
/// the method alone.
#[must_use]
pub fn method_ref(resource: Option<&SdkResource>, method: &SdkMethod, suffix: &str) -> SdkRef {
    let words = resource.map_or_else(Vec::new, |r| singular(&split_words(leaf(r))));
    let dir: &[String] = resource.map_or(&[], types_dir);
    let class = resource
        .and_then(|r| r.singular_class_segment.clone())
        .unwrap_or_else(|| class_segment(&words));
    let lower = suffix.to_lowercase();
    let snake = snake_case(&words);
    let leaf_name: Vec<&str> = [snake.as_str(), method.name.as_str(), lower.as_str()]
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    let mut module: Vec<String> = dir.to_vec();
    module.push(leaf_name.join("_"));
    SdkRef {
        name: format!("{class}{}{suffix}", class_segment(&split_words(&method.name))),
        module: module.join("."),
    }
}

/// What a method returns, from [`return_model`].
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnModel<'a> {
    pub r#ref: SdkRef,
    /// The inline type to synthesize, or `None` for a named model.
    pub inline: Option<&'a SdkType>,
}

/// What a method returns: for a paginated method, one item; for one that always streams, one
/// event; otherwise (a param-switched streaming method too) the response. A named model stays a
/// ref; an inline object becomes a synthesized `<Resource><Method>Response`.
#[must_use]
pub fn return_model<'a>(resource: Option<&SdkResource>, method: &'a SdkMethod) -> Option<ReturnModel<'a>> {
    let t = method
        .pagination
        .as_ref()
        .map(|p| &p.item)
        .or_else(|| {
            method
                .streaming
                .as_ref()
                .filter(|_| !is_param_switched(method))
                .and_then(|s| s.event.as_ref())
        })
        .or(method.response.as_ref())?;
    Some(typed_return(resource, method, t))
}

/// Whether a streaming method streams only when a body parameter says so (`streaming.
/// param_discriminator`): Stainless then writes an overload per value of that parameter.
#[must_use]
pub fn is_param_switched(method: &SdkMethod) -> bool {
    method
        .streaming
        .as_ref()
        .and_then(|s| s.param_discriminator.as_deref())
        .is_some_and(|d| !d.is_empty())
}

/// The body parameter that switches a param-switched streaming method to streaming (ogx
/// `responses.create(stream=...)`, stagehand `sessions.act(stream_response=...)`), or `None`
/// when the method is not param-switched or no body parameter has that wire name.
#[must_use]
pub fn stream_switch(method: &SdkMethod) -> Option<&SdkParam> {
    let wire = method.streaming.as_ref()?.param_discriminator.as_deref()?;
    method.body_params.iter().find(|q| q.wire_name == wire)
}

/// The type of one event of a param-switched streaming method: the named event model, else
/// (no `stream_event_model`) the response, as ogx `completions.create` returns
/// `Stream[CompletionCreateResponse]`.
#[must_use]
pub fn stream_event_model<'a>(resource: Option<&SdkResource>, method: &'a SdkMethod) -> Option<ReturnModel<'a>> {
    let t = method
        .streaming
        .as_ref()
        .and_then(|s| s.event.as_ref())
        .or(method.response.as_ref())?;
    Some(typed_return(resource, method, t))
}

/// `ty` with `bool` narrowed to `Literal[value]`, a nullable `bool` staying nullable: the
/// switch of a param-switched streaming method in one overload (`Optional[Literal[False]]`).
#[must_use]
pub fn with_bool_literal(ty: &SdkType, value: bool) -> SdkType {
    match ty {
        SdkType::Primitive(SdkPrimitive::Boolean) => SdkType::Literal(vec![SdkLiteral::Boolean(value)]),
        SdkType::Union {
            variants,
            discriminator,
        } => SdkType::Union {
            variants: variants.iter().map(|v| with_bool_literal(v, value)).collect(),
            discriminator: discriminator.clone(),
        },
        other => other.clone(),
    }
}

fn typed_return<'a>(resource: Option<&SdkResource>, method: &SdkMethod, t: &'a SdkType) -> ReturnModel<'a> {
    if let SdkType::Ref(r) = t {
        return ReturnModel {
            r#ref: r.clone(),
            inline: None,
        };
    }
    ReturnModel {
        r#ref: method_ref(resource, method, "Response"),
        inline: Some(t),
    }
}

/// The path parameters before the last, in path order: Stainless makes only the last one
/// positional and these required keyword arguments, first in the params `TypedDict` too
/// (honcho `messages.update(message_id, *, workspace_id, session_id)`, imagekit
/// `versions.delete(version_id, *, file_id)`, knock-mgmt `StepPreviewTemplateParams`).
#[must_use]
pub fn keyword_path_params(method: &SdkMethod) -> &[SdkParam] {
    &method.path_params[..method.path_params.len().saturating_sub(1)]
}

/// The keyword arguments of a method (query and body), in the oracle order.
#[must_use]
pub fn method_params(method: &SdkMethod) -> Vec<&SdkParam> {
    sort_fields(method.query_params.iter().chain(&method.body_params))
}

/// The header parameters of a method in the oracle order (ogx `models.list`, stagehand
/// `sessions.act`): by wire name, required first. They follow the other keyword arguments.
#[must_use]
pub fn header_params(method: &SdkMethod) -> Vec<&SdkParam> {
    sort_fields(&method.header_params)
}

/// Every keyword argument of a method: [`method_params`], then [`header_params`].
#[must_use]
pub fn method_args(method: &SdkMethod) -> Vec<&SdkParam> {
    let mut out = method_params(method);
    out.extend(header_params(method));
    out
}

/// Whether `q` is one of the method's header parameters.
#[must_use]
pub fn is_header(method: &SdkMethod, q: &SdkParam) -> bool {
    method.header_params.iter().any(|h| std::ptr::eq(h, q))
}

const EXTRA_ARGS: [(&str, &str); 4] = [
    ("extra_headers", "Send extra headers"),
    ("extra_query", "Add additional query parameters to the request"),
    ("extra_body", "Add additional JSON properties to the request"),
    (
        "timeout",
        "Override the client-level default timeout for this request, in seconds",
    ),
];

/// A method docstring: the description (else the summary), then, when the method takes any
/// parameter, an `Args:` section listing the documented keyword arguments and the extras.
#[must_use]
pub fn args_docstring(method: &SdkMethod, args: Option<&[&SdkParam]>) -> Vec<String> {
    let text = js::trim(if method.description.is_empty() {
        &method.summary
    } else {
        &method.description
    });
    let body = wrap(&escape_docstring(text), 80);
    let Some(args) = args else {
        return match body.len() {
            0 => Vec::new(),
            1 => vec![format!("\"\"\"{}\"\"\"", body[0])],
            _ => [vec!["\"\"\"".to_owned()], body, vec!["\"\"\"".to_owned()]].concat(),
        };
    };
    let mut all: Vec<(String, String)> = args
        .iter()
        .filter(|a| !js::trim(&a.description).is_empty())
        .map(|a| (a.py_name.clone(), escape_docstring(js::trim(&a.description))))
        .collect();
    all.extend(EXTRA_ARGS.iter().map(|&(n, d)| (n.to_owned(), d.to_owned())));
    let mut out = vec!["\"\"\"".to_owned()];
    let has_body = !body.is_empty();
    out.extend(body);
    if has_body {
        out.push(String::new());
    }
    out.push("Args:".to_owned());
    for (i, (name, desc)) in all.iter().enumerate() {
        let wrapped = wrap(desc, 80);
        if i > 0 {
            out.push(String::new());
        }
        out.push(format!("  {name}: {}", wrapped.first().map_or("", String::as_str)));
        out.extend(wrapped.iter().skip(1).map(|l| format!("      {l}")));
    }
    out.push("\"\"\"".to_owned());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SdkPrimitive;

    pub(crate) fn param(wire: &str, required: bool) -> SdkParam {
        SdkParam {
            wire_name: wire.into(),
            py_name: wire.into(),
            required,
            description: String::new(),
            ty: SdkType::Primitive(SdkPrimitive::String),
            deprecated: false,
            example: None,
            minimum: None,
            maximum: None,
        }
    }

    #[test]
    fn escapes_like_json_stringify() {
        assert_eq!(py_string("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
        assert_eq!(py_fstring("{x}"), "f\"{{x}}\"");
    }

    #[test]
    fn doubles_backslashes_as_stainless_does() {
        assert_eq!(escape_docstring("a \\* b"), "a \\\\* b");
        assert_eq!(escape_docstring("x \"\"\" y \""), "x \\\"\\\"\\\" y \\\"");
    }

    #[test]
    fn computes_relative_imports_and_wraps_text() {
        assert_eq!(rel_import("types.foo", "_models"), ".._models");
        assert_eq!(rel_import("types.shared.a", "types.shared.b"), ".b");
        assert_eq!(rel_import("resources.a", "types"), "..types");
        assert_eq!(wrap("", 80), Vec::<String>::new());
        assert_eq!(wrap("a b\n\nc", 3), ["a b", "", "c"]);
    }

    #[test]
    fn sorts_required_first_id_first_then_by_wire_name() {
        let fields = [
            param("b", true),
            param("z", false),
            param("id", true),
            param("A", true),
            param("a", false),
            param("a", false),
        ];
        let names: Vec<&str> = sort_fields(&fields).iter().map(|f| f.wire_name.as_str()).collect();
        assert_eq!(names, ["id", "A", "b", "a", "a", "z"]);
    }
}
