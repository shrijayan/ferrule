//! The package's declarations: every model class, enum and union class, each in its own file and
//! nested by namespace as Stainless nests them (`Files\File\AITag`), built from the neutral model.

use std::collections::{HashMap, HashSet};

use ferrule_sdk_model::model::{Client, Field, Literal, Method, ModelDecl, Object, Type, TypeRef, Union};
use serde_json::Value;

use crate::naming::{Casings, camel, case_name, class_name, class_words, singular, split_words, type_name, words};
use crate::types::{Backing, ClassPath, PType};

/// A field of a model or a parameter of a method.
#[derive(Debug, Clone, PartialEq)]
pub struct PField {
    /// `fileID`: the property and parameter name.
    pub name: String,
    /// `FileID`: the setter is `withFileID`.
    pub setter: String,
    /// `fileId`.
    pub wire: String,
    pub required: bool,
    /// The value may be null.
    pub nullable: bool,
    pub ty: PType,
    pub description: String,
    /// The schema's `default`, if any.
    pub default: Option<Value>,
    /// A path parameter in a params class: declared without its wire name.
    pub path: bool,
}

impl PField {
    /// Whether it is required and not a constant: the constructor's docs name it.
    #[must_use]
    pub fn required_value(&self) -> bool {
        self.required && !matches!(self.ty, PType::Const(_))
    }

    /// Whether a caller must pass it to `with()` (a constant or a default fills it otherwise).
    #[must_use]
    pub fn must_pass(&self) -> bool {
        self.required_value() && self.default.is_none()
    }

    /// The default as a PHP literal (`false`, `1000`, `'all'`).
    #[must_use]
    pub fn default_literal(&self) -> Option<String> {
        match self.default.as_ref()? {
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::String(v) => Some(crate::types::literal(&Literal::String(v.clone()))),
            Value::Array(a) if a.is_empty() => Some("[]".to_owned()),
            Value::Object(o) if o.is_empty() => Some("[]".to_owned()),
            _ => None,
        }
    }

    /// The wire name in `#[Required('jobId')]`, when it is not the property name.
    #[must_use]
    pub fn api_name(&self) -> Option<&str> {
        (self.wire != self.name && !self.path).then_some(self.wire.as_str())
    }
}

/// A declaration with its own file.
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Class(Class),
    Enum(EnumDecl),
    Union(UnionDecl),
}

impl Decl {
    #[must_use]
    pub fn path(&self) -> &ClassPath {
        match self {
            Self::Class(c) => &c.path,
            Self::Enum(e) => &e.path,
            Self::Union(u) => &u.path,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub path: ClassPath,
    pub description: String,
    /// `@see Imagekit\Services\FoldersService::copy()`: the method a params class is for.
    pub see: Option<String>,
    pub fields: Vec<PField>,
    /// A method's parameters (`SdkParams`).
    pub params: bool,
}

impl Class {
    /// The fields in the order the class declares them: the constants, then the others.
    #[must_use]
    pub fn declared(&self) -> Vec<&PField> {
        let is_const = |f: &&PField| matches!(f.ty, PType::Const(_));
        let mut out: Vec<&PField> = self.fields.iter().filter(is_const).collect();
        out.extend(self.fields.iter().filter(|f| !is_const(f)));
        out
    }

    /// The fields `with()` takes: the ones a caller must pass, the required ones with a default,
    /// then the optional ones.
    #[must_use]
    pub fn with_order(&self) -> Vec<&PField> {
        let mut out: Vec<&PField> = self.fields.iter().filter(|f| f.must_pass()).collect();
        out.extend(self.fields.iter().filter(|f| f.required_value() && !f.must_pass()));
        out.extend(self.fields.iter().filter(|f| !f.required));
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMember {
    /// `PENDING`.
    pub case: String,
    pub value: Literal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub path: ClassPath,
    pub description: String,
    pub members: Vec<EnumMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub ty: PType,
    /// The discriminator value that selects it.
    pub key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionDecl {
    pub path: ClassPath,
    pub description: String,
    /// The discriminating property's wire name.
    pub discriminator: Option<String>,
    pub variants: Vec<Variant>,
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

/// Builds declarations for one SDK; every declaration it builds is collected in `decls`.
pub struct Builder<'a> {
    pub client: &'a Client,
    /// The model before `for_target` inlined the models PHP does not generate.
    pub orig: &'a Client,
    pub casings: Casings<'a>,
    /// Where each named model is declared, by schema name.
    pub places: HashMap<String, ClassPath>,
    /// The named models in a reference cycle, which phpdoc names `mixed` (phpstan cannot resolve
    /// a recursive type alias).
    pub recursive: HashSet<String>,
    pub decls: Vec<Decl>,
}

/// How a method's arguments are sent.
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    /// The last path parameter: the method's first positional argument.
    pub positional: Option<PField>,
    /// The other path parameters, in path order.
    pub path: Vec<PField>,
    pub query: Vec<PField>,
    pub body: Vec<PField>,
    /// The params class, when the method has parameters besides `positional`.
    pub params: Option<Class>,
}

impl Args {
    /// The parameters besides the positional one: path, query and body, in the params class's
    /// order.
    #[must_use]
    pub fn rest(&self) -> Vec<&PField> {
        self.params
            .as_ref()
            .map(|c| c.fields.iter().collect())
            .unwrap_or_default()
    }
}

impl<'a> Builder<'a> {
    #[must_use]
    pub fn new(client: &'a Client, orig: &'a Client, places: HashMap<String, ClassPath>) -> Self {
        Self {
            client,
            orig,
            casings: Casings(&client.casings),
            places,
            recursive: recursive_models(client),
            decls: Vec::new(),
        }
    }

    /// A builder whose declarations are thrown away (for a type named elsewhere).
    fn scratch(&self) -> Self {
        Self {
            client: self.client,
            orig: self.orig,
            casings: self.casings,
            places: self.places.clone(),
            recursive: self.recursive.clone(),
            decls: Vec::new(),
        }
    }

    /// The class of a named model.
    #[must_use]
    pub fn model_path(&self, r: &TypeRef) -> ClassPath {
        self.places
            .get(&r.schema)
            .cloned()
            .unwrap_or_else(|| default_place(r, self.casings))
    }

    fn ref_type(&self, r: &TypeRef) -> PType {
        let t = self.named_type(r);
        if self.recursive.contains(&r.schema) {
            PType::Rec(Box::new(t))
        } else {
            t
        }
    }

    fn named_type(&self, r: &TypeRef) -> PType {
        let c = self.model_path(r);
        match self.client.model(r).map(|d| &d.ty) {
            Some(Type::Enum { members }) => PType::Enum(c, backing(members)),
            Some(Type::Union(u)) => {
                let mut scratch = self.scratch();
                let variants = scratch.union(c.clone(), u, String::new()).variants;
                PType::Union(c, variants.into_iter().map(|v| v.ty).collect())
            }
            Some(Type::Array { items }) => {
                let mut scratch = self.scratch();
                let parent = ClassPath::new(c.namespace().to_vec());
                PType::List(Box::new(scratch.ty(&parent, &item_words(&c), "", items)))
            }
            Some(Type::Map { values }) => {
                let mut scratch = self.scratch();
                let parent = ClassPath::new(c.namespace().to_vec());
                PType::Map(Box::new(scratch.ty(&parent, &item_words(&c), "", values)))
            }
            Some(Type::Scalar { .. }) | None => PType::Mixed,
            _ => PType::Model(c),
        }
    }

    /// A field's type; nested declarations are collected. `name` is the field's words, from
    /// which a nested declaration is named (singular for list items).
    fn ty(&mut self, parent: &ClassPath, name: &[String], doc: &str, ty: &Type) -> PType {
        use ferrule_sdk_model::model::Scalar as S;
        match ty {
            Type::Scalar { scalar } => match scalar {
                S::String | S::Uuid | S::Bytes => PType::String,
                S::Boolean => PType::Bool,
                S::Float | S::Double => PType::Float,
                S::DateTime => PType::DateTime,
                S::Date => PType::Date,
                S::Binary => PType::File,
                S::Unknown | S::Null => PType::Mixed,
                s if s.is_integer() => PType::Int,
                _ => PType::Mixed,
            },
            Type::Nullable { inner } => PType::Nullable(Box::new(self.ty(parent, name, doc, inner))),
            Type::Array { items } => {
                let item = singular(name);
                PType::List(Box::new(self.ty(parent, &item, "", items)))
            }
            Type::Map { values } => PType::Map(Box::new(self.ty(parent, name, "", values))),
            Type::Ref(r) => self.ref_type(r),
            Type::Object(o) => {
                let c = parent.child(&type_name(name, self.casings));
                let class = self.class(c.clone(), o, None, false);
                self.decls.push(Decl::Class(class));
                PType::Model(c)
            }
            Type::Enum { members } => {
                let c = parent.child(&type_name(name, self.casings));
                let e = enum_decl(c.clone(), members, doc.to_owned());
                self.decls.push(Decl::Enum(e));
                PType::Enum(c, backing(members))
            }
            Type::Union(u) => {
                let c = parent.child(&type_name(name, self.casings));
                let decl = self.union(c.clone(), u, doc.to_owned());
                let variants = decl.variants.iter().map(|v| v.ty.clone()).collect();
                self.decls.push(Decl::Union(decl));
                PType::Union(c, variants)
            }
        }
    }

    /// The fields of an object: the fields of the model it extends, then its own, each in
    /// Stainless's order.
    fn fields(&mut self, path: &ClassPath, o: &Object) -> Vec<PField> {
        let mut out = Vec::new();
        for b in &o.bases {
            if let Some(Type::Object(base)) = self.client.model(b).map(|d| d.ty.clone()) {
                let base_path = self.model_path(b);
                let mut scratch = self.scratch();
                let mut inherited = scratch.fields(&base_path, &base);
                // A field of the base that refers to a model is described by that model, not by
                // the property (the description beside its `$ref`), as Stainless merges `allOf`.
                if let Some(Type::Object(orig)) = self.orig.model(b).map(|d| &d.ty) {
                    for f in &mut inherited {
                        let r = orig
                            .fields
                            .iter()
                            .find(|g| g.wire_name == f.wire)
                            .and_then(|g| match &g.ty {
                                Type::Ref(r) => Some(r),
                                Type::Nullable { inner } => match &**inner {
                                    Type::Ref(r) => Some(r),
                                    _ => None,
                                },
                                _ => None,
                            });
                        if let Some(r) = r {
                            f.description = self.orig.model(r).map(|d| d.description.clone()).unwrap_or_default();
                        }
                    }
                }
                out.extend(inherited);
            }
        }
        for f in sort_fields(&o.fields) {
            out.push(self.field(path, f));
        }
        out
    }

    /// The class of an object: its fields in Stainless's order and what they nest.
    pub fn class(&mut self, path: ClassPath, o: &Object, see: Option<String>, params: bool) -> Class {
        let fields = self.fields(&path, o);
        // An object without a description of its own has its base's.
        let base_doc = o
            .bases
            .first()
            .and_then(|b| self.client.model(b))
            .map(|d| d.description.clone())
            .unwrap_or_default();
        Class {
            description: o.description.clone().filter(|d| !d.is_empty()).unwrap_or(base_doc),
            see,
            fields,
            params,
            path,
        }
    }

    fn field(&mut self, parent: &ClassPath, f: &Field) -> PField {
        let name = words(&f.name);
        let (ty, nullable) = match &f.ty {
            // A required field with one allowed value is a constant (`type: 'S3'`); an optional
            // one has the value's type.
            Type::Enum { members } if f.required && members.len() == 1 => {
                (PType::Const(members[0].value.clone()), false)
            }
            Type::Enum { members } if members.len() == 1 => (literal_type(&members[0].value), false),
            Type::Nullable { inner } => (self.ty(parent, &name, &f.description, inner), true),
            ty => (self.ty(parent, &name, &f.description, ty), false),
        };
        PField {
            name: camel(&name, self.casings),
            setter: setter_name(&f.wire_name, &name, self.casings),
            wire: f.wire_name.clone(),
            required: f.required,
            nullable,
            ty,
            description: f.description.clone(),
            default: f.default.clone(),
            path: false,
        }
    }

    fn union(&mut self, path: ClassPath, u: &Union, description: String) -> UnionDecl {
        let mut variants = Vec::new();
        for (i, v) in u.variants.iter().enumerate() {
            let schema = match &v.ty {
                Type::Object(o) => o.origin.clone(),
                Type::Ref(r) => Some(r.schema.clone()),
                _ => None,
            };
            let key = u.discriminator.as_ref().and(v.discriminator_value.clone());
            let named_key = key.clone().filter(|k| Some(k) != schema.as_ref());
            // Stainless names an unnamed variant by position (`x-stainless-variantName` and the
            // schema title are not in the model).
            let name = match (&named_key, &v.ty) {
                (Some(k), Type::Object(_)) => split_words(k),
                _ => v
                    .name
                    .as_ref()
                    .map(words)
                    .filter(|w| !w.is_empty())
                    .unwrap_or_else(|| split_words(&format!("UnionMember{i}"))),
            };
            let ty = match &v.ty {
                Type::Enum { members } if members.len() == 1 => PType::Const(members[0].value.clone()),
                t => self.ty(&path, &name, "", t),
            };
            variants.push(Variant { ty, key });
        }
        // Keyed variants first, as Stainless lists the discriminator's mapping.
        variants.sort_by_key(|v| v.key.is_none());
        UnionDecl {
            path,
            description,
            discriminator: u.discriminator.as_ref().map(|d| d.wire_name.clone()),
            variants,
        }
    }

    /// The declaration of a named model, if it has a shape PHP declares.
    pub fn model_decl(&mut self, d: &ModelDecl) {
        let path = self.model_path(&d.r#ref);
        let decl = match &d.ty {
            Type::Object(o) => {
                let mut c = self.class(path, o, None, false);
                c.description.clone_from(&d.description);
                Decl::Class(c)
            }
            Type::Enum { members } => Decl::Enum(enum_decl(path, members, d.description.clone())),
            Type::Union(u) => Decl::Union(self.union(path, u, d.description.clone())),
            // A list or map model is not a class; an inline item of it is (`Extensions` ->
            // `ExtensionItem`, `Labels` -> `LabelItem`).
            Type::Array { items: inner } | Type::Map { values: inner } => {
                let parent = ClassPath::new(path.namespace().to_vec());
                self.ty(&parent, &item_words(&path), "", inner);
                return;
            }
            _ => return,
        };
        self.decls.push(decl);
    }

    /// A method's arguments and its params class (`path`, collected unless empty): the path
    /// parameters but the last, then the query parameters and the body's fields in Stainless's
    /// order. A path parameter's description stays on the method.
    pub fn method_args(&mut self, path: ClassPath, m: &Method, see: String) -> Args {
        let mut positional = None;
        let mut path_fields: Vec<PField> = m
            .path_params
            .iter()
            .map(|p| PField {
                path: true,
                ..self.field(&path, &p.field)
            })
            .collect();
        if !path_fields.is_empty() {
            positional = path_fields.pop();
        }
        let query: Vec<PField> = sort_fields(&m.query_params.iter().map(|p| p.field.clone()).collect::<Vec<_>>())
            .into_iter()
            .map(|f| self.field(&path, f))
            .collect();
        let body: Vec<PField> = match &m.body {
            Some(b) => match self.client.object_of(&b.ty) {
                Some(o) => {
                    let o = o.clone();
                    self.fields(&path, &o)
                }
                None => self.union_fields(&path, &b.ty),
            },
            None => Vec::new(),
        };
        let mut fields: Vec<PField> = path_fields
            .iter()
            .cloned()
            .map(|mut f| {
                f.description.clear();
                f
            })
            .collect();
        fields.extend(query.iter().cloned());
        fields.extend(body.iter().cloned());
        let params = (!fields.is_empty()).then(|| Class {
            path,
            description: if m.description.is_empty() {
                m.summary.clone()
            } else {
                m.description.clone()
            },
            see: Some(see),
            fields,
            params: true,
        });
        Args {
            positional,
            path: path_fields,
            query,
            body,
            params,
        }
    }

    /// The parameters of a body that is a union of objects: every variant's fields, merged in
    /// variant order, each where it first appears and as the last variant that has it declares it
    /// (its type, description and default); the discriminator comes first.
    fn union_fields(&mut self, path: &ClassPath, ty: &Type) -> Vec<PField> {
        let u = match ty {
            Type::Union(u) => u.clone(),
            Type::Ref(r) => match self.client.model(r).map(|d| &d.ty) {
                Some(Type::Union(u)) => u.clone(),
                _ => return Vec::new(),
            },
            _ => return Vec::new(),
        };
        let mut out: Vec<PField> = Vec::new();
        for v in &u.variants {
            let Some(o) = self.client.object_of(&v.ty).cloned() else {
                return Vec::new();
            };
            for f in self.fields(path, &o) {
                if let Some(i) = out.iter().position(|g| g.name == f.name) {
                    out[i] = f;
                } else {
                    out.push(f);
                }
            }
        }
        if let Some(d) = &u.discriminator
            && let Some(i) = out.iter().position(|f| f.wire == d.wire_name)
        {
            let f = out.remove(i);
            out.insert(0, f);
        }
        out
    }

    /// The type of a method's response, declaring an inline object, enum or union as `path`.
    pub fn response_type(&mut self, path: &ClassPath, ty: &Type) -> PType {
        match ty {
            Type::Object(o) => {
                let c = self.class(path.clone(), o, None, false);
                self.decls.push(Decl::Class(c));
                PType::Model(path.clone())
            }
            Type::Map { values }
                if matches!(
                    **values,
                    Type::Scalar {
                        scalar: ferrule_sdk_model::model::Scalar::Unknown
                    }
                ) =>
            {
                let c = self.class(path.clone(), &Object::default(), None, false);
                self.decls.push(Decl::Class(c));
                PType::Model(path.clone())
            }
            Type::Enum { members } => {
                self.decls
                    .push(Decl::Enum(enum_decl(path.clone(), members, String::new())));
                PType::Enum(path.clone(), backing(members))
            }
            Type::Union(u) => {
                let decl = self.union(path.clone(), u, String::new());
                let variants = decl.variants.iter().map(|v| v.ty.clone()).collect();
                self.decls.push(Decl::Union(decl));
                PType::Union(path.clone(), variants)
            }
            Type::Array { items } => {
                // A list response's inline item is `<Method>ResponseItem`.
                let parent = ClassPath::new(path.namespace().to_vec());
                PType::List(Box::new(self.ty(&parent, &item_words(path), "", items)))
            }
            other => self.ty(path, &[], "", other),
        }
    }
}

/// Whether a request body's fields are a method's parameters: an object, or a union of objects
/// (whose fields [`Builder::method_args`] merges). Another body has no parameters to spread.
#[must_use]
pub fn body_spreads(client: &Client, ty: &Type) -> bool {
    if client.object_of(ty).is_some() {
        return true;
    }
    let u = match ty {
        Type::Union(u) => u,
        Type::Ref(r) => match client.model(r).map(|d| &d.ty) {
            Some(Type::Union(u)) => u,
            _ => return false,
        },
        _ => return false,
    };
    u.variants.iter().all(|v| client.object_of(&v.ty).is_some())
}

/// Where a named model is declared without a better place: its resource's namespace
/// (`Files\File`), or the root for a `$shared` model.
#[must_use]
pub fn default_place(r: &TypeRef, casings: Casings<'_>) -> ClassPath {
    let name = if r.name.written.is_empty() {
        class_name(&r.name.words, casings)
    } else {
        class_name(&class_words(&r.name.written), casings)
    };
    let parents = &r.path[..r.path.len().saturating_sub(1)];
    let mut segs: Vec<String> = parents
        .iter()
        .enumerate()
        .filter(|(i, n)| !(*i == 0 && n.written == "shared"))
        .map(|(_, n)| class_name(&words(n), casings))
        .collect();
    segs.push(name);
    ClassPath::new(segs)
}

fn enum_decl(path: ClassPath, members: &[ferrule_sdk_model::model::EnumMember], description: String) -> EnumDecl {
    EnumDecl {
        members: members
            .iter()
            .map(|m| EnumMember {
                case: case_name(&match &m.value {
                    Literal::String(s) => split_words(s),
                    other => split_words(&crate::types::literal(other)),
                }),
                value: m.value.clone(),
            })
            .collect(),
        path,
        description,
    }
}

/// The type of an enum's values, from its first member (as the enum declares it).
fn backing(members: &[ferrule_sdk_model::model::EnumMember]) -> Backing {
    Backing::of(members.first().map(|m| &m.value))
}

/// The type of a literal value: an optional field with one allowed value has it.
fn literal_type(v: &Literal) -> PType {
    match v {
        Literal::String(_) => PType::String,
        Literal::Number(n) if n.fract() == 0.0 => PType::Int,
        Literal::Number(_) => PType::Float,
        Literal::Boolean(_) => PType::Bool,
    }
}

/// The named models a type refers to, through inline types (not through other named models).
fn refs(ty: &Type, out: &mut Vec<String>) {
    match ty {
        Type::Scalar { .. } | Type::Enum { .. } => {}
        Type::Array { items: inner } | Type::Map { values: inner } | Type::Nullable { inner } => refs(inner, out),
        Type::Object(o) => {
            out.extend(o.bases.iter().map(|b| b.schema.clone()));
            for f in &o.fields {
                refs(&f.ty, out);
            }
            if let Some(extra) = &o.extra {
                refs(extra, out);
            }
        }
        Type::Union(u) => {
            for v in &u.variants {
                refs(&v.ty, out);
            }
        }
        Type::Ref(r) => out.push(r.schema.clone()),
    }
}

/// The named models on a reference cycle: each one reaches itself.
#[must_use]
pub fn recursive_models(client: &Client) -> HashSet<String> {
    let edges: HashMap<&str, Vec<String>> = client
        .models
        .iter()
        .map(|d| {
            let mut out = Vec::new();
            refs(&d.ty, &mut out);
            (d.r#ref.schema.as_str(), out)
        })
        .collect();
    let mut out = HashSet::new();
    for start in edges.keys() {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut pending: Vec<&str> = edges[start].iter().map(String::as_str).collect();
        while let Some(s) = pending.pop() {
            if s == *start {
                out.insert((*start).to_owned());
                break;
            }
            if seen.insert(s)
                && let Some(next) = edges.get(s)
            {
                pending.extend(next.iter().map(String::as_str));
            }
        }
    }
    out
}

/// A setter's name after `with`: a wire name in capitals as written (`ISO`), else the field's
/// words in `PascalCase` (`FileID`, `GpsVersionID`).
fn setter_name(wire: &str, words: &[String], casings: Casings<'_>) -> String {
    let caps = wire.len() > 1
        && wire.chars().any(|c| c.is_ascii_uppercase())
        && wire.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if caps {
        wire.to_owned()
    } else {
        class_name(words, casings)
    }
}

/// The words of the item of a list model or response: `Extensions` -> `ExtensionItem`,
/// `AssetListResponse` -> `AssetListResponseItem`.
fn item_words(c: &ClassPath) -> Vec<String> {
    let mut w = singular(&class_words(c.name()));
    w.push("Item".to_owned());
    w
}
