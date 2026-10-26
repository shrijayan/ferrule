//! The declaration tree: every file under `Models/` and the declarations in it, with the names
//! Stainless gives them.
//!
//! A file holds one top-level type (a method's params or response, or a named model) and the
//! types inlined in it, depth first in property order. An inline type is named after its
//! property (an array's items singular); when that name is taken in the namespace, its parent's
//! name comes first (`CustomMetadataFieldSchemaType`). Params and responses claim names before
//! named models, in service and method order, then named models in name order: so the first
//! params that inline a `Schema` keep `Schema`, and a named model with one gets
//! `CustomMetadataFieldSchema`. Names are claimed in one pass and the files built in another.

use std::collections::{HashMap, HashSet};

use ferrule_sdk_model::model::{Client, Field, Literal, ModelDecl, Object, Type, TypeRef, Union};

use crate::emit::Svc;
use crate::meta::CSharpTarget;
use crate::naming::{Casings, pascal, singular, split_words, words};
use crate::types::{CsType, EnumValue};

/// Where a property's value lives on its instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loc {
    /// A model's `_rawData`.
    Data,
    /// A params class's `_rawBodyData`.
    Body,
    Query,
    Header,
    /// A path param: a plain `string?` property.
    Path,
    /// A body that is not an object (a union, an array): the whole `RawBodyData` element.
    WholeBody,
}

impl Loc {
    /// The raw dictionary: `_rawData`, `_rawBodyData`.
    #[must_use]
    pub fn field(self) -> &'static str {
        match self {
            Self::Data => "_rawData",
            Self::Body => "_rawBodyData",
            Self::Query => "_rawQueryData",
            Self::Header => "_rawHeaderData",
            Self::Path | Self::WholeBody => "",
        }
    }
}

/// A property of a class.
#[derive(Debug, Clone)]
pub struct Prop {
    /// `RequestID`.
    pub name: String,
    pub wire: String,
    pub ty: CsType,
    pub required: bool,
    pub nullable: bool,
    pub description: String,
    pub deprecated: bool,
    /// A string `const`: the property is a `JsonElement` that always holds this value.
    pub constant: Option<Literal>,
    pub loc: Loc,
}

/// A class with properties (`JsonModel`).
#[derive(Debug, Clone)]
pub struct Obj {
    pub name: String,
    pub description: String,
    pub props: Vec<Prop>,
    /// Named bases (`allOf`): their classes and the names of their properties, for the implicit
    /// conversions.
    pub bases: Vec<(String, Vec<String>)>,
}

/// An enum with its values.
#[derive(Debug, Clone)]
pub struct En {
    pub name: String,
    pub description: String,
    /// (member, value), in spec order.
    pub members: Vec<(String, Literal)>,
    pub value: EnumValue,
}

/// One variant of a union.
#[derive(Debug, Clone)]
pub struct UVariant {
    /// `File`, `Double`: the `TryPick<label>` suffix.
    pub label: String,
    /// The variant's type.
    pub ty: CsType,
    /// The discriminator values that select it.
    pub discriminator: Vec<String>,
    /// An object variant's properties (the union exposes those all variants share).
    pub props: Vec<Prop>,
}

/// A union (`ModelBase` with one variant set).
#[derive(Debug, Clone)]
pub struct Un {
    pub name: String,
    pub description: String,
    pub variants: Vec<UVariant>,
    /// The discriminating property's wire name.
    pub discriminator: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Decl {
    Object(Obj),
    /// The inline member of an `allOf` with named bases (`SubtitleOverlaySubtitleOverlay`).
    Member(Obj),
    Enum(En),
    Union(Un),
}

impl Decl {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Object(o) | Self::Member(o) => &o.name,
            Self::Enum(e) => &e.name,
            Self::Union(u) => &u.name,
        }
    }
}

/// What a file's top-level type is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A method's params (`ParamsBase`): service and method index.
    Params(usize, usize),
    /// A method's inline response.
    Response(usize, usize),
    /// A named model.
    Model,
}

/// A file under `Models/`.
#[derive(Debug, Clone)]
pub struct ModelFile {
    /// The namespace below `Models`.
    pub ns: Vec<String>,
    pub kind: Kind,
    /// The top-level type first.
    pub decls: Vec<Decl>,
}

impl ModelFile {
    /// `Models/Files/FileUpdateParams.cs` (below `src/<Ns>/`).
    #[must_use]
    pub fn path(&self) -> String {
        let mut out = String::from("Models/");
        for s in &self.ns {
            out.push_str(s);
            out.push('/');
        }
        out.push_str(self.decls[0].name());
        out.push_str(".cs");
        out
    }
}

/// Where a named model is declared and what it is called.
#[derive(Debug, Clone)]
pub struct Named {
    pub ns: Vec<String>,
    pub class: String,
}

/// Every model file.
#[derive(Debug, Clone, Default)]
pub struct Files {
    pub files: Vec<ModelFile>,
    named: HashMap<String, Named>,
}

impl Files {
    /// The named model declared from `schema`.
    #[must_use]
    pub fn named(&self, schema: &str) -> Option<&Named> {
        self.named.get(schema)
    }
}

/// The names claimed per namespace, and the name of every inline position.
#[derive(Default)]
struct Namer {
    taken: HashMap<Vec<String>, HashSet<String>>,
    names: HashMap<String, String>,
}

impl Namer {
    /// The name of the inline type at `key`: `short` when free in `ns`, else `parent + short`.
    fn claim(&mut self, key: &str, ns: &[String], short: &str, parent: &str) -> String {
        if let Some(n) = self.names.get(key) {
            return n.clone();
        }
        let taken = self.taken.entry(ns.to_vec()).or_default();
        let name = if taken.contains(short) {
            format!("{parent}{short}")
        } else {
            short.to_owned()
        };
        taken.insert(name.clone());
        self.names.insert(key.to_owned(), name.clone());
        name
    }

    fn reserve(&mut self, ns: &[String], name: &str) {
        self.taken.entry(ns.to_vec()).or_default().insert(name.to_owned());
    }
}

struct Builder<'a> {
    client: &'a Client,
    casings: Casings<'a>,
    named: HashMap<String, Named>,
    namer: Namer,
    /// The root namespace, for `global::` names.
    root: String,
}

/// The refs a type reaches, following named models.
fn refs(client: &Client, ty: &Type, seen: &mut HashSet<String>) {
    match ty {
        Type::Ref(r) => {
            if seen.insert(r.schema.clone())
                && let Some(d) = client.model(r)
            {
                refs(client, &d.ty, seen);
            }
        }
        Type::Array { items } => refs(client, items, seen),
        Type::Map { values } => refs(client, values, seen),
        Type::Nullable { inner } => refs(client, inner, seen),
        Type::Object(o) => {
            for b in &o.bases {
                refs(client, &Type::Ref(b.clone()), seen);
            }
            for f in &o.fields {
                refs(client, &f.ty, seen);
            }
            if let Some(e) = &o.extra {
                refs(client, e, seen);
            }
        }
        Type::Union(u) => {
            for v in &u.variants {
                refs(client, &v.ty, seen);
            }
        }
        Type::Scalar { .. } | Type::Enum { .. } => {}
    }
}

fn method_refs(client: &Client, m: &ferrule_sdk_model::model::Method) -> HashSet<String> {
    let mut seen = HashSet::new();
    for p in m.path_params.iter().chain(&m.query_params).chain(&m.header_params) {
        refs(client, &p.field.ty, &mut seen);
    }
    if let Some(b) = &m.body {
        refs(client, &b.ty, &mut seen);
    }
    if let Some(r) = &m.response {
        refs(client, &r.ty, &mut seen);
    }
    seen
}

/// Where each resource model is declared: the models namespace of the first service (in
/// stainless.yml order) whose methods reach it, preferring one under the resources its path
/// names. Shared models are in `Models`.
///
/// `x-ferrule-models` records Python's module of a resource model (`File` for
/// `files.models.File`, `accounts` for `accounts.origins.models.OriginRequest`), not its resource
/// path, so the uses place the model (Phase 1 gap list: model positions for non-Python targets).
fn place(client: &Client, services: &[Svc<'_>], d: &ModelDecl) -> Vec<String> {
    let path: Vec<&str> = d.r#ref.path.iter().map(|n| n.written.as_str()).collect();
    if path.first() == Some(&"shared") {
        return Vec::new();
    }
    let declared = &path[..path.len().saturating_sub(1)];
    let users: Vec<&Svc<'_>> = services
        .iter()
        .filter(|s| {
            s.resource
                .methods
                .iter()
                .any(|m| method_refs(client, m).contains(&d.r#ref.schema))
        })
        .collect();
    let under = |s: &&&Svc<'_>| {
        let keys: Vec<&str> = s.resource.path.iter().map(|n| n.written.as_str()).collect();
        keys.len() >= declared.len() && keys[..declared.len()] == *declared
    };
    users
        .iter()
        .find(under)
        .or_else(|| users.first())
        .map(|s| s.model_ns.clone())
        .unwrap_or_default()
}

/// The params, responses and named models of an SDK.
#[must_use]
pub fn build(client: &Client, target: &CSharpTarget, services: &[Svc<'_>]) -> Files {
    let casings = Casings(&client.casings);
    let mut named = HashMap::new();
    let prefixes: HashSet<String> = services.iter().map(|s| s.prefix.clone()).collect();
    for d in &client.models {
        let ns = place(client, services, d);
        let mut class = pascal(&words(&d.r#ref.name), casings);
        if ns.is_empty() {
            if prefixes.contains(&class) {
                class = format!("Shared{class}");
            }
        } else if let Some(s) = services.iter().find(|s| s.model_ns == ns)
            && s.children.iter().any(|&c| services[c].prop == class)
        {
            class = format!("{}{class}", s.prefix);
        }
        named.insert(d.r#ref.schema.clone(), Named { ns, class });
    }
    let mut b = Builder {
        client,
        casings,
        named,
        namer: Namer::default(),
        root: target.ns.clone(),
    };
    // The top-level names first: params, responses and named models are files of their own.
    for (si, s) in services.iter().enumerate() {
        for (mi, m) in crate::emit::method_order(&s.resource.methods).into_iter().enumerate() {
            let method = pascal(&words(&m.name), casings);
            b.namer.reserve(&s.model_ns, &format!("{}{method}Params", s.prefix));
            if inline_response(m) {
                b.namer.reserve(&s.model_ns, &format!("{}{method}Response", s.prefix));
            }
            let _ = (si, mi);
        }
    }
    for n in b.named.values() {
        b.namer.reserve(&n.ns, &n.class);
    }
    // Pass 1 claims inline names in priority order; pass 2 builds every file with them.
    let mut order: Vec<&ModelDecl> = client.models.iter().collect();
    order.sort_by(|x, y| {
        let nx = &b.named[&x.r#ref.schema].class;
        let ny = &b.named[&y.r#ref.schema].class;
        nx.cmp(ny)
    });
    let mut files = Vec::new();
    for pass in 0..2 {
        files.clear();
        for (si, s) in services.iter().enumerate() {
            for (mi, m) in crate::emit::method_order(&s.resource.methods).into_iter().enumerate() {
                files.push(b.params_file(s, si, mi, m));
                if let Some(f) = b.response_file(s, si, mi, m) {
                    files.push(f);
                }
            }
        }
        for d in &order {
            if let Some(f) = b.model_file(d) {
                files.push(f);
            }
        }
        let _ = pass;
    }
    Files { files, named: b.named }
}

/// Whether a method's response is an inline object or union (so it has a file of its own).
fn inline_response(m: &ferrule_sdk_model::model::Method) -> bool {
    let Some(r) = &m.response else { return false };
    matches!(response_type(&r.ty).0, Type::Object(_) | Type::Union(_))
}

/// A response's element type and whether it is a list. An object with no properties (which
/// the model reads as a map of anything) is an empty model class, as Stainless writes it.
#[must_use]
pub fn response_type(ty: &Type) -> (Type, bool) {
    let (elem, list) = match ty {
        Type::Array { items } => ((**items).clone(), true),
        t => (t.clone(), false),
    };
    let elem = match elem {
        Type::Map { values }
            if matches!(
                *values,
                Type::Scalar {
                    scalar: ferrule_sdk_model::model::Scalar::Unknown
                }
            ) =>
        {
            Type::Object(Object::default())
        }
        t => t,
    };
    (elem, list)
}

/// The C# name of a property: `requestId` -> `RequestID`, renamed where it would clash with
/// a member of the class (`Url` of a params class) or the class itself.
fn prop_name(field: &Field, owner: &str, loc: Loc, casings: Casings<'_>) -> String {
    let name = pascal(&words(&field.name), casings);
    let clash = name == owner
        || (matches!(loc, Loc::Body | Loc::Query | Loc::Header | Loc::Path) && name == "Url")
        || name == "Value" && owner.is_empty();
    if clash { format!("{name}Value") } else { name }
}

/// A path param's C# type: `long` for an integer, else `string`.
#[must_use]
pub fn path_type(ty: &Type) -> CsType {
    match ty {
        Type::Scalar { scalar } if scalar.is_integer() => CsType::Long,
        _ => CsType::String,
    }
}

impl Builder<'_> {
    fn params_file(&mut self, s: &Svc<'_>, si: usize, mi: usize, m: &ferrule_sdk_model::model::Method) -> ModelFile {
        let method = pascal(&words(&m.name), self.casings);
        let name = format!("{}{method}Params", s.prefix);
        let key = format!("params:{}", m.pointer);
        let mut nested = Vec::new();
        let mut props = Vec::new();
        for p in &m.path_params {
            props.push(Prop {
                name: prop_name(&p.field, &name, Loc::Path, self.casings),
                wire: p.field.wire_name.clone(),
                ty: path_type(&p.field.ty),
                required: true,
                nullable: false,
                description: p.field.description.clone(),
                deprecated: p.field.deprecated,
                constant: None,
                loc: Loc::Path,
            });
        }
        let mut sections: Vec<Prop> = Vec::new();
        if let Some(body) = &m.body {
            if let Some(o) = self.client.object_of(&body.ty).cloned() {
                sections.extend(self.fields(&o, &key, &s.model_ns, &name, Loc::Body, &mut nested));
            } else {
                // A union or array body is one property named after its type.
                let field_key = format!("{key}/body");
                let ty = self.ty(
                    &body.ty,
                    &field_key,
                    &s.model_ns,
                    &["body".to_owned()],
                    &name,
                    &body.description,
                    &mut nested,
                );
                let description = match &body.ty {
                    Type::Ref(r) if body.description.is_empty() => {
                        self.client.model(r).map(|d| d.description.clone()).unwrap_or_default()
                    }
                    _ => body.description.clone(),
                };
                let prop_name = match &ty {
                    CsType::Class { name, .. } | CsType::Enum { name, .. } => name.clone(),
                    _ => "Body".to_owned(),
                };
                sections.push(Prop {
                    name: prop_name,
                    wire: String::new(),
                    ty,
                    required: body.required,
                    nullable: false,
                    description,
                    deprecated: false,
                    constant: None,
                    loc: Loc::WholeBody,
                });
            }
        }
        let mut query: Vec<Prop> = Vec::new();
        for p in &m.query_params {
            query.push(self.prop(&p.field, &key, &s.model_ns, &name, Loc::Query, &mut nested));
        }
        let mut headers: Vec<Prop> = Vec::new();
        for p in &m.header_params {
            headers.push(self.prop(&p.field, &key, &s.model_ns, &name, Loc::Header, &mut nested));
        }
        props.extend(sections);
        props.extend(order(query));
        props.extend(order(headers));
        let mut decls = vec![Decl::Object(Obj {
            name,
            description: if m.description.is_empty() {
                m.summary.clone()
            } else {
                m.description.clone()
            },
            props,
            bases: Vec::new(),
        })];
        decls.extend(nested);
        ModelFile {
            ns: s.model_ns.clone(),
            kind: Kind::Params(si, mi),
            decls,
        }
    }

    fn response_file(
        &mut self,
        s: &Svc<'_>,
        si: usize,
        mi: usize,
        m: &ferrule_sdk_model::model::Method,
    ) -> Option<ModelFile> {
        if !inline_response(m) {
            return None;
        }
        let (ty, _) = response_type(&m.response.as_ref()?.ty);
        let method = pascal(&words(&m.name), self.casings);
        let name = format!("{}{method}Response", s.prefix);
        let key = format!("response:{}", m.pointer);
        let mut decls = Vec::new();
        self.top_decl(&ty, &name, &key, &s.model_ns, "", &mut decls);
        Some(ModelFile {
            ns: s.model_ns.clone(),
            kind: Kind::Response(si, mi),
            decls,
        })
    }

    fn model_file(&mut self, d: &ModelDecl) -> Option<ModelFile> {
        let n = self.named.get(&d.r#ref.schema)?.clone();
        let key = format!("model:{}", d.r#ref.schema);
        let mut decls = Vec::new();
        match &d.ty {
            Type::Object(_) | Type::Union(_) => {
                self.top_decl(&d.ty, &n.class, &key, &n.ns, &d.description, &mut decls);
            }
            Type::Enum { members } if !is_const(members) => {
                decls.push(Decl::Enum(self.enum_decl(&n.class, members, &d.description)));
            }
            // Aliases of maps, arrays and scalars are spelled where they are used; the file holds
            // only the types inlined in them (the item class of a map of objects), if any.
            other => {
                self.ty(
                    other,
                    &key,
                    &n.ns,
                    &split_words(&n.class),
                    &n.class,
                    &d.description,
                    &mut decls,
                );
                if decls.is_empty() {
                    return None;
                }
            }
        }
        Some(ModelFile {
            ns: n.ns,
            kind: Kind::Model,
            decls,
        })
    }

    /// The top-level declaration of a file and the types inlined in it.
    fn top_decl(&mut self, ty: &Type, name: &str, key: &str, ns: &[String], description: &str, out: &mut Vec<Decl>) {
        let mut nested = Vec::new();
        match ty {
            Type::Object(o) => {
                let decl = self.object(o, name, key, ns, description, &mut nested);
                out.push(Decl::Object(decl.0));
                if let Some(m) = decl.1 {
                    out.push(Decl::Member(m));
                }
            }
            Type::Union(u) => {
                let un = self.union(u, name, key, ns, description, &mut nested);
                out.push(Decl::Union(un));
            }
            _ => {}
        }
        out.extend(nested);
    }

    /// An object's class and, when it extends named models with fields of its own, the class
    /// of its inline `allOf` member.
    fn object(
        &mut self,
        o: &Object,
        name: &str,
        key: &str,
        ns: &[String],
        description: &str,
        nested: &mut Vec<Decl>,
    ) -> (Obj, Option<Obj>) {
        let mut description = if description.is_empty() {
            o.description.clone().unwrap_or_default()
        } else {
            description.to_owned()
        };
        // An object that only extends named models is described by its first base.
        if description.is_empty()
            && let Some(m) = o.bases.first().and_then(|b| self.client.model(b))
        {
            description.clone_from(&m.description);
        }
        if o.bases.is_empty() {
            let props = self.fields(o, key, ns, name, Loc::Data, nested);
            return (
                Obj {
                    name: name.to_owned(),
                    description,
                    props,
                    bases: Vec::new(),
                },
                None,
            );
        }
        let mut props = Vec::new();
        let mut bases = Vec::new();
        for b in &o.bases {
            let base_props = self.base_props(b);
            let class = self.named.get(&b.schema).map(|n| n.class.clone()).unwrap_or_default();
            bases.push((class, base_props.iter().map(|p| p.name.clone()).collect()));
            props.extend(base_props);
        }
        if o.fields.is_empty() {
            return (
                Obj {
                    name: name.to_owned(),
                    description,
                    props,
                    bases,
                },
                None,
            );
        }
        let member_name = format!("{name}{name}");
        let own = self.fields(o, &format!("{key}/allOf"), ns, &member_name, Loc::Data, nested);
        props.extend(own.iter().cloned());
        (
            Obj {
                name: name.to_owned(),
                description,
                props,
                bases,
            },
            Some(Obj {
                name: member_name,
                description: String::new(),
                props: own,
                bases: Vec::new(),
            }),
        )
    }

    /// A named base's properties, with its inline types named as its own file names them.
    fn base_props(&mut self, base: &TypeRef) -> Vec<Prop> {
        let Some(decl) = self.client.model(base).cloned() else {
            return Vec::new();
        };
        let Some(name) = self.named.get(&base.schema).cloned() else {
            return Vec::new();
        };
        let Type::Object(o) = &decl.ty else { return Vec::new() };
        let mut scratch = Vec::new();
        let key = format!("model:{}", base.schema);
        let (obj, _) = self.object(o, &name.class, &key, &name.ns, &decl.description, &mut scratch);
        // A copied property that refers to a named model is described by that model, not by the
        // base's property.
        let client = self.client;
        obj.props
            .into_iter()
            .map(|mut p| {
                let field = o.fields.iter().find(|f| f.wire_name == p.wire);
                let target = field.and_then(|f| match &f.ty {
                    Type::Ref(r) => Some(r),
                    Type::Nullable { inner } => match &**inner {
                        Type::Ref(r) => Some(r),
                        _ => None,
                    },
                    _ => None,
                });
                if let Some(m) = target.and_then(|r| client.model(r)) {
                    p.description.clone_from(&m.description);
                }
                unqualify(p)
            })
            .collect()
    }

    /// An object's own fields as properties, required first, then constants, then optional.
    fn fields(
        &mut self,
        o: &Object,
        key: &str,
        ns: &[String],
        owner: &str,
        loc: Loc,
        nested: &mut Vec<Decl>,
    ) -> Vec<Prop> {
        let props: Vec<Prop> = o
            .fields
            .iter()
            .map(|f| self.prop(f, key, ns, owner, loc, nested))
            .collect();
        order(props)
    }

    fn prop(&mut self, f: &Field, key: &str, ns: &[String], owner: &str, loc: Loc, nested: &mut Vec<Decl>) -> Prop {
        let (inner, nullable) = match &f.ty {
            Type::Nullable { inner } => (&**inner, true),
            t => (t, false),
        };
        let constant = match inner {
            Type::Enum { members } if is_const(members) => Some(members[0].value.clone()),
            _ => None,
        };
        let field_key = format!("{key}/{}", f.wire_name);
        let ty = if constant.is_some() {
            CsType::Json
        } else {
            self.ty(inner, &field_key, ns, &words(&f.name), owner, &f.description, nested)
        };
        Prop {
            name: prop_name(f, owner, loc, self.casings),
            wire: f.wire_name.clone(),
            ty,
            required: f.required,
            nullable,
            description: f.description.clone(),
            deprecated: f.deprecated,
            constant,
            loc,
        }
    }

    /// The C# type of `ty` at `key`, declaring the types it inlines into `nested`.
    #[expect(
        clippy::too_many_arguments,
        reason = "the position of an inline type: key, namespace, words, owner"
    )]
    fn ty(
        &mut self,
        ty: &Type,
        key: &str,
        ns: &[String],
        field: &[String],
        owner: &str,
        description: &str,
        nested: &mut Vec<Decl>,
    ) -> CsType {
        match ty {
            Type::Scalar { scalar } => CsType::scalar(*scalar),
            Type::Nullable { inner } => self.ty(inner, key, ns, field, owner, description, nested),
            Type::Array { items } => {
                let item = singular(field);
                CsType::List(Box::new(self.ty(
                    items,
                    &format!("{key}[]"),
                    ns,
                    &item,
                    owner,
                    description,
                    nested,
                )))
            }
            Type::Map { values } => {
                let mut item = field.to_vec();
                item.push("item".to_owned());
                CsType::Dict(Box::new(self.ty(
                    values,
                    &format!("{key}{{}}"),
                    ns,
                    &item,
                    owner,
                    description,
                    nested,
                )))
            }
            Type::Enum { members } => {
                let short = pascal(field, self.casings);
                let name = self.namer.claim(key, ns, &short, owner);
                let e = self.enum_decl(&name, members, description);
                let value = e.value;
                nested.push(Decl::Enum(e));
                CsType::Enum {
                    name: self.qualify(&name, ns),
                    value,
                    ns: None,
                }
            }
            Type::Object(o) => {
                let short = pascal(field, self.casings);
                let name = self.namer.claim(key, ns, &short, owner);
                let mut inner = Vec::new();
                let (obj, member) = self.object(o, &name, key, ns, description, &mut inner);
                nested.push(Decl::Object(obj));
                if let Some(m) = member {
                    nested.push(Decl::Member(m));
                }
                nested.extend(inner);
                CsType::Class { name, ns: None }
            }
            Type::Union(u) => {
                let mut short = pascal(field, self.casings);
                // A union holds its variant in `Value`, so a union of that name takes its parent's.
                if short == "Value" {
                    short = format!("{owner}{short}");
                }
                let name = self.namer.claim(key, ns, &short, owner);
                let mut inner = Vec::new();
                let un = self.union(u, &name, key, ns, description, &mut inner);
                nested.push(Decl::Union(un));
                nested.extend(inner);
                CsType::Class { name, ns: None }
            }
            Type::Ref(r) => self.named_ty(r),
        }
    }

    /// A named model's type where it is used.
    fn named_ty(&mut self, r: &TypeRef) -> CsType {
        let Some(d) = self.client.model(r).cloned() else {
            return CsType::Json;
        };
        let (class, model_ns) = self
            .named
            .get(&r.schema)
            .map(|n| (n.class.clone(), n.ns.clone()))
            .unwrap_or_default();
        match &d.ty {
            Type::Object(_) | Type::Union(_) => CsType::Class {
                name: class,
                ns: Some(model_ns),
            },
            Type::Enum { members } if !is_const(members) => CsType::Enum {
                name: class,
                value: EnumValue::of(&members[0].value),
                ns: Some(model_ns),
            },
            // An alias is spelled as its type (its inline types are named in its own position).
            other => {
                let mut scratch = Vec::new();
                let key = format!("model:{}", r.schema);
                let ns = self.named.get(&r.schema).map(|n| n.ns.clone()).unwrap_or_default();
                self.ty(
                    other,
                    &key,
                    &ns,
                    &split_words(&class),
                    &class,
                    &d.description,
                    &mut scratch,
                )
            }
        }
    }

    fn enum_decl(&self, name: &str, members: &[ferrule_sdk_model::model::EnumMember], description: &str) -> En {
        let value = members.first().map_or(EnumValue::String, |m| EnumValue::of(&m.value));
        En {
            name: name.to_owned(),
            description: description.to_owned(),
            members: members
                .iter()
                .map(|m| (enum_member(&m.value, &m.name, self.casings), m.value.clone()))
                .collect(),
            value,
        }
    }

    fn union(
        &mut self,
        u: &Union,
        name: &str,
        key: &str,
        ns: &[String],
        description: &str,
        nested: &mut Vec<Decl>,
    ) -> Un {
        let mut variants = Vec::new();
        for (i, v) in u.variants.iter().enumerate() {
            let vkey = format!("{key}|{i}");
            let label_words: Vec<String> = match (&v.name, &v.ty) {
                (_, Type::Ref(r)) => {
                    split_words(&self.named.get(&r.schema).map(|n| n.class.clone()).unwrap_or_default())
                }
                (Some(n), _) => words(n),
                (None, Type::Scalar { scalar }) => split_words(&CsType::scalar(*scalar).spell()),
                (None, _) => vec![format!("variant{i}")],
            };
            let before = nested.len();
            let ty = self.ty(&v.ty, &vkey, ns, &label_words, name, "", nested);
            // A discriminated variant is named after its discriminator value.
            let label = match (&u.discriminator, &v.discriminator_value, &ty) {
                (Some(_), Some(value), _) => pascal(&split_words(value), self.casings),
                (_, _, CsType::Class { name, .. } | CsType::Enum { name, .. }) => name.clone(),
                (_, _, other) => pascal(&split_words(&other.spell()), self.casings),
            };
            let props = match &v.ty {
                Type::Ref(r) => self.variant_props(r),
                Type::Object(_) => nested[before..]
                    .iter_mut()
                    .find_map(|d| match d {
                        Decl::Object(o)
                            if CsType::Class {
                                name: o.name.clone(),
                                ns: None,
                            } == ty =>
                        {
                            // The union's `Value` holds the variant, so an inline variant's
                            // `value` is `ValueValue`, in the variant and lifted onto the union.
                            for p in &mut o.props {
                                if p.name == "Value" {
                                    "ValueValue".clone_into(&mut p.name);
                                }
                            }
                            Some(o.props.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            variants.push(UVariant {
                label,
                ty,
                discriminator: v.discriminator_value.iter().cloned().collect(),
                props,
            });
        }
        // A union with no description of its own is described by its first named variant.
        let mut description = description.to_owned();
        if description.is_empty()
            && let Some(Type::Ref(r)) = u.variants.first().map(|v| &v.ty)
            && let Some(d) = self.client.model(r)
        {
            description.clone_from(&d.description);
        }
        Un {
            name: name.to_owned(),
            description,
            variants,
            discriminator: u.discriminator.as_ref().map(|d| d.wire_name.clone()),
        }
    }

    /// The properties of a named object variant, as its own file declares them.
    fn variant_props(&mut self, r: &TypeRef) -> Vec<Prop> {
        let Some(d) = self.client.model(r).cloned() else {
            return Vec::new();
        };
        let Some(n) = self.named.get(&r.schema).cloned() else {
            return Vec::new();
        };
        let Type::Object(o) = &d.ty else { return Vec::new() };
        let mut scratch = Vec::new();
        let key = format!("model:{}", r.schema);
        let (obj, _) = self.object(o, &n.class, &key, &n.ns, &d.description, &mut scratch);
        obj.props.into_iter().map(unqualify).collect()
    }
}

impl Builder<'_> {
    /// How the declaring file spells an inline enum: one called `Type` is `global::` qualified,
    /// as it would otherwise mean `System.Type`.
    fn qualify(&self, name: &str, ns: &[String]) -> String {
        if name == "Type" {
            let mut q = format!("global::{}.Models", self.root);
            for s in ns {
                q.push('.');
                q.push_str(s);
            }
            format!("{q}.Type")
        } else {
            name.to_owned()
        }
    }
}

/// A base's property as a derived model spells it: inline types by their plain names, since the
/// derived file does not declare them.
fn unqualify(mut p: Prop) -> Prop {
    fn go(t: &mut CsType) {
        match t {
            CsType::Enum { name, .. } => {
                if let Some(rest) = name.strip_prefix("global::") {
                    *name = rest.rsplit('.').next().unwrap_or(rest).to_owned();
                }
            }
            CsType::List(t) | CsType::Dict(t) => go(t),
            _ => {}
        }
    }
    go(&mut p.ty);
    p
}

/// A single string value is a `JsonElement` constant; a single boolean is still an enum.
fn is_const(members: &[ferrule_sdk_model::model::EnumMember]) -> bool {
    members.len() == 1 && matches!(members[0].value, Literal::String(_))
}

/// An enum member's name: `COPY_FOLDER` -> `CopyFolder`, `240` -> `V240`, `true` -> `True`.
fn enum_member(value: &Literal, name: &ferrule_sdk_model::model::Name, casings: Casings<'_>) -> String {
    let text = match value {
        Literal::String(s) => s.clone(),
        Literal::Boolean(b) => b.to_string(),
        Literal::Number(n) => n.to_string(),
    };
    let mut w = split_words(&text);
    if w.is_empty() {
        w.clone_from(&name.words);
    }
    let out = pascal(&w, casings);
    match out.strip_prefix('_') {
        Some(rest) => format!("V{rest}"),
        None => out,
    }
}

/// Required properties first, then constants, then optional ones, each by wire name with `id`
/// first.
fn order(mut props: Vec<Prop>) -> Vec<Prop> {
    props.sort_by(|a, b| {
        (a.wire != "id")
            .cmp(&(b.wire != "id"))
            .then_with(|| ferrule_sdk_model::js::cmp(&a.wire.to_lowercase(), &b.wire.to_lowercase()))
            .then_with(|| ferrule_sdk_model::js::cmp(&a.wire, &b.wire))
    });
    let (consts, rest): (Vec<Prop>, Vec<Prop>) = props.into_iter().partition(|p| p.constant.is_some());
    let (required, optional): (Vec<Prop>, Vec<Prop>) = rest.into_iter().partition(|p| p.required);
    required.into_iter().chain(consts).chain(optional).collect()
}
