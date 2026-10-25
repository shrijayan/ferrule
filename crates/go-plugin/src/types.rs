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
}

impl Position {
    /// A type named exactly `name`, inside and as an item.
    #[must_use]
    pub fn exact(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            item_name: name.to_owned(),
            child_prefix: name.to_owned(),
        }
    }

    /// The position of a field with words `field` inside a type with this child prefix.
    #[must_use]
    pub fn field(&self, field: &[String], casings: Casings<'_>) -> Self {
        let plain = format!("{}{}", self.child_prefix, exported(field, casings));
        Self {
            item_name: format!("{}{}", self.child_prefix, exported(&singular(field), casings)),
            child_prefix: plain.clone(),
            name: plain,
        }
    }

    fn as_item(&self) -> Self {
        Self {
            name: self.item_name.clone(),
            item_name: self.item_name.clone(),
            child_prefix: self.child_prefix.clone(),
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

/// The Go name of a named model, qualified for `ctx.pkg`.
pub fn model_ref(r: &TypeRef, ctx: &mut Ctx<'_>) -> String {
    let name = r.name.written.clone();
    match (model_pkg(r), ctx.pkg) {
        (Pkg::Shared, Pkg::Root) => {
            ctx.import_module("shared");
            format!("shared.{name}")
        }
        _ => name,
    }
}

fn scalar(s: Scalar, mode: Mode, ctx: &mut Ctx<'_>) -> Mapped {
    let mut m = Mapped::plain(match s {
        Scalar::Unknown | Scalar::Null => "interface{}",
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

/// The struct for an object at `pos`, followed by the types its fields introduce.
pub fn object_decls(obj: &Object, pos: &Position, doc: &str, mode: Mode, ctx: &mut Ctx<'_>) -> Vec<Decl> {
    let mut nested = Vec::new();
    let mut fields = Vec::new();
    for f in sort_fields(&obj.fields) {
        let fpos = pos.field(&words(&f.name), ctx.casings);
        let mapped = map_type(&f.ty, &fpos, &f.description, mode, ctx);
        nested.extend(mapped.decls);
        fields.push(GoField {
            name: exported(&words(&f.name), ctx.casings),
            wire: f.wire_name.clone(),
            ty: mapped.ty,
            required: f.required,
            nullable: mapped.nullable,
            format: mapped.format,
            doc: f.description.clone(),
        });
    }
    let bases = obj.bases.iter().map(|b| model_ref(b, ctx)).collect();
    let mut out = vec![Decl::Struct {
        name: pos.name.clone(),
        doc: doc.to_owned(),
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
            let consts = members
                .iter()
                .map(|m| {
                    let suffix = exported(&words(&m.name), ctx.casings);
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
        Type::Object(o) if o.fields.is_empty() && o.bases.is_empty() => Mapped::plain("interface{}"),
        Type::Object(o) => Mapped {
            ty: pos.name.clone(),
            format: None,
            nullable: false,
            decls: object_decls(o, pos, doc, mode, ctx),
        },
        // Stainless's Go v1 unions (interface types with registered variants) are not emitted
        // yet; any value decodes.
        Type::Union(_) => Mapped::plain("interface{}"),
        Type::Ref(r) => Mapped::plain(&model_ref(r, ctx)),
        Type::Nullable { inner } => {
            let m = map_type(inner, pos, doc, mode, ctx);
            Mapped { nullable: true, ..m }
        }
    }
}
