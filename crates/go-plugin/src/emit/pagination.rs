//! `packages/pagination/pagination.go`: one generic page type per `pagination` scheme the
//! methods use, with its auto-pager, and what the resource emitters need to return pages.
//!
//! Text from Stainless's Go v1 page types (knock-go, Apache-2.0; see NOTICE). Only the `cursor`
//! scheme is emitted: it is the one the Go oracles use. A method paginated with another scheme
//! is emitted as an ordinary method returning its response (Phase 1 gap list).
//!
//! Page fields come from `x-ferrule-pagination`'s response map in its order, which the converter
//! writes sorted rather than in stainless.yml's order (Phase 1 gap list). A field's type is the
//! type of the same path in the method's response, `string` when the response has no such path.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Method, PaginationScheme, ParamLocation, Purpose, Scalar, Type};

use super::{Gen, method_order};
use crate::naming::{exported, split_words, unexported};
use crate::printer::{ALIGN, comment, go_file, quote};
use crate::types::{GoField, Mode};

/// One field of a page type or of an object inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageField {
    pub name: String,
    pub wire: String,
    /// `[]T` for the items, the struct name for an object, else a Go scalar type.
    pub ty: String,
    /// The fields of an object (`page_info`), declared as `<Page><Field>`.
    pub fields: Vec<PageField>,
}

/// A page type, such as `EntriesCursor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The scheme's name (`entries_cursor`).
    pub scheme: String,
    /// `EntriesCursor`.
    pub name: String,
    pub fields: Vec<PageField>,
    /// The Go name of the items field (`Entries`).
    pub items: String,
    /// The Go path of the next cursor (`PageInfo.After`).
    pub next: String,
    /// The request parameter the next cursor is sent as (`after`).
    pub param: String,
    pub location: ParamLocation,
}

/// The Go type of a page scalar.
fn scalar(s: Scalar) -> &'static str {
    match s {
        Scalar::Boolean => "bool",
        s if s.is_integer() => "int64",
        Scalar::Float | Scalar::Double => "float64",
        _ => "string",
    }
}

/// The type at wire path `path` inside the response `ty`.
fn type_at<'a>(g: &'a Gen<'_>, ty: &'a Type, path: &[&str]) -> Option<&'a Type> {
    let Some((first, rest)) = path.split_first() else {
        return Some(ty);
    };
    let ty = match ty {
        Type::Nullable { inner } => inner,
        t => t,
    };
    let obj = g.client.object_of(ty)?;
    let field = obj.fields.iter().find(|f| f.wire_name == *first)?;
    type_at(g, &field.ty, rest)
}

fn leaf_type(g: &Gen<'_>, response: Option<&Type>, path: &[&str]) -> String {
    let ty = match response.and_then(|r| type_at(g, r, path)) {
        Some(Type::Nullable { inner }) => Some(&**inner),
        t => t,
    };
    match ty {
        Some(Type::Scalar { scalar: s }) => scalar(*s).to_owned(),
        _ => "string".to_owned(),
    }
}

/// Inserts the field at `path` (wire segments) into `fields`, creating the objects above it,
/// each named `<parent><Field>`.
fn insert(g: &Gen<'_>, parent: &str, fields: &mut Vec<PageField>, path: &[&str], ty: String) {
    let Some((first, rest)) = path.split_first() else {
        return;
    };
    let name = exported(&split_words(first), g.casings);
    let index = fields.iter().position(|f| f.wire == *first).unwrap_or_else(|| {
        fields.push(PageField {
            name: name.clone(),
            wire: (*first).to_owned(),
            ty: if rest.is_empty() {
                ty.clone()
            } else {
                format!("{parent}{name}")
            },
            fields: Vec::new(),
        });
        fields.len() - 1
    });
    if !rest.is_empty() {
        insert(g, &format!("{parent}{name}"), &mut fields[index].fields, rest, ty);
    }
}

fn purpose<'a>(list: &'a [Purpose], purpose: &str) -> Option<&'a str> {
    list.iter().find(|p| p.purpose == purpose).map(|p| p.field.as_str())
}

/// The page type of a method, if it is paginated with a scheme the plugin emits.
#[must_use]
pub fn page_of(g: &Gen<'_>, m: &Method) -> Option<Page> {
    let p = m.pagination.as_ref()?;
    if p.scheme != PaginationScheme::Cursor || p.response.iter().any(|r| r.field.contains("[]")) {
        return None;
    }
    let items = purpose(&p.response, "items")?;
    let next = purpose(&p.response, "next_cursor_field")?;
    let param = purpose(&p.request, "next_cursor_param")?;
    let name = exported(&p.page.words, g.casings);
    let response = m.response.as_ref().map(|r| &r.ty);
    let mut fields = Vec::new();
    for r in &p.response {
        let path: Vec<&str> = r.field.split('.').collect();
        let ty = if r.field == items {
            "[]T".to_owned()
        } else {
            leaf_type(g, response, &path)
        };
        insert(g, &name, &mut fields, &path, ty);
    }
    let go_path = |wire: &str| {
        wire.split('.')
            .map(|s| exported(&split_words(s), g.casings))
            .collect::<Vec<_>>()
            .join(".")
    };
    Some(Page {
        scheme: p.page.written.clone(),
        name,
        items: go_path(items),
        next: go_path(next),
        param: param.to_owned(),
        location: p.param_location,
        fields,
    })
}

/// Every page type the methods use, in the order of first use.
#[must_use]
pub fn pages(g: &Gen<'_>) -> Vec<Page> {
    let mut out: Vec<Page> = Vec::new();
    for s in &g.services {
        for m in method_order(&s.resource.methods) {
            if let Some(page) = page_of(g, m)
                && !out.iter().any(|p| p.scheme == page.scheme)
            {
                out.push(page);
            }
        }
    }
    out
}

/// A response struct with its JSON metadata, as the resource emitters print one.
fn object(out: &mut String, name: &str, fields: &[PageField], ctx: &mut crate::types::Ctx<'_>) {
    for f in fields.iter().filter(|f| !f.fields.is_empty()) {
        object(out, &f.ty, &f.fields, ctx);
    }
    let go_fields: Vec<GoField> = fields
        .iter()
        .map(|f| GoField {
            name: f.name.clone(),
            wire: f.wire.clone(),
            ty: f.ty.clone(),
            required: false,
            nullable: false,
            format: None,
            doc: String::new(),
        })
        .collect();
    super::decls::print_decls(
        out,
        &[crate::types::Decl::Struct {
            name: name.to_owned(),
            doc: String::new(),
            fields: go_fields,
            bases: Vec::new(),
            mode: Mode::Response,
        }],
        ctx,
    );
}

/// The page type and its auto-pager.
fn page_decl(out: &mut String, p: &Page, ctx: &mut crate::types::Ctx<'_>) {
    for f in p.fields.iter().filter(|f| !f.fields.is_empty()) {
        object(out, &f.ty, &f.fields, ctx);
    }
    let name = &p.name;
    let generic = format!("{name}[T]");
    let json = format!("{}JSON", unexported(name));
    let _ = writeln!(out, "type {name}[T any] struct {{");
    for f in &p.fields {
        let _ = writeln!(out, "\t{}{ALIGN}{}{ALIGN}`json:\"{}\"`", f.name, f.ty, f.wire);
    }
    let _ = writeln!(out, "\tJSON{ALIGN}{json}{ALIGN}`json:\"-\"`");
    let _ = writeln!(out, "\tcfg{ALIGN}*requestconfig.RequestConfig");
    let _ = writeln!(out, "\tres{ALIGN}*http.Response");
    out.push_str("}\n\n");
    comment(
        out,
        0,
        &format!("{json} contains the JSON metadata for the struct [{generic}]"),
    );
    let _ = writeln!(out, "type {json} struct {{");
    for f in &p.fields {
        let _ = writeln!(out, "\t{}{ALIGN}apijson.Field", f.name);
    }
    let _ = writeln!(out, "\traw{ALIGN}string");
    let _ = writeln!(out, "\tExtraFields{ALIGN}map[string]apijson.Field");
    out.push_str("}\n\n");
    let _ = write!(
        out,
        "func (r *{generic}) UnmarshalJSON(data []byte) (err error) {{\n\treturn apijson.UnmarshalRoot(data, r)\n}}\n\n\
         func (r {json}) RawJSON() string {{\n\treturn r.raw\n}}\n\n"
    );
    let apply = match p.location {
        ParamLocation::Query => format!("option.WithQuery({}, next)", quote(&p.param)),
        // Unverified: no Go oracle sends its cursor in the body.
        ParamLocation::Body => format!("option.WithJSONSet({}, next)", quote(&p.param)),
    };
    comment(
        out,
        0,
        "GetNextPage returns the next page as defined by this pagination style. When there is no next page, \
         this function will return a 'nil' for the page value, but will not return an error",
    );
    let _ = write!(
        out,
        "func (r *{generic}) GetNextPage() (res *{generic}, err error) {{\n\
         \tnext := r.{next}\n\tif len(next) == 0 {{\n\t\treturn nil, nil\n\t}}\n\
         \tcfg := r.cfg.Clone(r.cfg.Context)\n\terr = cfg.Apply({apply})\n\tif err != nil {{\n\t\treturn nil, err\n\t}}\n\
         \tvar raw *http.Response\n\tcfg.ResponseInto = &raw\n\tcfg.ResponseBodyInto = &res\n\terr = cfg.Execute()\n\
         \tif err != nil {{\n\t\treturn nil, err\n\t}}\n\tres.SetPageConfig(cfg, raw)\n\treturn res, nil\n}}\n\n\
         func (r *{generic}) SetPageConfig(cfg *requestconfig.RequestConfig, res *http.Response) {{\n\
         \tif r == nil {{\n\t\tr = &{generic}{{}}\n\t}}\n\tr.cfg = cfg\n\tr.res = res\n}}\n\n",
        next = p.next,
    );
    let pager = format!("{name}AutoPager");
    let items = &p.items;
    let _ = write!(
        out,
        "type {pager}[T any] struct {{\n\tpage{ALIGN}*{generic}\n\tcur{ALIGN}T\n\tidx{ALIGN}int\n\trun{ALIGN}int\n\terr{ALIGN}error\n}}\n\n\
         func New{pager}[T any](page *{generic}, err error) *{pager}[T] {{\n\treturn &{pager}[T]{{\n\t\tpage:{ALIGN}page,\n\t\terr:{ALIGN}err,\n\t}}\n}}\n\n\
         func (r *{pager}[T]) Next() bool {{\n\tif r.page == nil || len(r.page.{items}) == 0 {{\n\t\treturn false\n\t}}\n\
         \tif r.idx >= len(r.page.{items}) {{\n\t\tr.idx = 0\n\t\tr.page, r.err = r.page.GetNextPage()\n\
         \t\tif r.err != nil || r.page == nil || len(r.page.{items}) == 0 {{\n\t\t\treturn false\n\t\t}}\n\t}}\n\
         \tr.cur = r.page.{items}[r.idx]\n\tr.run += 1\n\tr.idx += 1\n\treturn true\n}}\n\n\
         func (r *{pager}[T]) Current() T {{\n\treturn r.cur\n}}\n\n\
         func (r *{pager}[T]) Err() error {{\n\treturn r.err\n}}\n\n\
         func (r *{pager}[T]) Index() int {{\n\treturn r.run\n}}\n\n"
    );
}

/// `packages/pagination/pagination.go`, if any method is paginated.
#[must_use]
pub fn pagination_file(g: &Gen<'_>) -> Option<(String, String)> {
    let pages = pages(g);
    if pages.is_empty() {
        return None;
    }
    let mut ctx = g.ctx(crate::types::Pkg::Root);
    ctx.import("net/http");
    ctx.import_module("internal/apijson");
    ctx.import_module("internal/requestconfig");
    ctx.import_module("option");
    let mut body = String::new();
    for p in &pages {
        page_decl(&mut body, p, &mut ctx);
    }
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some((
        "packages/pagination/pagination.go".to_owned(),
        go_file("pagination", &imports, &body),
    ))
}
