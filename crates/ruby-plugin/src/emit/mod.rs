//! The Ruby emitters: `lib/<gem>/{models,resources}/**` with their `rbi/` and `sig/` views, the
//! client, `models.rb`, `version.rb`, `lib/<gem>.rb`, the tests, and the text of the runtime
//! template's slots.

pub mod client;
pub mod index;
pub mod model;
pub mod project;
pub mod resource;
pub mod runtime;
pub mod tests;

use ferrule_sdk_model::model::{Client, Method, Resource, Type};

use crate::meta::RubyTarget;
use crate::naming::{Casings, class_name, snake, words};
use crate::tree::{Builder, Decl, ModelFile, See, method_prefix, method_prefix_file};
use crate::types::{ClassRef, RType};

/// Everything the emitters share about one SDK.
pub struct Gen<'a> {
    pub client: &'a Client,
    pub target: &'a RubyTarget,
    /// The top-level module, `OnebusawaySDK`.
    pub m: String,
    pub builder: Builder<'a>,
    /// Every resource with a class, depth first in stainless.yml order.
    pub resources: Vec<Res<'a>>,
    /// Every file under `models/`, sorted by path.
    pub models: Vec<ModelFile>,
}

/// A resource as a Ruby class.
pub struct Res<'a> {
    pub resource: &'a Resource,
    /// `["CurrentTime"]`; a subresource nests in its parent (`["Files", "Versions"]`).
    pub class: Vec<String>,
    /// `current_time`, `files/versions`.
    pub file: String,
    /// The attribute on the client or the parent resource: `current_time`.
    pub attr: String,
    pub methods: Vec<Meth<'a>>,
}

impl Res<'_> {
    /// `OnebusawaySDK::Resources::CurrentTime`.
    #[must_use]
    pub fn path(&self, m: &str) -> String {
        format!("{m}::Resources::{}", self.class.join("::"))
    }

    /// The resources below this one, by index in [`Gen::resources`].
    #[must_use]
    pub fn depth(&self) -> usize {
        self.class.len() - 1
    }
}

/// A method of a resource.
pub struct Meth<'a> {
    pub method: &'a Method,
    /// `retrieve`.
    pub name: String,
    /// `ArrivalAndDepartureRetrieveParams`.
    pub params: ClassRef,
    /// The response as the method returns it; `None` without a JSON response.
    pub response: Option<RType>,
    /// Its arguments.
    pub args: crate::tree::Args,
    /// Its parameters class.
    pub class: crate::tree::Class,
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

impl<'a> Gen<'a> {
    #[must_use]
    pub fn new(client: &'a Client, target: &'a RubyTarget) -> Self {
        let builder = Builder::new(client);
        let mut g = Self {
            client,
            target,
            m: target.module.clone(),
            builder,
            resources: Vec::new(),
            models: Vec::new(),
        };
        let mut resources = Vec::new();
        for r in client.resources.iter().filter(|r| has_methods(r)) {
            g.collect(r, &[], &mut resources);
        }
        g.resources = resources;
        g.models = g.model_files();
        g
    }

    #[must_use]
    pub fn casings(&self) -> Casings<'a> {
        Casings(&self.client.casings)
    }

    fn collect(&self, r: &'a Resource, parent: &[String], out: &mut Vec<Res<'a>>) {
        let casings = self.casings();
        let mut class = parent.to_vec();
        class.push(class_name(&words(&r.name), casings));
        let file = r.path.iter().map(|n| snake(&words(n))).collect::<Vec<_>>().join("/");
        let prefix = method_prefix(Some(r), casings);
        let methods = method_order(&r.methods)
            .into_iter()
            .map(|m| {
                let mut segs = parent.to_vec();
                let name = snake(&words(&m.name));
                let method_class = class_name(&words(&m.name), casings);
                segs.push(format!("{prefix}{method_class}Params"));
                let params = ClassRef { segs, aliased: true };
                let response = m.response.as_ref().map(|resp| {
                    let mut segs = parent.to_vec();
                    segs.push(format!("{prefix}{method_class}Response"));
                    self.builder.response_type(&ClassRef { segs, aliased: false }, &resp.ty)
                });
                let see = See::Method {
                    resource: class.clone(),
                    method: name.clone(),
                };
                let (args, params_class) = self.builder.method_shape(params.clone(), m, see);
                Meth {
                    method: m,
                    name,
                    args,
                    class: params_class,
                    params,
                    response,
                }
            })
            .collect();
        out.push(Res {
            resource: r,
            attr: snake(&words(&r.name)),
            class: class.clone(),
            file,
            methods,
        });
        for s in r.subresources.iter().filter(|s| has_methods(s)) {
            self.collect(s, &class, out);
        }
    }

    /// The model files: named models, then each method's params and inline response.
    fn model_files(&self) -> Vec<ModelFile> {
        let mut out = Vec::new();
        for d in &self.client.models {
            if let Some(decl) = self.builder.model_decl(d) {
                let path = decl
                    .r()
                    .segs
                    .iter()
                    .map(|s| snake(&crate::naming::class_words(s)))
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(ModelFile { path, decl });
            }
        }
        for res in &self.resources {
            let path = &res.resource.path;
            let dir: String = path[..path.len().saturating_sub(1)]
                .iter()
                .fold(String::new(), |acc, n| acc + &snake(&words(n)) + "/");
            let prefix = method_prefix_file(Some(res.resource));
            for m in &res.methods {
                let see = See::Method {
                    resource: res.class.clone(),
                    method: m.name.clone(),
                };
                out.push(ModelFile {
                    path: format!("{dir}{prefix}_{}_params", m.name),
                    decl: Decl::Class(m.class.clone()),
                });
                if let Some(resp) = &m.method.response
                    && !matches!(resp.ty, Type::Ref(_))
                    && let Some(RType::Model(r) | RType::Enum(r) | RType::Union(r, _) | RType::Alias(r, _)) =
                        &m.response
                    && let Some(decl) = self.builder.response_decl(r.clone(), &resp.ty, see)
                {
                    out.push(ModelFile {
                        path: format!("{dir}{prefix}_{}_response", m.name),
                        decl,
                    });
                }
            }
        }
        out.sort_by(|a, b| ferrule_sdk_model::js::cmp(&a.path, &b.path));
        out
    }

    /// `lib/onebusaway_sdk`.
    #[must_use]
    pub fn lib(&self, view: View) -> String {
        format!("{}/{}", view.dir(), self.target.lib)
    }
}

/// One of the gem's three views of each file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// `lib/**.rb`.
    Rb,
    /// `rbi/**.rbi` (Sorbet).
    Rbi,
    /// `sig/**.rbs` (RBS).
    Rbs,
}

impl View {
    #[must_use]
    pub fn dir(self) -> &'static str {
        match self {
            Self::Rb => "lib",
            Self::Rbi => "rbi",
            Self::Rbs => "sig",
        }
    }

    #[must_use]
    pub fn ext(self) -> &'static str {
        match self {
            Self::Rb => "rb",
            Self::Rbi => "rbi",
            Self::Rbs => "rbs",
        }
    }

    pub const ALL: [Self; 3] = [Self::Rb, Self::Rbi, Self::Rbs];
}
