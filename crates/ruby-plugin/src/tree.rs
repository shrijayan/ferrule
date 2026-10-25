//! The gem's declarations: every model class, enum and union module, nested as Stainless nests
//! them, built from the neutral model. The three views of each file print from these (see
//! `emit::model`), so the shape is decided once.

use ferrule_sdk_model::model::{Client, Field, Literal, Method, ModelDecl, Object, Resource, Type, TypeRef, Union};

use crate::naming::{Casings, class_name, constant, singular, snake, split_words, words};
use crate::types::{ClassRef, RType};

/// Where a field is declared and sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A field of a model or a body.
    Plain,
    /// Sent by the server only (`readOnly`): declared in a `response_only` block.
    ResponseOnly,
    /// A path or query parameter of a params class: the resource renames it on the wire, so its
    /// declaration has no `api_name:`.
    Url,
}

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
    pub role: Role,
}

impl RField {
    /// The argument in a generated initializer: `name:`, `name: nil`, or a constant's value.
    #[must_use]
    pub fn init_arg(&self) -> String {
        match &self.ty {
            RType::Const(v) => format!("{}: {}", self.name, crate::printer::literal(v)),
            _ if self.required => format!("{}:", self.name),
            _ => format!("{}: nil", self.name),
        }
    }

    /// Whether a caller must pass it (a constant has a default).
    #[must_use]
    pub fn must_pass(&self) -> bool {
        self.required && !matches!(self.ty, RType::Const(_))
    }

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
    Alias(AliasDecl),
}

impl Decl {
    #[must_use]
    pub fn r(&self) -> &ClassRef {
        match self {
            Self::Class(c) => &c.r,
            Self::Enum(e) => &e.r,
            Self::Union(u) => &u.r,
            Self::Alias(a) => &a.r,
        }
    }
}

/// A named converter: `Extensions = ArrayOf[...]`, with the declarations its type names before it
/// (an array response's `...Item` union).
#[derive(Debug, Clone, PartialEq)]
pub struct AliasDecl {
    pub r: ClassRef,
    pub description: String,
    pub ty: RType,
    pub before: Vec<Decl>,
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
    /// Whether requests carry it (a params class, or a model reached from one): Sorbet types its
    /// enums as `OrSymbol` and its unions as `T.any(...)` (see `emit::mark_inputs`).
    pub input: bool,
}

impl Class {
    /// The fields in the order a generated initializer takes them: the ones a caller must
    /// pass, the optional ones, then the constants (they have defaults).
    #[must_use]
    pub fn init_order(&self) -> Vec<&RField> {
        let is_const = |f: &&RField| matches!(f.ty, RType::Const(_));
        let mut out: Vec<&RField> = self.fields.iter().filter(|f| f.must_pass()).collect();
        out.extend(self.fields.iter().filter(|f| !f.must_pass() && !is_const(f)));
        out.extend(self.fields.iter().filter(is_const));
        out
    }
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
    /// The variant's model description, written above it.
    pub description: String,
    /// The discriminator value that selects it (`variant :text, -> { ... }`).
    pub key: Option<String>,
    /// The converter constant an array variant is declared as (`StringArray`).
    pub constant: Option<ClassRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionDecl {
    pub r: ClassRef,
    pub description: String,
    pub see: Option<See>,
    /// The discriminating property's wire name.
    pub discriminator: Option<String>,
    pub variants: Vec<Variant>,
    pub nested: Vec<Decl>,
    /// The converter constants of its array variants (`StringArray`), after everything else.
    pub consts: Vec<AliasDecl>,
}

/// How a method sends its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    None,
    /// `body: parsed`: the parameters are the body's fields.
    Fields,
    /// `body: parsed[:name]`: one parameter is the body.
    Param(String),
}

/// A method's arguments: its path parameters (in path order) and the rest (query parameters and
/// body fields, Stainless's order), with their descriptions.
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub path: Vec<RField>,
    pub rest: Vec<RField>,
    pub body: Body,
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

/// Field names Stainless suffixes with `_`: Ruby's keywords and methods every model has.
const FIELD_RESERVED: [&str; 41] = [
    "alias",
    "and",
    "begin",
    "break",
    "case",
    "class",
    "def",
    "defined?",
    "do",
    "else",
    "elsif",
    "end",
    "ensure",
    "false",
    "for",
    "if",
    "in",
    "module",
    "next",
    "nil",
    "not",
    "or",
    "redo",
    "rescue",
    "retry",
    "return",
    "self",
    "super",
    "then",
    "true",
    "undef",
    "unless",
    "until",
    "when",
    "while",
    "yield",
    "__method__",
    "__FILE__",
    "format",
    "method",
    "y",
];

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

    /// The class of a named model: `References`, or `Accounts::OriginRequest` for a model a
    /// subresource declares (its parent's namespace, as Python's `types/accounts/`), aliased at the
    /// top of the module.
    #[must_use]
    pub fn model_ref(&self, r: &TypeRef) -> ClassRef {
        let name = if r.name.written.is_empty() {
            class_name(&r.name.words, self.casings)
        } else {
            class_name(&crate::naming::class_words(&r.name.written), self.casings)
        };
        let parents = &r.path[..r.path.len().saturating_sub(1)];
        let mut segs: Vec<String> = parents
            .iter()
            .enumerate()
            .filter(|(i, n)| !(*i == 0 && n.written == "shared"))
            .map(|(_, n)| class_name(&words(n), self.casings))
            .collect();
        segs.push(name);
        ClassRef { segs, aliased: true }
    }

    /// The type a reference to a named model has.
    fn ref_type(&self, r: &TypeRef) -> RType {
        let c = self.model_ref(r);
        match self.client.model(r).map(|d| &d.ty) {
            Some(Type::Enum { .. }) => RType::Enum(c),
            Some(Type::Union(u)) => {
                let variants = self.variant_types(&c, u);
                RType::Union(c, variants)
            }
            Some(t @ (Type::Array { .. } | Type::Map { .. })) => match self.alias(c.clone(), t, String::new()) {
                Decl::Alias(a) => RType::Alias(c, Box::new(a.ty)),
                _ => RType::Unknown,
            },
            _ => RType::Model(c),
        }
    }

    /// The types of a union's variants, for YARD.
    fn variant_types(&self, r: &ClassRef, u: &Union) -> Vec<RType> {
        self.union(r.clone(), u, String::new(), None)
            .variants
            .into_iter()
            .map(|v| {
                if v.nullable {
                    RType::Nullable(Box::new(v.ty))
                } else {
                    v.ty
                }
            })
            .collect()
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
            Type::Ref(r) => self.ref_type(r),
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
                let variants = decl
                    .variants
                    .iter()
                    .map(|v| {
                        if v.nullable {
                            RType::Nullable(Box::new(v.ty.clone()))
                        } else {
                            v.ty.clone()
                        }
                    })
                    .collect();
                nested.push(Decl::Union(decl));
                RType::Union(r, variants)
            }
        }
    }

    /// The class of an object: its fields in Stainless's order and what they nest.
    #[must_use]
    pub fn class(&self, r: ClassRef, o: &Object, see: Option<See>, params: bool) -> Class {
        let base = o.bases.first().map(|b| self.model_ref(b));
        let base_doc = o
            .bases
            .first()
            .and_then(|b| self.client.model(b))
            .map(|d| d.description.clone())
            .unwrap_or_default();
        let see_parent = (!params && base.is_none()).then(|| r.clone());
        let site = Site { parent: &r, see_parent };
        let mut nested = Vec::new();
        let fields = sort_fields(&o.fields)
            .into_iter()
            .map(|f| self.field(&site, f, &mut nested))
            .collect();
        Class {
            base,
            description: o.description.clone().filter(|d| !d.is_empty()).unwrap_or(base_doc),
            see,
            fields,
            nested,
            params,
            input: params,
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
        let (ty, nullable) = match &f.ty {
            // A required field with one allowed value is a constant (`const: :S3`).
            Type::Enum { members } if f.required && members.len() == 1 => {
                (RType::Const(members[0].value.clone()), false)
            }
            ty => self.field_type(site, at, ty, nested),
        };
        let mut ruby = snake(&name);
        if FIELD_RESERVED.contains(&ruby.as_str()) {
            ruby.push('_');
        }
        RField {
            name: ruby,
            wire: f.wire_name.clone(),
            required: f.required,
            nullable,
            ty,
            description: f.description.clone(),

            role: if f.read_only { Role::ResponseOnly } else { Role::Plain },
        }
    }

    fn union(&self, r: ClassRef, u: &Union, description: String, see: Option<See>) -> UnionDecl {
        let site = Site {
            parent: &r,
            see_parent: None,
        };
        let mut nested = Vec::new();
        let mut consts = Vec::new();
        let mut variants = Vec::new();
        for (i, v) in u.variants.iter().enumerate() {
            // A discriminator value is the variant's key when the mapping gives it one (the
            // adapter writes the schema name when it does not); an inline variant with a key is
            // named after it.
            let schema = match &v.ty {
                Type::Object(o) => o.origin.clone(),
                Type::Ref(r) => Some(r.schema.clone()),
                _ => None,
            };
            let key = u
                .discriminator
                .as_ref()
                .and(v.discriminator_value.clone())
                .filter(|k| Some(k) != schema.as_ref());
            // Stainless names an unnamed variant by position (`x-stainless-variantName` and the
            // schema title are not in the model).
            let name = match (&key, &v.ty) {
                (Some(k), Type::Object(_)) => split_words(k),
                _ => v
                    .name
                    .as_ref()
                    .map(words)
                    .filter(|w| !w.is_empty())
                    .unwrap_or_else(|| split_words(&format!("UnionMember{i}"))),
            };
            let (inner, nullable) = match &v.ty {
                Type::Nullable { inner } => (&**inner, true),
                t => (t, false),
            };
            let mut constant = None;
            let ty = match inner {
                Type::Enum { members } if members.len() == 1 => RType::Const(members[0].value.clone()),
                Type::Array { items } => {
                    let item = self.ty(&site, At::plain(&name), items, &mut nested, false);
                    let base = match &item {
                        RType::Model(c) | RType::Enum(c) | RType::Union(c, _) | RType::Alias(c, _) => {
                            c.name().to_owned()
                        }
                        scalar => scalar.rb_inner("").trim_start_matches("::Internal::Type::").to_owned(),
                    };
                    let cr = r.child(&format!("{base}Array"));
                    let array = RType::Array(Box::new(item));
                    consts.push(AliasDecl {
                        r: cr.clone(),
                        description: String::new(),
                        ty: array.clone(),
                        before: Vec::new(),
                    });
                    constant = Some(cr);
                    array
                }
                t => self.ty(&site, At::plain(&name), t, &mut nested, false),
            };
            let description = match &v.ty {
                Type::Ref(r) => self.client.model(r).map(|d| d.description.clone()).unwrap_or_default(),
                Type::Object(o) => o.description.clone().unwrap_or_default(),
                _ => String::new(),
            };
            variants.push(Variant {
                ty,
                nullable,
                description,
                key,
                constant,
            });
        }
        // Keyed variants first, as Stainless lists the discriminator's mapping.
        variants.sort_by_key(|v| v.key.is_none());
        UnionDecl {
            r,
            description,
            see,
            discriminator: u.discriminator.as_ref().map(|d| d.wire_name.clone()),
            variants,
            nested,
            consts,
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
            t @ (Type::Array { .. } | Type::Map { .. }) => Some(self.alias(r, t, d.description.clone())),
            _ => None,
        }
    }

    /// A converter constant of an array or map type; an inline item union or object is declared
    /// before it as `<Name>Item`.
    fn alias(&self, r: ClassRef, ty: &Type, description: String) -> Decl {
        let item = format!("{}Item", r.name());
        let mut parent = r.clone();
        parent.segs.pop();
        let site = Site {
            parent: &parent,
            see_parent: None,
        };
        let mut before = Vec::new();
        let words = crate::naming::class_words(&item);
        let inner = match ty {
            Type::Array { items } => {
                RType::Array(Box::new(self.ty(&site, At::plain(&words), items, &mut before, false)))
            }
            Type::Map { values } => RType::Map(Box::new(self.ty(&site, At::plain(&words), values, &mut before, false))),
            other => self.ty(&site, At::plain(&words), other, &mut before, false),
        };
        Decl::Alias(AliasDecl {
            r,
            description,
            ty: inner,
            before,
        })
    }

    /// The object of a response, when Ruby declares a class for it: an object, or the empty
    /// object Forge maps to a map of unknown values.
    fn response_object(ty: &Type) -> Option<std::borrow::Cow<'_, Object>> {
        match ty {
            Type::Object(o) => Some(std::borrow::Cow::Borrowed(o)),
            Type::Map { values }
                if matches!(
                    **values,
                    Type::Scalar {
                        scalar: ferrule_sdk_model::model::Scalar::Unknown
                    }
                ) =>
            {
                Some(std::borrow::Cow::Owned(Object::default()))
            }
            _ => None,
        }
    }

    /// A method's arguments and its parameters class. The class holds the path parameters (in
    /// path order, without their descriptions), then the query parameters and an inline object
    /// body's fields in Stainless's order; a body that is a named object model becomes the
    /// class's base instead, and a body of another named model (a union) is a method argument
    /// only, named after the model.
    #[must_use]
    pub fn method_shape(&self, r: ClassRef, m: &Method, see: See) -> (Args, Class) {
        let site = Site {
            parent: &r,
            see_parent: None,
        };
        let mut nested = Vec::new();
        let path: Vec<RField> = m
            .path_params
            .iter()
            .map(|p| RField {
                role: Role::Url,
                ..self.field(&site, &p.field, &mut nested)
            })
            .collect();
        let mut own: Vec<Field> = m.query_params.iter().map(|p| p.field.clone()).collect();
        let mut base = None;
        let mut inherited = Vec::new();
        let mut body = Body::None;
        if let Some(b) = &m.body {
            match (&b.ty, self.client.object_of(&b.ty)) {
                (Type::Ref(rf), Some(o)) => {
                    base = Some(self.model_ref(rf));
                    let model_site = Site {
                        parent: &self.model_ref(rf),
                        see_parent: None,
                    };
                    inherited = sort_fields(&o.fields)
                        .into_iter()
                        .map(|f| self.field(&model_site, f, &mut Vec::new()))
                        .collect();
                    body = Body::Fields;
                }
                (_, Some(o)) => {
                    own.extend(o.fields.iter().cloned());
                    body = Body::Fields;
                }
                (ty, None) => {
                    let wire = match ty {
                        Type::Ref(rf) => snake(&crate::naming::class_words(self.model_ref(rf).name())),
                        _ => "body".to_owned(),
                    };
                    let field = Field {
                        name: ferrule_sdk_model::model::Name {
                            words: split_words(&wire),
                            written: wire.clone(),
                        },
                        wire_name: wire.clone(),
                        required: b.required,
                        deprecated: false,
                        read_only: false,
                        description: b.description.clone(),
                        default: None,
                        ty: ty.clone(),
                    };
                    inherited.push(self.field(&site, &field, &mut nested));
                    body = Body::Param(wire);
                }
            }
        }
        let in_query = |f: &Field| m.query_params.iter().any(|q| q.field.wire_name == f.wire_name);
        let own: Vec<RField> = sort_fields(&own)
            .into_iter()
            .map(|f| RField {
                role: if in_query(f) { Role::Url } else { Role::Plain },
                ..self.field(&site, f, &mut nested)
            })
            .collect();
        let mut fields: Vec<RField> = path
            .iter()
            .cloned()
            .map(|mut f| {
                f.description.clear();
                f
            })
            .collect();
        fields.extend(own.iter().cloned());
        let mut rest = own;
        rest.extend(inherited);
        let class = Class {
            base,
            description: String::new(),
            see: Some(see),
            fields,
            nested,
            params: true,
            input: true,
            r,
        };
        (Args { path, rest, body }, class)
    }

    /// The class (or other declaration) of an inline response.
    #[must_use]
    pub fn response_decl(&self, r: ClassRef, ty: &Type, see: See) -> Option<Decl> {
        if let Some(o) = Self::response_object(ty) {
            return Some(Decl::Class(self.class(r, &o, Some(see), false)));
        }
        match ty {
            Type::Enum { members } => Some(Decl::Enum(enum_decl(r, members, String::new(), Some(see)))),
            Type::Union(u) => Some(Decl::Union(self.union(r, u, String::new(), Some(see)))),
            Type::Array { .. } => Some(self.alias(r, ty, String::new())),
            _ => None,
        }
    }

    /// The type of a method's response as the resource names it.
    #[must_use]
    pub fn response_type(&self, r: &ClassRef, ty: &Type) -> RType {
        if Self::response_object(ty).is_some() {
            return RType::Model(r.clone());
        }
        match ty {
            Type::Enum { .. } => RType::Enum(r.clone()),
            Type::Union(u) => RType::Union(r.clone(), self.variant_types(r, u)),
            Type::Array { .. } => match self.alias(r.clone(), ty, String::new()) {
                Decl::Alias(a) => RType::Alias(r.clone(), Box::new(a.ty)),
                _ => RType::Unknown,
            },
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
