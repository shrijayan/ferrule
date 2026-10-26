//! The C# emitters: `src/<Ns>/Services/**` (an interface and a class per resource, each with a
//! raw-response view), `src/<Ns>/Models/**` (params, responses, named models), the client and
//! its interface, the generated parts of `Core/`, the tests, and the project files.

pub mod client;
pub mod example;
pub mod model;
pub mod model_union;
pub mod options;
pub mod project;
pub mod runtime;
pub mod service;
pub mod tests;

use std::collections::BTreeMap;

use ferrule_sdk_model::model::{Client, Method, Param, Resource, Type, TypeRef};

use crate::meta::CSharpTarget;
use crate::naming::{Casings, camel, camel_of, pascal, plural, singular, words};
use crate::tree::{self, Files};

/// Everything the emitters share about one SDK.
pub struct Gen<'a> {
    pub client: &'a Client,
    pub target: &'a CSharpTarget,
    /// Every resource with methods or subresources, depth first in stainless.yml order.
    pub services: Vec<Svc<'a>>,
    /// The model files and every declaration's name.
    pub files: Files,
}

/// A resource as a pair of services (the service and its raw-response view).
pub struct Svc<'a> {
    pub resource: &'a Resource,
    /// `File` (the resource key's last word singular): `FileService`, `FileUpdateParams`.
    pub prefix: String,
    /// `Files`: the property on the client or the parent service.
    pub prop: String,
    /// The service's namespace below `Services` (`["Accounts"]` for `accounts.origins`).
    pub service_ns: Vec<String>,
    /// The namespace below `Models` of its params and responses (`["Accounts", "Origins"]`).
    pub model_ns: Vec<String>,
    /// Indices of the subresources' services in [`Gen::services`].
    pub children: Vec<usize>,
    pub methods: Vec<Meth<'a>>,
}

/// A method of a service.
pub struct Meth<'a> {
    pub method: &'a Method,
    /// `Update`.
    pub name: String,
    /// `FileUpdateParams`.
    pub params: String,
    /// The response as the method returns it: `FileUpdateResponse`, `List<File>`; `None` for
    /// a method without a JSON response.
    pub response: Option<Response>,
    /// The path params, in path order.
    pub path_params: Vec<PathParam>,
    /// Whether the params have a required member other than the last path param.
    pub required_beyond_last_path_param: bool,
    /// Whether the params have any required member.
    pub required: bool,
}

/// A path param of a method.
#[derive(Debug, Clone, Default)]
pub struct PathParam {
    /// `FileID`: the params property.
    pub prop: String,
    /// `fileID`: the overload's parameter.
    pub arg: String,
    /// `string`, or `long` for an integer.
    pub ty: String,
}

/// A method's response type.
pub struct Response {
    /// The element type: `File`, `Models::SharedSavedExtension`.
    pub ty: String,
    /// Whether the method returns a `List<ty>`.
    pub list: bool,
    /// The local the raw-response method deserializes into: `file`, `originResponses`.
    pub var: String,
    /// The namespace the element type lives in (below `Models`), `None` when it is a namespace
    /// alias reference.
    pub ns: Option<Vec<String>>,
}

impl Response {
    /// `List<File>` or `File`.
    #[must_use]
    pub fn spell(&self) -> String {
        if self.list {
            format!("List<{}>", self.ty)
        } else {
            self.ty.clone()
        }
    }
}

/// Standard methods come first in this order; the rest follow by name (as the oracles order).
const STANDARD: [&str; 5] = ["create", "retrieve", "update", "list", "delete"];

/// Methods whose response local is named after the resource (`var file = ...`); the others use
/// `deserializedResponse`.
const NAMED_RESPONSES: [&str; 6] = ["create", "retrieve", "get", "update", "list", "delete"];

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

/// Whether a param is required (a path param always is).
fn required(p: &Param) -> bool {
    p.field.required
}

impl<'a> Gen<'a> {
    #[must_use]
    pub fn new(client: &'a Client, target: &'a CSharpTarget) -> Self {
        let casings = Casings(&client.casings);
        let mut services = Vec::new();
        for r in client.resources.iter().filter(|r| has_methods(r)) {
            collect(r, &[], casings, &mut services);
        }
        let files = tree::build(client, target, &services);
        let mut g = Self {
            client,
            target,
            services,
            files,
        };
        g.fill_methods();
        g
    }

    #[must_use]
    pub fn casings(&self) -> Casings<'a> {
        Casings(&self.client.casings)
    }

    /// The top-level services, in stainless.yml order.
    pub fn top_services(&self) -> impl Iterator<Item = &Svc<'a>> {
        self.services.iter().filter(|s| s.resource.path.len() == 1)
    }

    fn fill_methods(&mut self) {
        let casings = self.casings();
        for i in 0..self.services.len() {
            let s = &self.services[i];
            let methods = method_order(&s.resource.methods)
                .into_iter()
                .map(|m| {
                    let name = pascal(&words(&m.name), casings);
                    let params = format!("{}{name}Params", s.prefix);
                    let path_params: Vec<PathParam> = m
                        .path_params
                        .iter()
                        .map(|p| {
                            let w = words(&p.field.name);
                            PathParam {
                                prop: pascal(&w, casings),
                                arg: crate::naming::local(camel(&w, casings)),
                                ty: crate::tree::path_type(&p.field.ty).spell(),
                            }
                        })
                        .collect();
                    let others = m.query_params.iter().chain(&m.header_params).any(required)
                        || m.body
                            .as_ref()
                            .is_some_and(|b| body_required(self.client, &b.ty, b.required));
                    let required_beyond_last_path_param = others || m.path_params.len() > 1;
                    let response = m.response.as_ref().and_then(|r| self.response(s, m, &name, &r.ty));
                    Meth {
                        method: m,
                        name,
                        params,
                        response,
                        required: others || !m.path_params.is_empty(),
                        required_beyond_last_path_param,
                        path_params,
                    }
                })
                .collect();
            self.services[i].methods = methods;
        }
    }

    /// The response type of `m` and its local.
    fn response(&self, s: &Svc<'_>, m: &Method, name: &str, ty: &Type) -> Option<Response> {
        let casings = self.casings();
        let (elem, list) = tree::response_type(ty);
        let elem = &elem;
        let resource_var = || {
            let w = words(&s.resource.name);
            if list {
                camel(&plural(&w), casings)
            } else {
                camel(&singular(&w), casings)
            }
        };
        let named_method = NAMED_RESPONSES.contains(&m.name.written.as_str());
        match elem {
            Type::Ref(r) => {
                let decl = self.files.named(&r.schema)?;
                let shared = decl.ns.is_empty();
                let class = decl.class.clone();
                let model_words = words(&r.name);
                let var = if list {
                    camel(&plural(&model_words), casings)
                } else {
                    camel(&model_words, casings)
                };
                Some(Response {
                    ty: if shared && !s.model_ns.is_empty() {
                        format!("Models::{class}")
                    } else {
                        class
                    },
                    list,
                    // A named model's local is always named after it.
                    var,
                    ns: (!shared || s.model_ns.is_empty()).then(|| decl.ns.clone()),
                })
            }
            Type::Object(_) | Type::Union(_) => Some(Response {
                ty: format!("{}{name}Response", s.prefix),
                list,
                var: if named_method {
                    resource_var()
                } else {
                    "deserializedResponse".to_owned()
                },
                ns: Some(s.model_ns.clone()),
            }),
            _ => None,
        }
    }

    /// `Imagekit.Models.Files` for `["Files"]`.
    #[must_use]
    pub fn models_ns(&self, ns: &[String]) -> String {
        let mut out = format!("{}.Models", self.target.ns);
        for s in ns {
            out.push('.');
            out.push_str(s);
        }
        out
    }

    /// `Imagekit.Services.Accounts` for `["Accounts"]`.
    #[must_use]
    pub fn services_ns(&self, ns: &[String]) -> String {
        let mut out = format!("{}.Services", self.target.ns);
        for s in ns {
            out.push('.');
            out.push_str(s);
        }
        out
    }

    /// `IImageKitClient`.
    #[must_use]
    pub fn iclient(&self) -> String {
        format!("I{}Client", self.target.client)
    }
}

/// Whether a body makes the params required: an object with a required field, or any other
/// required body.
fn body_required(client: &Client, ty: &Type, required: bool) -> bool {
    match client.object_of(ty) {
        Some(o) => o.fields.iter().any(|f| f.required) || o.bases.iter().any(|b| base_required(client, b)),
        None => required,
    }
}

fn base_required(client: &Client, b: &TypeRef) -> bool {
    client
        .model(b)
        .and_then(|d| match &d.ty {
            Type::Object(o) => Some(o.fields.iter().any(|f| f.required)),
            _ => None,
        })
        .unwrap_or(false)
}

fn collect<'a>(r: &'a Resource, parent: &[String], casings: Casings<'_>, out: &mut Vec<Svc<'a>>) -> usize {
    let w = words(&r.name);
    let key = pascal(&w, casings);
    let mut model_ns = parent.to_vec();
    model_ns.push(key.clone());
    let index = out.len();
    out.push(Svc {
        resource: r,
        prefix: pascal(&singular(&w), casings),
        prop: key,
        service_ns: parent.to_vec(),
        model_ns: model_ns.clone(),
        children: Vec::new(),
        methods: Vec::new(),
    });
    let mut children = Vec::new();
    for s in r.subresources.iter().filter(|s| has_methods(s)) {
        children.push(collect(s, &model_ns, casings, out));
    }
    out[index].children = children;
    index
}

/// A text template embedded from `src/emit/*.txt`, without its first line (which credits the
/// oracle it comes from).
#[must_use]
pub fn template(text: &str) -> String {
    text.split_once('\n').map_or("", |(_, rest)| rest).to_owned()
}

/// `camelCase` of a class name, for locals.
#[must_use]
pub fn local_of(class: &str) -> String {
    crate::naming::local(camel_of(class))
}

/// `using` lines in the given order.
#[must_use]
pub fn usings(list: Vec<String>) -> String {
    let mut out = String::new();
    for u in list {
        out.push_str("using ");
        out.push_str(&u);
        out.push_str(";\n");
    }
    out
}

/// The files of one kind, by path.
pub type FileMap = BTreeMap<String, String>;
