//! Neutral types -> Go types: the one place type mapping lives (ADR 0017, "Type mapping lives in
//! one function": [`map_type`]).
//!
//! Inline objects and enums become named declarations at their position ([`Position`]), in the
//! order Stainless declares them: a struct, then the types its fields introduce, depth first in
//! field order.

use std::collections::BTreeSet;

use ferrule_sdk_model::js;
use ferrule_sdk_model::model::{Client, Field, Literal, Object, Scalar, Type, TypeRef};

use crate::naming::{Casings, exported, singular, words};

/// Which side of the wire a type is on: responses decode into plain fields, params wrap every
/// field in `param.Field`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Response,
    /// Query parameters (`query:"..."` tags, `URLQuery`).
    Query,
    /// A JSON body (`json:"..."` tags, `MarshalJSON`).
    Body,
}

/// The Go package a declaration lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pkg {
    /// The module's root package (`onebusaway`).
    Root,
    /// `shared/`, for `$shared` models.
    Shared,
}

/// One struct field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoField {
    pub name: String,
    pub wire: String,
    /// The value type (`string`, `[]ReferencesAgency`); params wrap it in `param.Field`.
    pub ty: String,
    pub required: bool,
    pub nullable: bool,
    pub format: Option<&'static str>,
    pub doc: String,
    /// The Go literals of an enum-typed field, which v2 lists in its doc (`Any of "a", "b".`).
    pub enum_values: Vec<String>,
    /// The struct tag key: `json`, `query`, or `path` (v2's path params in a params struct).
    pub key: &'static str,
}

/// A declaration the emitters print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decl {
    Struct {
        name: String,
        doc: String,
        fields: Vec<GoField>,
        /// Embedded named models (`allOf` bases), qualified for the declaring package.
        bases: Vec<String>,
        mode: Mode,
    },
    Enum {
        name: String,
        doc: String,
        /// `string`, `int64`, `float64` or `bool`.
        underlying: &'static str,
        /// Constant names and Go literals, in spec order.
        members: Vec<(String, String)>,
    },
}

impl Decl {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Struct { name, .. } | Self::Enum { name, .. } => name,
        }
    }
}

/// Where an inline type sits: the name it takes, the name it takes as an array item (the last
/// word singular), and the prefix of the types inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub name: String,
    pub item_name: String,
    pub child_prefix: String,
    /// Fields from the type declared exactly (`0`): v2 declares a params enum only one level
    /// down (`VariableListParamsType`), deeper ones are plain strings.
    pub depth: usize,
    /// Appended to every struct declared here and below, not to enums: v2's param variant of a
    /// model (`MessageTypeBooleanFieldSettingsParam`, whose enums are the model's).
    pub suffix: String,
}

impl Position {
    /// A type named exactly `name`, inside and as an item.
    #[must_use]
    pub fn exact(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            item_name: name.to_owned(),
            child_prefix: name.to_owned(),
            depth: 0,
            suffix: String::new(),
        }
    }

    /// [`Position::exact`] for the structs of a model's param variant: `<name>...Param`.
    #[must_use]
    pub fn param_variant(name: &str) -> Self {
        Self {
            suffix: "Param".to_owned(),
            ..Self::exact(name)
        }
    }

    /// The name of a struct declared here.
    #[must_use]
    pub fn struct_name(&self) -> String {
        format!("{}{}", self.name, self.suffix)
    }

    /// The position of a field with words `field` inside a type with this child prefix.
    #[must_use]
    pub fn field(&self, field: &[String], casings: Casings<'_>) -> Self {
        let plain = format!("{}{}", self.child_prefix, exported(field, casings));
        Self {
            item_name: format!("{}{}", self.child_prefix, exported(&singular(field), casings)),
            child_prefix: plain.clone(),
            name: plain,
            depth: self.depth + 1,
            suffix: self.suffix.clone(),
        }
    }

    fn as_item(&self) -> Self {
        Self {
            name: self.item_name.clone(),
            item_name: self.item_name.clone(),
            child_prefix: self.child_prefix.clone(),
            depth: self.depth,
            suffix: self.suffix.clone(),
        }
    }
}

/// What mapping a type needs from around it.
pub struct Ctx<'a> {
    pub client: &'a Client,
    pub casings: Casings<'a>,
    pub module: &'a str,
    /// The package being emitted, to qualify references into the other one.
    pub pkg: Pkg,
    pub imports: BTreeSet<String>,
    /// Stainless's Go v2 layout (`param.Opt`, `respjson`).
    pub v2: bool,
}

impl Ctx<'_> {
    pub fn import(&mut self, path: &str) {
        self.imports.insert(path.to_owned());
    }

    /// `module/<rel>`, imported.
    pub fn import_module(&mut self, rel: &str) {
        let path = format!("{}/{rel}", self.module);
        self.imports.insert(path);
    }
}

/// The result of mapping one type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapped {
    pub ty: String,
    pub format: Option<&'static str>,
    pub nullable: bool,
    pub decls: Vec<Decl>,
}

impl Mapped {
    fn plain(ty: &str) -> Self {
        Self {
            ty: ty.to_owned(),
            format: None,
            nullable: false,
            decls: Vec::new(),
        }
    }
}

/// The package a named model is declared in.
#[must_use]
pub fn model_pkg(r: &TypeRef) -> Pkg {
    if r.path.first().is_some_and(|p| p.written == "shared") {
        Pkg::Shared
    } else {
        Pkg::Root
    }
}

/// The Go name of a named model: its config name in Go's casing (`UrlEndpointRequest` ->
/// `URLEndpointRequest`, imagekit-go).
#[must_use]
pub fn model_name(r: &TypeRef, casings: Casings<'_>) -> String {
    exported(&crate::naming::split_words(&r.name.written), casings)
}

/// Whether v2 declares a param variant of the model: an object with fields.
#[must_use]
pub fn has_param_variant(client: &Client, r: &TypeRef) -> bool {
    client
        .model(r)
        .is_some_and(|d| matches!(&d.ty, Type::Object(o) if !(o.fields.is_empty() && o.bases.is_empty())))
}

/// The Go name of a named model, qualified for `ctx.pkg`.
pub fn model_ref(r: &TypeRef, ctx: &mut Ctx<'_>) -> String {
    model_ref_in(r, Mode::Response, ctx)
}

/// The Go name of a named model in `mode`: in v2 params, its param variant (`DurationParam`).
pub fn model_ref_in(r: &TypeRef, mode: Mode, ctx: &mut Ctx<'_>) -> String {
    let mut name = model_name(r, ctx.casings);
    if ctx.v2 && mode != Mode::Response && has_param_variant(ctx.client, r) {
        name.push_str("Param");
    }
    match (model_pkg(r), ctx.pkg) {
        (Pkg::Shared, Pkg::Root) => {
            ctx.import_module("shared");
            format!("shared.{name}")
        }
        _ => name,
    }
}

/// Any value: `interface{}`, which v2 spells `any`.
fn any(ctx: &Ctx<'_>) -> Mapped {
    Mapped::plain(if ctx.v2 { "any" } else { "interface{}" })
}

fn scalar(s: Scalar, mode: Mode, ctx: &mut Ctx<'_>) -> Mapped {
    if matches!(s, Scalar::Unknown | Scalar::Null) {
        return any(ctx);
    }
    let mut m = Mapped::plain(match s {
        Scalar::Boolean => "bool",
        Scalar::String | Scalar::Uuid | Scalar::Bytes => "string",
        s if s.is_integer() => "int64",
        Scalar::Float | Scalar::Double => "float64",
        Scalar::Date | Scalar::DateTime => "time.Time",
        Scalar::Binary if mode != Mode::Response => "io.Reader",
        _ => "string",
    });
    m.format = match s {
        Scalar::Uuid => Some("uuid"),
        Scalar::Date => Some("date"),
        Scalar::DateTime => Some("date-time"),
        Scalar::Binary if mode != Mode::Response => Some("binary"),
        _ => None,
    };
    match m.ty.as_str() {
        "time.Time" => ctx.import("time"),
        "io.Reader" => ctx.import("io"),
        _ => {}
    }
    m
}

/// The values of an enum type (inline, nullable or a named enum model), as Go literals.
fn enum_values(ty: &Type, client: &Client) -> Vec<String> {
    match ty {
        Type::Enum { members } => members.iter().map(|m| literal(&m.value).1).collect(),
        Type::Nullable { inner } => enum_values(inner, client),
        Type::Ref(r) => client.model(r).map_or_else(Vec::new, |d| enum_values(&d.ty, client)),
        _ => Vec::new(),
    }
}

fn literal(value: &Literal) -> (&'static str, String) {
    match value {
        Literal::String(s) => ("string", crate::printer::quote(s)),
        Literal::Boolean(b) => ("bool", b.to_string()),
        Literal::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => ("int64", format!("{n:.0}")),
        Literal::Number(n) => ("float64", n.to_string()),
    }
}

/// Field order in the oracle SDKs: required before optional; within each, `id` first, then by
/// wire name ignoring case.
#[must_use]
pub fn sort_fields(fields: &[Field]) -> Vec<&Field> {
    let mut out: Vec<&Field> = fields.iter().collect();
    out.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| (b.wire_name == "id").cmp(&(a.wire_name == "id")))
            .then_with(|| js::cmp(&a.wire_name.to_lowercase(), &b.wire_name.to_lowercase()))
    });
    out
}

/// Whether a Go type is one v2's `param.Opt` wraps when optional (`string`, `int64`, ...).
#[must_use]
pub fn opt_scalar(ty: &str) -> bool {
    matches!(ty, "string" | "int64" | "float64" | "bool" | "time.Time")
}

/// v2 params order: required fields before optional ones; within each, nullable scalars,
/// then the other scalars, then the rest (imagekit-go, knock-mgmt-go:
/// `MessageTypeBooleanFieldParam` is `Label` (nullable), `Key`, `Type`), each group in
/// [`sort_fields`] order.
fn v2_param_rank(f: &GoField) -> u8 {
    let group = if f.required { 0 } else { 3 };
    let kind = if opt_scalar(&f.ty) && f.enum_values.is_empty() {
        u8::from(!f.nullable)
    } else {
        2
    };
    group + kind
}

/// The tag key of a field mapped in `mode`.
fn key(mode: Mode) -> &'static str {
    if mode == Mode::Query { "query" } else { "json" }
}

/// The fields of `fields` (in [`sort_fields`] order) mapped at `pos`, their declarations
/// appended to `nested`.
fn go_fields(fields: &[Field], pos: &Position, mode: Mode, ctx: &mut Ctx<'_>, nested: &mut Vec<Decl>) -> Vec<GoField> {
    let mut out = Vec::new();
    for f in sort_fields(fields) {
        let fpos = pos.field(&words(&f.name), ctx.casings);
        let mapped = map_type(&f.ty, &fpos, &f.description, mode, ctx);
        nested.extend(mapped.decls);
        out.push(GoField {
            name: exported(&words(&f.name), ctx.casings),
            wire: f.wire_name.clone(),
            ty: mapped.ty,
            required: f.required,
            nullable: mapped.nullable,
            format: mapped.format,
            doc: f.description.clone(),
            enum_values: enum_values(&f.ty, ctx.client),
            key: key(mode),
        });
    }
    out
}

/// A method's params struct named `name`: v2's path fields (`path:"..."`), then the query
/// params, then the body fields, each group in [`sort_fields`] order; v2 then moves required
/// fields first and optional scalars before the other optional fields (knock-mgmt-go's
/// `WorkflowStepPreviewTemplateParams`). The struct is `Mode::Body` when it has body fields.
pub fn params_decls(path: &[Field], query: &[Field], body: &[Field], name: &str, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let pos = Position::exact(name);
    let mut nested = Vec::new();
    let mut fields: Vec<GoField> = path
        .iter()
        .map(|f| GoField {
            name: exported(&words(&f.name), ctx.casings),
            wire: f.wire_name.clone(),
            ty: crate::emit::path_param_type(&f.ty).to_owned(),
            required: true,
            nullable: false,
            format: None,
            // imagekit-go's `FileVersionDeleteParams.FileID` is undocumented.
            doc: String::new(),
            enum_values: Vec::new(),
            key: "path",
        })
        .collect();
    fields.extend(go_fields(query, &pos, Mode::Query, ctx, &mut nested));
    fields.extend(go_fields(body, &pos, Mode::Body, ctx, &mut nested));
    if ctx.v2 {
        fields.sort_by_key(|f| if f.key == "path" { 0 } else { 1 + v2_param_rank(f) });
    }
    let mode = if body.is_empty() { Mode::Query } else { Mode::Body };
    let mut out = vec![Decl::Struct {
        name: name.to_owned(),
        doc: String::new(),
        fields,
        bases: Vec::new(),
        mode,
    }];
    out.extend(nested);
    out
}

/// The struct for an object at `pos`, followed by the types its fields introduce.
pub fn object_decls(obj: &Object, pos: &Position, doc: &str, mode: Mode, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let mut nested = Vec::new();
    let mut fields = go_fields(&obj.fields, pos, mode, ctx, &mut nested);
    let mut doc = doc.trim_end().to_owned();
    if ctx.v2 && mode != Mode::Response {
        fields.sort_by_key(v2_param_rank);
        // v2 names a param object's required properties in its doc (knock-mgmt-go).
        let mut required: Vec<&str> = fields.iter().filter(|f| f.required).map(|f| f.name.as_str()).collect();
        required.sort_unstable();
        let line = match required.as_slice() {
            [] => String::new(),
            [one] => format!("The property {one} is required."),
            many => format!("The properties {} are required.", many.join(", ")),
        };
        if !line.is_empty() {
            doc = if doc.is_empty() {
                line
            } else {
                format!("{doc}\n\n{line}")
            };
        }
    }
    let bases = obj.bases.iter().map(|b| model_ref(b, ctx)).collect();
    let mut out = vec![Decl::Struct {
        name: pos.struct_name(),
        doc,
        fields,
        bases,
        mode,
    }];
    out.extend(nested);
    out
}

/// Maps `ty` at `pos` (`doc` is the description it was written under, for inline declarations).
pub fn map_type(ty: &Type, pos: &Position, doc: &str, mode: Mode, ctx: &mut Ctx<'_>) -> Mapped {
    match ty {
        Type::Scalar { scalar: s } => scalar(*s, mode, ctx),
        Type::Enum { members } => {
            let underlying = members.first().map_or("string", |m| literal(&m.value).0);
            // v2 params declare an enum only for a field of the params struct itself
            // (`VariableListParamsType`); deeper ones take the underlying type, listing the
            // values in the field's doc (knock-mgmt-go's `Operator string`).
            if ctx.v2 && mode != Mode::Response && pos.depth > 1 {
                return Mapped::plain(underlying);
            }
            // v2 spells an upper-case value's words in title case (`COPY_FOLDER` ->
            // `CopyFolder`, imagekit-go).
            let consts = members
                .iter()
                .map(|m| {
                    let mut w = words(&m.name);
                    if ctx.v2 {
                        w = w.iter().map(|s| s.to_lowercase()).collect();
                    }
                    let suffix = exported(&w, ctx.casings);
                    (format!("{}{suffix}", pos.name), literal(&m.value).1)
                })
                .collect();
            Mapped {
                ty: pos.name.clone(),
                format: None,
                nullable: false,
                decls: vec![Decl::Enum {
                    name: pos.name.clone(),
                    doc: doc.to_owned(),
                    underlying,
                    members: consts,
                }],
            }
        }
        Type::Array { items } => {
            let inner = map_type(items, &pos.as_item(), doc, mode, ctx);
            Mapped {
                ty: format!("[]{}", inner.ty),
                ..inner
            }
        }
        Type::Map { values } => {
            let inner = map_type(values, pos, doc, mode, ctx);
            Mapped {
                ty: format!("map[string]{}", inner.ty),
                ..inner
            }
        }
        Type::Object(o) if o.fields.is_empty() && o.bases.is_empty() => any(ctx),
        Type::Object(o) => Mapped {
            ty: pos.struct_name(),
            format: None,
            nullable: false,
            decls: object_decls(o, pos, doc, mode, ctx),
        },
        // Stainless's Go v1 unions (interface types with registered variants) are not emitted
        // yet; any value decodes.
        Type::Union(_) => any(ctx),
        Type::Ref(r) => Mapped::plain(&model_ref_in(r, mode, ctx)),
        Type::Nullable { inner } => {
            let m = map_type(inner, pos, doc, mode, ctx);
            Mapped { nullable: true, ..m }
        }
    }
}
