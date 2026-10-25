//! The JVM emitters (Stainless's Gradle layout, `java` and `kotlin` targets): the `-core`
//! module's `models/`, `services/{blocking,async}/`, `client/` and `core/ClientOptions.kt`, the
//! client-okhttp module's clients, the tests, and the Gradle and project files.

pub mod client;
pub mod model;
pub mod params;
pub mod project;
pub mod runtime;
pub mod service;
pub mod tests;
pub mod tmpl;

use ferrule_sdk_model::model::{Client, Method, Resource, Type};

use crate::meta::{Flavor, JvmTarget};
use crate::naming::{Casings, camel, package, pascal, singular, words};
use crate::types::Mapper;

/// Everything the emitters share about one SDK.
pub struct Gen<'a> {
    pub client: &'a Client,
    pub casings: Casings<'a>,
    pub target: &'a JvmTarget,
    pub mapper: Mapper<'a>,
    /// Every resource with methods or subresources, depth first in stainless.yml order.
    pub services: Vec<Service<'a>>,
}

/// A resource as a pair of services (blocking and async).
pub struct Service<'a> {
    pub resource: &'a Resource,
    /// `TripDetail` (`TripDetailService`, `TripDetailServiceAsync`).
    pub name: String,
    /// The prefix of its params and responses: the resource singular (`TripDetail`).
    pub prefix: String,
    /// Its accessor on the parent (`tripDetails`).
    pub accessor: String,
    /// The package path below `services.blocking` (`accounts` for `accounts.origins`), empty at
    /// the top level.
    pub service_pkg: Vec<String>,
    /// The package path below `models` (`accounts.origins`).
    pub model_pkg: Vec<String>,
    /// Indices of the subresources' services.
    pub children: Vec<usize>,
}

impl Service<'_> {
    /// The methods in Stainless's order.
    #[must_use]
    pub fn methods(&self) -> Vec<&Method> {
        method_order(&self.resource.methods)
    }
}

/// The schemas `ty` names, through inline types but not into named models.
fn refs(ty: &Type, out: &mut Vec<String>) {
    match ty {
        Type::Ref(r) => out.push(r.schema.clone()),
        Type::Array { items: t } | Type::Map { values: t } | Type::Nullable { inner: t } => refs(t, out),
        Type::Object(o) => {
            out.extend(o.bases.iter().map(|b| b.schema.clone()));
            for f in &o.fields {
                refs(&f.ty, out);
            }
        }
        Type::Union(u) => {
            for v in &u.variants {
                refs(&v.ty, out);
            }
        }
        Type::Scalar { .. } | Type::Enum { .. } => {}
    }
}

fn method_refs(m: &Method) -> Vec<String> {
    let mut out = Vec::new();
    for p in m.path_params.iter().chain(&m.query_params).chain(&m.header_params) {
        refs(&p.field.ty, &mut out);
    }
    for ty in [m.body.as_ref().map(|b| &b.ty), m.response.as_ref().map(|r| &r.ty)]
        .into_iter()
        .flatten()
    {
        refs(ty, &mut out);
    }
    out
}

/// Where each resource model is declared: the package of the first service (stainless.yml order)
/// whose methods use it, preferring one under the resources its path names. Shared models stay in
/// `models`.
///
/// `x-ferrule-models` writes Python's module for a resource model (`File` for `files.models.File`,
/// `accounts` for `accounts.origins.models.OriginRequest`), not its resource path, so the uses
/// place the model (Phase 1 gap list: model positions for non-Python targets).
fn place(client: &Client, services: &[Service<'_>]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for d in &client.models {
        let path: Vec<&str> = d.r#ref.path.iter().map(|n| n.written.as_str()).collect();
        if path.first() == Some(&"shared") {
            continue;
        }
        let declared = &path[..path.len().saturating_sub(1)];
        let users: Vec<&Service<'_>> = services
            .iter()
            .filter(|s| {
                s.resource
                    .methods
                    .iter()
                    .any(|m| method_refs(m).contains(&d.r#ref.schema))
            })
            .collect();
        let under = |s: &&&Service<'_>| {
            let keys: Vec<&str> = s.resource.path.iter().map(|n| n.written.as_str()).collect();
            keys.len() >= declared.len() && keys[..declared.len()] == *declared
        };
        if let Some(s) = users.iter().find(under).or_else(|| users.first()) {
            out.push((d.r#ref.schema.clone(), s.model_pkg.clone()));
        }
    }
    out
}

/// Standard methods come first in this order; the rest follow by name (as the oracles order).
const STANDARD: [&str; 5] = ["create", "retrieve", "update", "list", "delete"];

/// A resource's methods in Stainless's order.
#[must_use]
pub fn method_order(methods: &[Method]) -> Vec<&Method> {
    let rank = |m: &Method| {
        STANDARD
            .iter()
            .position(|s| *s == m.name.written)
            .unwrap_or(STANDARD.len())
    };
    let mut out: Vec<&Method> = methods.iter().collect();
    out.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a.name.written.cmp(&b.name.written))
            .then(a.order.cmp(&b.order))
    });
    out
}

impl<'a> Gen<'a> {
    #[must_use]
    pub fn new(client: &'a Client, target: &'a JvmTarget) -> Self {
        let casings = Casings(&client.casings);
        let mut g = Self {
            client,
            casings,
            target,
            mapper: Mapper {
                client,
                casings,
                root: &target.package,
                flavor: target.flavor,
                placement: Vec::new(),
            },
            services: Vec::new(),
        };
        for r in &client.resources {
            g.add_service(r, &[], &[]);
        }
        g.mapper.placement = place(client, &g.services);
        g
    }

    fn add_service(&mut self, r: &'a Resource, parent_svc: &[String], parent_models: &[String]) -> Option<usize> {
        if r.methods.is_empty() && r.subresources.is_empty() {
            return None;
        }
        let w = words(&r.name);
        let seg = package(&w);
        let mut model_pkg = parent_models.to_vec();
        model_pkg.push(seg.clone());
        let index = self.services.len();
        self.services.push(Service {
            resource: r,
            name: pascal(&singular(&w), self.casings),
            prefix: pascal(&singular(&w), self.casings),
            accessor: camel(&w, self.casings),
            service_pkg: parent_svc.to_vec(),
            model_pkg: model_pkg.clone(),
            children: Vec::new(),
        });
        let mut svc_pkg = parent_svc.to_vec();
        svc_pkg.push(seg);
        let mut children = Vec::new();
        for s in &r.subresources {
            if let Some(i) = self.add_service(s, &svc_pkg, &model_pkg) {
                children.push(i);
            }
        }
        self.services[index].children = children;
        Some(index)
    }

    #[must_use]
    pub fn java(&self) -> bool {
        self.target.flavor == Flavor::Java
    }

    /// The top-level services, in stainless.yml order.
    #[must_use]
    pub fn top_services(&self) -> Vec<usize> {
        let children: Vec<usize> = self.services.iter().flat_map(|s| s.children.clone()).collect();
        (0..self.services.len()).filter(|i| !children.contains(i)).collect()
    }

    /// The SDK's root package (`org.onebusaway`).
    #[must_use]
    pub fn root(&self) -> &str {
        &self.target.package
    }

    /// `org/onebusaway`.
    #[must_use]
    pub fn root_path(&self) -> String {
        self.target.package.replace('.', "/")
    }

    /// The main source directory of the core module.
    #[must_use]
    pub fn core_main(&self) -> String {
        format!("{}-core/src/main/kotlin/{}", self.target.artifact, self.root_path())
    }

    /// The test source directory of the core module.
    #[must_use]
    pub fn core_test(&self) -> String {
        format!("{}-core/src/test/kotlin/{}", self.target.artifact, self.root_path())
    }

    /// The package of a service's models: `org.onebusaway.models.agency`.
    #[must_use]
    pub fn models_package(&self, s: &Service<'_>) -> String {
        let mut p = format!("{}.models", self.root());
        for seg in &s.model_pkg {
            p.push('.');
            p.push_str(seg);
        }
        p
    }

    /// The package of a service: `org.onebusaway.services.blocking` (or `.async`) plus its
    /// parents.
    #[must_use]
    pub fn service_package(&self, s: &Service<'_>, async_: bool) -> String {
        let mut p = format!("{}.services.{}", self.root(), if async_ { "async" } else { "blocking" });
        for seg in &s.service_pkg {
            p.push('.');
            p.push_str(seg);
        }
        p
    }

    /// The name of a method's params class: `AgencyRetrieveParams`.
    #[must_use]
    pub fn params_name(&self, s: &Service<'_>, m: &Method) -> String {
        format!("{}{}Params", s.prefix, pascal(&words(&m.name), self.casings))
    }

    /// The name of a method's response class, when it declares one: `AgencyRetrieveResponse`.
    #[must_use]
    pub fn response_name(&self, s: &Service<'_>, m: &Method) -> String {
        format!("{}{}Response", s.prefix, pascal(&words(&m.name), self.casings))
    }

    /// A method's name: `retrieve`.
    #[must_use]
    pub fn method_name(&self, m: &Method) -> String {
        camel(&words(&m.name), self.casings)
    }

    /// `X-Stainless` style client class prefix: `OnebusawaySdk`.
    #[must_use]
    pub fn client_name(&self) -> &str {
        &self.target.client
    }
}

/// `a/b/` for the package segments `[a, b]` (empty for none).
#[must_use]
pub fn dir_prefix(segments: &[String]) -> String {
    segments.iter().fold(String::new(), |mut out, s| {
        out.push_str(s);
        out.push('/');
        out
    })
}

/// A method's doc text: its description, else its summary.
#[must_use]
pub fn method_doc(m: &Method) -> String {
    let d = m.description.trim();
    if d.is_empty() {
        m.summary.trim().to_owned()
    } else {
        d.to_owned()
    }
}
