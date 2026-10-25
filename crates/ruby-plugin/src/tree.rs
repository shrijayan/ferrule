//! The gem's declarations: every model class, enum and union module, nested as Stainless nests
//! them, built from the neutral model. The three views of each file print from these (see
//! `emit::model`), so the shape is decided once.

use ferrule_sdk_model::model::{Client, Field, Literal, Method, ModelDecl, Object, Resource, Type, TypeRef, Union};

use crate::naming::{Casings, class_name, constant, singular, snake, split_words, words};
use crate::types::{ClassRef, RType};

/// A field of a model or a parameter of a method.
#[derive(Debug, Clone, PartialEq)]
pub struct RField {
    /// `readable_time`.
    pub name: String,
    /// `readableTime`.
    pub wire: String,
    pub required: bool,
    /// The value may be nil (`nil?: true`).
    pub nullable: bool,
    pub ty: RType,
    pub description: String,
    pub deprecated: bool,
}

impl RField {
    /// `api_name: :readableTime` when the wire name is not the Ruby name.
    #[must_use]
    pub fn api_name(&self) -> Option<&str> {
        (self.wire != self.name).then_some(self.wire.as_str())
    }
}

/// What a `# @see` line points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum See {
    /// The method that returns or takes it: `Resources::<path>#<method>`.
    Method { resource: Vec<String>, method: String },
    /// The attribute it is the type of: `<parent>#<field>`.
    Field { parent: ClassRef, field: String },
}

impl See {
    #[must_use]
    pub fn render(&self, module: &str) -> String {
        match self {
            Self::Method { resource, method } => format!("{module}::Resources::{}#{method}", resource.join("::")),
            Self::Field { parent, field } => format!("{}#{field}", parent.doc(module)),
        }
    }
}

/// A declaration in a model file.
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Class(Class),
    Enum(EnumDecl),
    Union(UnionDecl),
}

impl Decl {
    #[must_use]
    pub fn r(&self) -> &ClassRef {
        match self {
            Self::Class(c) => &c.r,
            Self::Enum(e) => &e.r,
            Self::Union(u) => &u.r,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub r: ClassRef,
    /// The model it extends; `None` for `Internal::Type::BaseModel`.
    pub base: Option<ClassRef>,
    pub description: String,
    /// `# @see` target: the method or attribute it is the type of.
    pub see: Option<See>,
    pub fields: Vec<RField>,
    /// Classes, enums and unions declared inside, in field order.
    pub nested: Vec<Decl>,
    /// A method's parameters (`RequestParameters`).
    pub params: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMember {
    /// `STOP_NAME_WRONG`.
    pub constant: String,
    pub value: Literal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub r: ClassRef,
    pub description: String,
    pub see: Option<See>,
    pub members: Vec<EnumMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub ty: RType,
    pub nullable: bool,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionDecl {
    pub r: ClassRef,
    pub description: String,
    pub see: Option<See>,
    pub variants: Vec<Variant>,
    pub nested: Vec<Decl>,
}

/// One file under `models/`: its path from `models/` without extension and its declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFile {
    /// `current_time_retrieve_response`, `references`.
    pub path: String,
    pub decl: Decl,
}

/// Field order in the oracle SDKs: required before optional; within each, `id` first, then by
/// wire name ignoring case (the Python plugin's `sort_fields`).
#[must_use]
pub fn sort_fields(fields: &[Field]) -> Vec<&Field> {
    let mut out: Vec<&Field> = fields.iter().collect();
    out.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| (b.wire_name == "id").cmp(&(a.wire_name == "id")))
            .then_with(|| ferrule_sdk_model::js::cmp(&a.wire_name.to_lowercase(), &b.wire_name.to_lowercase()))
    });
    out
}

/// Builds declarations for one SDK.
pub struct Builder<'a> {
    pub client: &'a Client,
    pub casings: Casings<'a>,
}

/// What a type is named after: the field's words (array items singular), its wire name (for
/// `@see`) and its description (for nested enums and unions).
#[derive(Clone, Copy)]
struct At<'n> {
    name: &'n [String],
    wire: &'n str,
    doc: &'n str,
}

impl<'n> At<'n> {
    fn plain(name: &'n [String]) -> Self {
        Self {
            name,
            wire: "",
            doc: "",
        }
    }
}

/// Where a type is declared, for naming what it nests.
struct Site<'p> {
    parent: &'p ClassRef,
    /// The parent, for `@see`, when it gives its nested types one.
    see_parent: Option<ClassRef>,
}

impl<'a> Builder<'a> {
    #[must_use]
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            casings: Casings(&client.casings),
        }
    }

    /// The class of a named model: `References` (aliased at the top of the module).
    #[must_use]
    pub fn model_ref(&self, r: &TypeRef) -> ClassRef {
        let name = if r.name.written.is_empty() {
            class_name(&r.name.words, self.casings)
        } else {
            r.name.written.clone()
        };
        ClassRef {
            segs: vec![name],
            aliased: true,
        }
    }

    /// A field's Ruby type; nested declarations go to `nested`. `name` is the field's words, from
    /// which a nested declaration is named (singular for array items).
    fn field_type(&self, site: &Site<'_>, at: At<'_>, ty: &Type, nested: &mut Vec<Decl>) -> (RType, bool) {
        match ty {
            Type::Nullable { inner } => (self.ty(site, at, inner, nested, true), true),
            _ => (self.ty(site, at, ty, nested, true), false),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn ty(&self, site: &Site<'_>, at: At<'_>, ty: &Type, nested: &mut Vec<Decl>, direct: bool) -> RType {
        use ferrule_sdk_model::model::Scalar as S;
        let At { name, wire, doc } = at;
        let see = || {
            if direct {
                site.see_parent.as_ref().map(|p| See::Field {
                    parent: p.clone(),
                    field: snake(&split_words(wire)),
                })
            } else {
                None
            }
        };
        match ty {
            Type::Scalar { scalar } => match scalar {
                S::String | S::Uuid | S::Bytes => RType::String,
                S::Boolean => RType::Boolean,
                S::Float | S::Double => RType::Float,
                S::DateTime => RType::Time,
                S::Date => RType::Date,
                S::Binary => RType::File,
                S::Unknown | S::Null => RType::Unknown,
                s if s.is_integer() => RType::Integer,
                _ => RType::Unknown,
            },
            Type::Nullable { inner } => RType::Nullable(Box::new(self.ty(site, at, inner, nested, false))),
            Type::Array { items } => {
                let item = singular(name);
                RType::Array(Box::new(self.ty(site, At { name: &item, ..at }, items, nested, false)))
            }
            Type::Map { values } => RType::Map(Box::new(self.ty(site, at, values, nested, false))),
            Type::Ref(r) => {
                let c = self.model_ref(r);
                match self.client.model(r).map(|d| &d.ty) {
                    Some(Type::Enum { .. }) => RType::Enum(c),
                    Some(Type::Union(_)) => RType::Union(c),
                    _ => RType::Model(c),
                }
            }
            Type::Object(o) => {
                let r = site.parent.child(&class_name(name, self.casings));
                let class = self.class(r.clone(), o, see(), false);
                nested.push(Decl::Class(class));
                RType::Model(r)
            }
            Type::Enum { members } => {
                let r = site.parent.child(&class_name(name, self.casings));
                let e = enum_decl(r.clone(), members, doc.to_owned(), see());
                nested.push(Decl::Enum(e));
                RType::Enum(r)
            }
            Type::Union(u) => {
                let r = site.parent.child(&class_name(name, self.casings));
                let decl = self.union(r.clone(), u, doc.to_owned(), see());
                nested.push(Decl::Union(decl));
                RType::Union(r)
            }
        }
    }

    /// The class of an object: its fields in Stainless's order and what they nest.
    #[must_use]
    pub fn class(&self, r: ClassRef, o: &Object, see: Option<See>, params: bool) -> Class {
        let base = o.bases.first().map(|b| self.model_ref(b));
        let see_parent = (!params && base.is_none()).then(|| r.clone());
        let site = Site { parent: &r, see_parent };
        let mut nested = Vec::new();
        let fields = sort_fields(&o.fields)
            .into_iter()
            .map(|f| self.field(&site, f, &mut nested))
            .collect();
        Class {
            base,
            description: o.description.clone().unwrap_or_default(),
            see,
            fields,
            nested,
            params,
            r,
        }
    }

    fn field(&self, site: &Site<'_>, f: &Field, nested: &mut Vec<Decl>) -> RField {
        let name = words(&f.name);
        let at = At {
            name: &name,
            wire: &f.wire_name,
            doc: &f.description,
        };
        let (ty, nullable) = self.field_type(site, at, &f.ty, nested);
        RField {
            name: snake(&name),
            wire: f.wire_name.clone(),
            required: f.required,
            nullable,
            ty,
            description: f.description.clone(),
            deprecated: f.deprecated,
        }
    }

    fn union(&self, r: ClassRef, u: &Union, description: String, see: Option<See>) -> UnionDecl {
        let site = Site {
            parent: &r,
            see_parent: None,
        };
        let mut nested = Vec::new();
        let variants = u
            .variants
            .iter()
            .map(|v| {
                let name = v.name.as_ref().map_or_else(Vec::new, words);
                let (ty, nullable) = match &v.ty {
                    Type::Nullable { inner } => (self.ty(&site, At::plain(&name), inner, &mut nested, false), true),
                    t => (self.ty(&site, At::plain(&name), t, &mut nested, false), false),
                };
                Variant {
                    ty,
                    nullable,
                    description: String::new(),
                }
            })
            .collect();
        UnionDecl {
            r,
            description,
            see,
            variants,
            nested,
        }
    }

    /// The declaration of a named model, if it has a shape Ruby declares.
    #[must_use]
    pub fn model_decl(&self, d: &ModelDecl) -> Option<Decl> {
        let r = self.model_ref(&d.r#ref);
        match &d.ty {
            Type::Object(o) => {
                let mut c = self.class(r, o, None, false);
                c.description.clone_from(&d.description);
                Some(Decl::Class(c))
            }
            Type::Enum { members } => Some(Decl::Enum(enum_decl(r, members, d.description.clone(), None))),
            Type::Union(u) => Some(Decl::Union(self.union(r, u, d.description.clone(), None))),
            _ => None,
        }
    }

    /// The parameters class of `m`: path parameters first (without their descriptions), then the
    /// query and body fields in Stainless's order.
    #[must_use]
    pub fn params_class(&self, r: ClassRef, m: &Method, see: See) -> Class {
        let site = Site {
            parent: &r,
            see_parent: None,
        };
        let mut nested = Vec::new();
        let mut fields: Vec<RField> = m
            .path_params
            .iter()
            .map(|p| {
                let mut f = self.field(&site, &p.field, &mut nested);
                f.description.clear();
                f
            })
            .collect();
        let mut rest: Vec<Field> = m.query_params.iter().map(|p| p.field.clone()).collect();
        if let Some(o) = m.body.as_ref().and_then(|b| self.client.object_of(&b.ty)) {
            rest.extend(o.fields.iter().cloned());
        }
        for f in sort_fields(&rest) {
            fields.push(self.field(&site, f, &mut nested));
        }
        Class {
            base: None,
            description: String::new(),
            see: Some(see),
            fields,
            nested,
            params: true,
            r,
        }
    }

    /// The class (or other declaration) of an inline response.
    #[must_use]
    pub fn response_decl(&self, r: ClassRef, ty: &Type, see: See) -> Option<Decl> {
        match ty {
            Type::Object(o) => Some(Decl::Class(self.class(r, o, Some(see), false))),
            Type::Enum { members } => Some(Decl::Enum(enum_decl(r, members, String::new(), Some(see)))),
            Type::Union(u) => Some(Decl::Union(self.union(r, u, String::new(), Some(see)))),
            _ => None,
        }
    }

    /// The type of a method's response as the resource names it.
    #[must_use]
    pub fn response_type(&self, r: &ClassRef, ty: &Type) -> RType {
        match ty {
            Type::Object(_) => RType::Model(r.clone()),
            Type::Enum { .. } => RType::Enum(r.clone()),
            Type::Union(_) => RType::Union(r.clone()),
            other => {
                let site = Site {
                    parent: r,
                    see_parent: None,
                };
                self.ty(&site, At::plain(&[]), other, &mut Vec::new(), false)
            }
        }
    }
}

fn enum_decl(
    r: ClassRef,
    members: &[ferrule_sdk_model::model::EnumMember],
    description: String,
    see: Option<See>,
) -> EnumDecl {
    let owner = r.name().to_owned();
    EnumDecl {
        members: members
            .iter()
            .map(|m| EnumMember {
                constant: constant(&words(&m.name), &owner),
                value: m.value.clone(),
            })
            .collect(),
        r,
        description,
        see,
    }
}

/// The prefix of a method's types: the resource's key with its last word singular
/// (`trip-details` -> `TripDetail`); empty for a `$client` method.
#[must_use]
pub fn method_prefix(resource: Option<&Resource>, casings: Casings<'_>) -> String {
    resource.map_or_else(String::new, |r| class_name(&singular(&words(&r.name)), casings))
}

/// `snake` of the prefix: `trip_detail`.
#[must_use]
pub fn method_prefix_file(resource: Option<&Resource>) -> String {
    resource.map_or_else(String::new, |r| snake(&singular(&words(&r.name))))
}
