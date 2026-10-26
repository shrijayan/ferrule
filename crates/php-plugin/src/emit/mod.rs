//! The PHP emitters: the model classes, enums and unions under `src/<Resource>/**` and at the root,
//! the services (`src/Services/**`, each with a raw service) and their contracts
//! (`src/ServiceContracts/**`), the client, the version, the tests, the project files, and the
//! runtime template's options.

pub mod client;
pub mod file;
pub mod model;
pub mod project;
pub mod runtime;
pub mod service;
pub mod tests;

use std::collections::HashMap;

use ferrule_sdk_model::model::{Client, Method, Resource, Type, TypeRef};

use crate::meta::PhpTarget;
use crate::naming::{Casings, camel, class_name, singular, words};
use crate::tree::{Args, Builder, Decl, default_place};
use crate::types::{ClassPath, Imports, PType};

/// The four classes of a resource: (top namespace, suffix).
pub const SERVICE_CLASSES: [(&str, &str); 4] = [
    ("Services", "Service"),
    ("Services", "RawService"),
    ("ServiceContracts", "Contract"),
    ("ServiceContracts", "RawContract"),
];

/// Everything the emitters share about one SDK.
pub struct Gen<'a> {
    pub client: &'a Client,
    pub target: &'a PhpTarget,
    /// The root namespace, `Imagekit`.
    pub ns: String,
    /// Every resource with a service, depth first in stainless.yml order.
    pub resources: Vec<Res<'a>>,
    /// Every model file's declaration, sorted by path.
    pub decls: Vec<Decl>,
    /// The names of the classes in each namespace (below the root).
    pub names: HashMap<Vec<String>, Vec<String>>,
}

/// A resource as a service.
pub struct Res<'a> {
    pub resource: &'a Resource,
    /// `["Files", "Versions"]`: the namespace of its models and, below `Services`, of its service.
    pub segs: Vec<String>,
    /// The property on the client or the parent service: `versions`.
    pub attr: String,
    pub methods: Vec<Meth<'a>>,
    /// The subresources with a service, by index in [`Gen::resources`].
    pub children: Vec<usize>,
}

impl Res<'_> {
    /// `Versions`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.segs.last().map_or("", String::as_str)
    }

    /// `Services\Files\VersionsService` (and `...RawService`, `ServiceContracts\...Contract`,
    /// `...RawContract`).
    #[must_use]
    pub fn class(&self, top: &str, suffix: &str) -> ClassPath {
        let mut segs = vec![top.to_owned()];
        segs.extend(self.segs[..self.segs.len() - 1].iter().cloned());
        segs.push(format!("{}{suffix}", self.name()));
        ClassPath::new(segs)
    }
}

/// A method of a resource.
pub struct Meth<'a> {
    pub method: &'a Method,
    /// `getFromURL`.
    pub name: String,
    pub args: Args,
    /// The response as the method returns it; `None` without a JSON response.
    pub response: Option<PType>,
}

/// Standard methods come first in this order; the rest follow by name (as the oracles order).
const STANDARD: [&str; 5] = ["create", "retrieve", "update", "list", "delete"];

/// A resource's methods in Stainless's order.
#[must_use]
pub fn method_order(methods: &[Method]) -> Vec<&Method> {
    let rank = |m: &Method| {
        STANDARD
            .iter()
            .position(|&s| s == m.name.written)
            .unwrap_or(STANDARD.len())
    };
    let mut out: Vec<&Method> = methods.iter().collect();
    out.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| ferrule_sdk_model::js::cmp(&a.name.written, &b.name.written))
    });
    out
}

fn has_methods(r: &Resource) -> bool {
    !r.methods.is_empty() || r.subresources.iter().any(has_methods)
}

/// The prefix of a method's types: the resource's key with its last word singular
/// (`folders` -> `Folder`).
#[must_use]
pub fn method_prefix(r: &Resource, casings: Casings<'_>) -> String {
    class_name(&singular(&words(&r.name)), casings)
}

/// The class name a method's response is declared with: `create` is spelled `New`.
#[must_use]
pub fn response_word(method: &str, casings: Casings<'_>) -> String {
    if method == "create" {
        "New".to_owned()
    } else {
        class_name(&crate::naming::class_words(method), casings)
    }
}

/// Whether `ty` names the model `schema`, through inline types.
fn mentions(ty: &Type, schema: &str) -> bool {
    match ty {
        Type::Ref(r) => r.schema == schema,
        Type::Array { items: inner } | Type::Map { values: inner } | Type::Nullable { inner } => {
            mentions(inner, schema)
        }
        Type::Object(o) => {
            o.bases.iter().any(|b| b.schema == schema)
                || o.fields.iter().any(|f| mentions(&f.ty, schema))
                || o.extra.as_ref().is_some_and(|e| mentions(e, schema))
        }
        Type::Union(u) => u.variants.iter().any(|v| mentions(&v.ty, schema)),
        Type::Scalar { .. } | Type::Enum { .. } => false,
    }
}

/// Whether a method's parameters, body or response name the model `schema`.
fn names(m: &Method, schema: &str) -> bool {
    m.path_params
        .iter()
        .chain(&m.query_params)
        .any(|p| mentions(&p.field.ty, schema))
        || m.body.as_ref().is_some_and(|b| mentions(&b.ty, schema))
        || m.response.as_ref().is_some_and(|r| mentions(&r.ty, schema))
}

/// Whether the methods of `r` or of its subresources name `schema`.
fn uses(r: &Resource, schema: &str) -> bool {
    r.methods.iter().any(|m| names(m, schema)) || r.subresources.iter().any(|s| uses(s, schema))
}

/// Where each named model is declared: in the namespace of the resource that declares it.
///
/// The model records the module the converter gives it (Python's: a top-level resource's model
/// at the top, `accounts.OriginRequest` for a model of a subresource of `accounts`), not the
/// resource itself (Phase 1 gap list), so the resource is the first candidate whose methods use
/// the model: a top-level resource for a model at the top, else a subresource of the module's
/// resource. A model only another resource uses lands there (imagekit's `Folder`).
fn places(client: &Client, casings: Casings<'_>) -> HashMap<String, ClassPath> {
    let mut out = HashMap::new();
    for d in &client.models {
        let r: &TypeRef = &d.r#ref;
        let mut place = default_place(r, casings);
        let shared = r.path.first().is_some_and(|n| n.written == "shared");
        if !shared && !r.path.is_empty() {
            let parents = &r.path[..r.path.len() - 1];
            // The resource the module names, then its subresources (or the top-level resources).
            let mut owner: Option<&Resource> = None;
            let mut keys: Vec<&Resource> = Vec::new();
            let mut level: &[Resource] = &client.resources;
            for n in parents {
                let Some(found) = level.iter().find(|t| t.name.written == n.written) else {
                    break;
                };
                keys.push(found);
                owner = Some(found);
                level = &found.subresources;
            }
            if keys.len() == parents.len() {
                let candidates: &[Resource] = owner.map_or(&client.resources, |o| &o.subresources);
                if let Some(c) = candidates.iter().find(|c| uses(c, &r.schema)) {
                    keys.push(c);
                }
                let mut segs: Vec<String> = keys.iter().map(|k| class_name(&words(&k.name), casings)).collect();
                segs.push(place.name().to_owned());
                place = ClassPath::new(segs);
            }
        }
        out.insert(r.schema.clone(), place);
    }
    out
}

impl<'a> Gen<'a> {
    #[must_use]
    pub fn new(client: &'a Client, orig: &'a Client, target: &'a PhpTarget) -> Self {
        let casings = Casings(&client.casings);
        let mut builder = Builder::new(client, orig, places(client, casings));
        let mut g = Self {
            client,
            target,
            ns: target.namespace.clone(),
            resources: Vec::new(),
            decls: Vec::new(),
            names: HashMap::new(),
        };
        for d in &client.models {
            builder.model_decl(d);
        }
        let mut resources = Vec::new();
        for r in client.resources.iter().filter(|r| has_methods(r)) {
            g.collect(r, &[], &mut builder, &mut resources);
        }
        g.resources = resources;
        let mut decls: Vec<Decl> = Vec::new();
        for d in builder.decls {
            if !decls.iter().any(|e| e.path() == d.path()) {
                decls.push(d);
            }
        }
        decls.sort_by(|a, b| ferrule_sdk_model::js::cmp(&a.path().file(), &b.path().file()));
        g.decls = decls;
        let mut classes: Vec<ClassPath> = g.decls.iter().map(|d| d.path().clone()).collect();
        classes.push(ClassPath::new(vec!["Client".to_owned()]));
        for r in &g.resources {
            for (top, suffix) in SERVICE_CLASSES {
                classes.push(r.class(top, suffix));
            }
        }
        for c in classes {
            g.names
                .entry(c.namespace().to_vec())
                .or_default()
                .push(c.name().to_owned());
        }
        g
    }

    /// The imports of the file declaring `own` (a class in `namespace`).
    #[must_use]
    pub fn imports(&self, namespace: &[String], own: Option<&ClassPath>) -> Imports {
        let siblings = self.names.get(namespace).cloned().unwrap_or_default();
        Imports::new(&self.ns, namespace, own.cloned(), siblings)
    }

    #[must_use]
    pub fn casings(&self) -> Casings<'a> {
        Casings(&self.client.casings)
    }

    fn collect(&self, r: &'a Resource, parent: &[String], b: &mut Builder<'a>, out: &mut Vec<Res<'a>>) -> usize {
        let casings = self.casings();
        let mut segs = parent.to_vec();
        segs.push(class_name(&words(&r.name), casings));
        let prefix = method_prefix(r, casings);
        let index = out.len();
        out.push(Res {
            resource: r,
            attr: camel(&words(&r.name), casings),
            segs: segs.clone(),
            methods: Vec::new(),
            children: Vec::new(),
        });
        let service = {
            let mut s = vec!["Services".to_owned()];
            s.extend(segs[..segs.len() - 1].iter().cloned());
            s.push(format!("{}Service", segs[segs.len() - 1]));
            s.join("\\")
        };
        let mut methods = Vec::new();
        for m in method_order(&r.methods) {
            let name = camel(&words(&m.name), casings);
            let class = class_name(&words(&m.name), casings);
            let mut params = segs.clone();
            params.push(format!("{prefix}{class}Params"));
            let see = format!("{}\\{service}::{name}()", self.ns);
            let args = b.method_args(ClassPath::new(params), m, see);
            if let Some(c) = &args.params {
                b.decls.push(Decl::Class(c.clone()));
            }
            let response = m.response.as_ref().map(|resp| {
                let mut path = segs.clone();
                path.push(format!("{prefix}{}Response", response_word(&m.name.written, casings)));
                b.response_type(&ClassPath::new(path), &resp.ty)
            });
            methods.push(Meth {
                method: m,
                name,
                args,
                response,
            });
        }
        out[index].methods = methods;
        let mut children = Vec::new();
        for s in r.subresources.iter().filter(|s| has_methods(s)) {
            children.push(self.collect(s, &segs, b, out));
        }
        out[index].children = children;
        index
    }
}
