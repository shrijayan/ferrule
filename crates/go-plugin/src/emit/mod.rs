//! The Go emitters (Stainless Go v1 layout): one file per resource at the module root,
//! `client.go`, `aliases.go`, `shared/shared.go`, `api.md`, the tests, `internal/version.go`, and
//! the text for the runtime template's slots.

pub mod api_md;
pub mod client;
pub mod decls;
pub mod project;
pub mod resource;
pub mod runtime;
pub mod shared;
pub mod tests;

use std::collections::BTreeSet;

use ferrule_sdk_model::model::{Client, Method, ModelDecl, Param, Resource, Type};

use crate::meta::GoTarget;
use crate::naming::{Casings, exported, local, method_name, singular, words};
use crate::types::{Ctx, Pkg, model_pkg};

/// Everything the emitters share about one SDK.
pub struct Gen<'a> {
    pub client: &'a Client,
    pub casings: Casings<'a>,
    pub target: &'a GoTarget,
    /// `organization.name`, as the service docs name the API (`the onebusaway-sdk API`).
    pub org: String,
    pub services: Vec<Service<'a>>,
    /// The service (index into `services`) whose file declares each root-package model, by schema.
    pub placement: Vec<(String, usize)>,
}

/// The schemas `ty` references, in order.
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
    if let Some(b) = &m.body {
        refs(&b.ty, &mut out);
    }
    if let Some(r) = &m.response {
        refs(&r.ty, &mut out);
    }
    out
}

/// Where each root-package model is declared: the resource stainless.yml declares it under when
/// its path says so, else the first service whose methods reference it, else the first service.
///
/// `x-ferrule-models` writes Python's module for a resource model (`widget` for
/// `widgets.models.widget`), not the resource path, so the fallback is what places most models
/// today (Phase 1 gap list: model positions for non-Python targets).
fn place(client: &Client, services: &[Service<'_>]) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for d in client.models.iter().filter(|d| model_pkg(&d.r#ref) == Pkg::Root) {
        let path: Vec<&str> = d.r#ref.path.iter().map(|n| n.written.as_str()).collect();
        let by_path = services.iter().position(|s| {
            let keys: Vec<&str> = s.resource.path.iter().map(|n| n.written.as_str()).collect();
            path.len() == keys.len() + 1 && path[..keys.len()] == keys[..]
        });
        let by_use = || {
            services.iter().position(|s| {
                s.resource
                    .methods
                    .iter()
                    .any(|m| method_refs(m).contains(&d.r#ref.schema))
            })
        };
        if let Some(i) = by_path.or_else(by_use).or((!services.is_empty()).then_some(0)) {
            out.push((d.r#ref.schema.clone(), i));
        }
    }
    out
}

/// A resource as a Go service.
pub struct Service<'a> {
    pub resource: &'a Resource,
    /// `AgencyService`.
    pub service: String,
    /// The prefix of its types: each path segment singular (`UserFeed`, `TripDetail`).
    pub prefix: String,
    /// Its field on the parent (`TripDetails`, `Feeds`).
    pub field: String,
    /// `client.TripDetails`, `client.Users.Feeds`.
    pub accessor: String,
    /// `tripdetail` (`.go`, `_test.go`).
    pub file: String,
    /// Depth below the client (0 for a top-level resource).
    pub depth: usize,
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

fn collect<'a>(
    resources: &'a [Resource],
    parent: Option<(&str, &str)>,
    depth: usize,
    casings: Casings<'_>,
    out: &mut Vec<Service<'a>>,
) {
    for r in resources {
        let w = words(&r.name);
        let segment = exported(&singular(&w), casings);
        let prefix = parent.map_or_else(|| segment.clone(), |(p, _)| format!("{p}{segment}"));
        let field = exported(&w, casings);
        let accessor = parent.map_or_else(|| format!("client.{field}"), |(_, a)| format!("{a}.{field}"));
        out.push(Service {
            resource: r,
            service: format!("{prefix}Service"),
            file: prefix.to_lowercase(),
            prefix: prefix.clone(),
            field,
            accessor: accessor.clone(),
            depth,
        });
        collect(&r.subresources, Some((&prefix, &accessor)), depth + 1, casings, out);
    }
}

impl<'a> Gen<'a> {
    #[must_use]
    pub fn new(client: &'a Client, target: &'a GoTarget) -> Self {
        let casings = Casings(&client.casings);
        let mut services = Vec::new();
        collect(&client.resources, None, 0, casings, &mut services);
        let placement = place(client, &services);
        Self {
            client,
            casings,
            target,
            org: client
                .settings
                .as_ref()
                .map_or_else(|| target.package_name.clone(), |s| s.name.written.clone()),
            services,
            placement,
        }
    }

    /// The root-package models the file of `s` declares, in model order.
    #[must_use]
    pub fn models_of(&self, s: &Service<'_>) -> Vec<&'a ModelDecl> {
        let Some(index) = self.services.iter().position(|x| std::ptr::eq(x.resource, s.resource)) else {
            return Vec::new();
        };
        self.client
            .models
            .iter()
            .filter(|d| {
                self.placement
                    .iter()
                    .any(|(schema, i)| *schema == d.r#ref.schema && *i == index)
            })
            .collect()
    }

    /// A mapping context for a file of `pkg`.
    #[must_use]
    pub fn ctx(&self, pkg: Pkg) -> Ctx<'a> {
        Ctx {
            client: self.client,
            casings: self.casings,
            module: &self.target.module_path,
            pkg,
            imports: BTreeSet::new(),
        }
    }

    /// The service of a resource.
    #[must_use]
    pub fn service_of(&self, r: &Resource) -> Option<&Service<'a>> {
        self.services.iter().find(|s| std::ptr::eq(s.resource, r))
    }

    /// The method the README and the usage test call: `meta.example`, else the first method
    /// without required parameters.
    #[must_use]
    pub fn example(&self) -> Option<(&Service<'a>, &'a Method)> {
        if let Some((path, method)) = &self.target.example {
            for s in &self.services {
                let keys: Vec<&str> = s.resource.path.iter().map(|n| n.written.as_str()).collect();
                if keys == *path
                    && let Some(m) = s.resource.methods.iter().find(|m| m.name.written == *method)
                {
                    return Some((s, m));
                }
            }
        }
        self.services.iter().find_map(|s| {
            method_order(&s.resource.methods)
                .into_iter()
                .find(|m| m.path_params.is_empty() && params(m).iter().all(|p| !p.required))
                .map(|m| (s, m))
        })
    }

    #[must_use]
    pub fn method_name(&self, m: &Method) -> String {
        method_name(&m.name, self.casings)
    }

    /// `ctx` param names and Go types for each path param.
    #[must_use]
    pub fn path_params(&self, m: &Method) -> Vec<(String, &'static str)> {
        m.path_params
            .iter()
            .map(|p| (local(&words(&p.field.name), self.casings), path_param_type(&p.field.ty)))
            .collect()
    }
}

/// The Go type of a path parameter.
#[must_use]
pub fn path_param_type(ty: &Type) -> &'static str {
    match ty {
        Type::Scalar { scalar } if scalar.is_integer() => "int64",
        Type::Scalar {
            scalar: ferrule_sdk_model::model::Scalar::Float | ferrule_sdk_model::model::Scalar::Double,
        } => "float64",
        Type::Scalar {
            scalar: ferrule_sdk_model::model::Scalar::Boolean,
        } => "bool",
        _ => "string",
    }
}

/// A method's query and header params (body fields are in [`body_fields`]).
#[must_use]
pub fn params(m: &Method) -> Vec<&ferrule_sdk_model::model::Field> {
    m.query_params
        .iter()
        .chain(&m.header_params)
        .map(|p: &Param| &p.field)
        .collect()
}

/// The fields of a method's object body, if it has one.
#[must_use]
pub fn body_fields<'c>(client: &'c Client, m: &'c Method) -> Vec<&'c ferrule_sdk_model::model::Field> {
    m.body
        .as_ref()
        .and_then(|b| client.object_of(&b.ty))
        .map(|o| o.fields.iter().collect())
        .unwrap_or_default()
}

/// The name of a method's params argument: `query`, `body` or `params` (both, or headers).
#[must_use]
pub fn params_arg(client: &Client, m: &Method) -> Option<&'static str> {
    let query = !m.query_params.is_empty();
    let body = m.body.is_some() && !body_fields(client, m).is_empty();
    let headers = !m.header_params.is_empty();
    match (query, body, headers) {
        (false, false, false) => None,
        (true, false, false) => Some("query"),
        (false, true, false) => Some("body"),
        _ => Some("params"),
    }
}
