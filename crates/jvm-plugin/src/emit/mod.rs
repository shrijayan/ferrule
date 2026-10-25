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

use ferrule_sdk_model::model::{Client, Method, Resource};

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

/// Standard methods come first in this order; the rest keep their stainless.yml order.
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
    out.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.order.cmp(&b.order)));
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
            },
            services: Vec::new(),
        };
        let mut top = Vec::new();
        for r in &client.resources {
            if let Some(i) = g.add_service(r, &[], &[]) {
                top.push(i);
            }
        }
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
