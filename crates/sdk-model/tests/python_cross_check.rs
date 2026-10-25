//! The neutral model is complete enough for Python: every field of the Python plugin's own
//! `SdkModel` can be derived from it, names re-spelled with the Python rules giving the same
//! identifiers. [`to_python`] is that derivation, and the mapping MIGRATION.md describes; the
//! tests compare it with the Python adapter's output on every fixture and cached config oracle.

mod common;

use ferrule_python_plugin::adapter::to_model::client_class_name_with;
use ferrule_python_plugin::model as py;
use ferrule_python_plugin::naming::{self, Casing as PyCasing, class_segment_with, pascal_case, singular, snake_case};
use ferrule_sdk_model::model as n;

/// The Python namer and type mapping over one neutral client.
struct Python<'a> {
    client: &'a n::Client,
    casings: Vec<(String, PyCasing)>,
}

/// `/^[a-z_][a-z0-9_]*$/`: a segment Python keeps as its module name.
fn module_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A stainless.yml key as Python spells a module or resource: `trip-details` -> `trip_details`.
fn py_key(written: &str) -> String {
    if module_name(written) {
        written.to_owned()
    } else {
        snake_case(&naming::split_words(written))
    }
}

fn py_scalar(s: n::Scalar) -> py::SdkType {
    use n::Scalar as S;
    let p = match s {
        S::Unknown | S::Binary => return py::SdkType::Unknown,
        S::Null => py::SdkPrimitive::Null,
        S::Boolean => py::SdkPrimitive::Boolean,
        S::String | S::Uuid | S::Bytes => py::SdkPrimitive::String,
        S::Date => py::SdkPrimitive::Date,
        S::DateTime => py::SdkPrimitive::Datetime,
        S::Float | S::Double => py::SdkPrimitive::Number,
        S::Int | S::Int8 | S::Int16 | S::Int32 | S::Int64 | S::Uint8 | S::Uint16 | S::Uint32 | S::Uint64 => {
            py::SdkPrimitive::Integer
        }
    };
    py::SdkType::Primitive(p)
}

fn py_literal(l: &n::Literal) -> py::SdkLiteral {
    match l {
        n::Literal::String(s) => py::SdkLiteral::String(s.clone()),
        n::Literal::Number(x) => py::SdkLiteral::Number(*x),
        n::Literal::Boolean(b) => py::SdkLiteral::Boolean(*b),
    }
}

fn py_null() -> py::SdkType {
    py::SdkType::Primitive(py::SdkPrimitive::Null)
}

impl Python<'_> {
    fn sdk_ref(r: &n::TypeRef) -> py::SdkRef {
        py::SdkRef {
            name: r.name.written.clone(),
            module: r.path.iter().map(|s| py_key(&s.written)).collect::<Vec<_>>().join("."),
        }
    }

    fn object(&self, o: &n::Object) -> py::SdkObject {
        py::SdkObject {
            bases: o.bases.iter().map(Self::sdk_ref).collect(),
            fields: o.fields.iter().map(|f| self.field(f)).collect(),
            description: o.description.clone(),
            hint: o.origin.clone(),
        }
    }

    fn ty(&self, t: &n::Type) -> py::SdkType {
        match t {
            n::Type::Scalar { scalar } => py_scalar(*scalar),
            n::Type::Enum { members } => py::SdkType::Literal(members.iter().map(|m| py_literal(&m.value)).collect()),
            n::Type::Array { items } => py::SdkType::Array(Box::new(self.ty(items))),
            n::Type::Map { values } => py::SdkType::Map(Box::new(self.ty(values))),
            // Python has no empty object: it is any value.
            n::Type::Object(o) if o.fields.is_empty() && o.bases.is_empty() => py::SdkType::Unknown,
            n::Type::Object(o) => py::SdkType::Object(self.object(o)),
            n::Type::Union(u) => py::SdkType::Union {
                variants: u.variants.iter().map(|v| self.ty(&v.ty)).collect(),
                discriminator: u.discriminator.as_ref().map(|d| d.wire_name.clone()),
            },
            n::Type::Ref(r) => py::SdkType::Ref(Self::sdk_ref(r)),
            // `T | null` is a union with null last; a discriminated union stays one member.
            n::Type::Nullable { inner } => {
                let variants = match &**inner {
                    n::Type::Union(u) if u.discriminator.is_none() => {
                        u.variants.iter().map(|v| self.ty(&v.ty)).chain([py_null()]).collect()
                    }
                    other => vec![self.ty(other), py_null()],
                };
                py::SdkType::Union {
                    variants,
                    discriminator: None,
                }
            }
        }
    }

    fn field(&self, f: &n::Field) -> py::SdkField {
        py::SdkField {
            wire_name: f.wire_name.clone(),
            py_name: snake_case(&f.name.words),
            required: f.required,
            description: f.description.clone(),
            ty: self.ty(&f.ty),
        }
    }

    fn param(&self, f: &n::Field, deprecated: bool) -> py::SdkParam {
        let field = self.field(f);
        py::SdkParam {
            wire_name: field.wire_name,
            py_name: field.py_name,
            required: field.required,
            description: field.description,
            ty: field.ty,
            deprecated,
            example: None,
            minimum: None,
            maximum: None,
        }
    }

    fn params(&self, ps: &[n::Param]) -> Vec<py::SdkParam> {
        ps.iter().map(|p| self.param(&p.field, p.field.deprecated)).collect()
    }

    fn pagination(&self, p: &n::Pagination) -> py::SdkPagination {
        let pairs = |ps: &[n::Purpose]| {
            ps.iter()
                .map(|p| (p.field.clone(), p.purpose.clone()))
                .collect::<Vec<_>>()
        };
        py::SdkPagination {
            scheme: match p.scheme {
                n::PaginationScheme::Cursor => py::PaginationScheme::Cursor,
                n::PaginationScheme::CursorId => py::PaginationScheme::CursorId,
                n::PaginationScheme::CursorUrl => py::PaginationScheme::CursorUrl,
                n::PaginationScheme::Offset => py::PaginationScheme::Offset,
                n::PaginationScheme::PageNumber => py::PaginationScheme::PageNumber,
                n::PaginationScheme::FakePage => py::PaginationScheme::FakePage,
            },
            page: pascal_case(&naming::split_words(&p.page.written)),
            request: pairs(&p.request),
            response: pairs(&p.response),
            param_location: match p.param_location {
                n::ParamLocation::Query => py::ParamLocation::Query,
                n::ParamLocation::Body => py::ParamLocation::Body,
            },
            headers: pairs(&p.headers),
            item: self.ty(&p.item),
        }
    }

    fn method(&self, m: &n::Method) -> py::SdkMethod {
        let body_params = m
            .body
            .as_ref()
            .and_then(|b| self.client.object_of(&b.ty))
            // Python's body params never carry `deprecated` (see the Python adapter).
            .map(|o| o.fields.iter().map(|f| self.param(f, false)).collect())
            .unwrap_or_default();
        py::SdkMethod {
            name: m.name.written.clone(),
            pointer: m.pointer.clone(),
            http_method: match m.http_method {
                n::HttpMethod::Get => py::HttpVerb::Get,
                n::HttpMethod::Post => py::HttpVerb::Post,
                n::HttpMethod::Put => py::HttpVerb::Put,
                n::HttpMethod::Patch => py::HttpVerb::Patch,
                n::HttpMethod::Delete => py::HttpVerb::Delete,
                other => panic!("Python has no {} method", other.as_str()),
            },
            path: m.path.clone(),
            summary: m.summary.clone(),
            description: m.description.clone(),
            order: m.order,
            deprecated: m.deprecated,
            path_params: self.params(&m.path_params),
            query_params: self.params(&m.query_params),
            header_params: self.params(&m.header_params),
            body: m.body.as_ref().map(|b| self.ty(&b.ty)),
            body_params,
            body_media_type: m.body.as_ref().map(|b| b.media_type.clone()),
            response: m.response.as_ref().map(|r| self.ty(&r.ty)),
            pagination: m.pagination.as_ref().map(|p| self.pagination(p)),
            streaming: m.streaming.as_ref().map(|s| py::SdkStreaming {
                format: match s.format {
                    n::StreamFormat::Sse => py::StreamFormat::Sse,
                    n::StreamFormat::Jsonl => py::StreamFormat::Jsonl,
                },
                param_discriminator: s.param_discriminator.clone(),
                event: s.event.as_ref().map(|e| self.ty(e)),
            }),
        }
    }

    fn resource(&self, r: &n::Resource) -> py::SdkResource {
        let path: Vec<String> = r.path.iter().map(|k| py_key(&k.written)).collect();
        let words = naming::split_words(path.last().map_or("", String::as_str));
        py::SdkResource {
            class_segment: Some(class_segment_with(&words, &self.casings)),
            singular_class_segment: Some(class_segment_with(&singular(&words), &self.casings)),
            path,
            order: r.order,
            methods: r.methods.iter().map(|m| self.method(m)).collect(),
            subresources: r.subresources.iter().map(|s| self.resource(s)).collect(),
            standalone: r.standalone,
        }
    }

    fn settings(&self, s: &n::ClientSettings) -> py::SdkClientSettings {
        py::SdkClientSettings {
            name: s.name.written.clone(),
            class_name: client_class_name_with(&s.name.written, &self.casings),
            env_prefix: s.env_prefix.clone(),
            opts: s.opts.iter().map(py_opt).collect(),
            environments: s
                .environments
                .iter()
                .map(|e| py::SdkEnvironment {
                    name: e.name.written.clone(),
                    url: e.url.clone(),
                })
                .collect(),
            security_schemes: s
                .security_schemes
                .iter()
                .map(|n| (n.name.clone(), py_scheme(&n.scheme)))
                .collect(),
            security: s.security.clone(),
            timeout_seconds: s.timeout_seconds,
            max_retries: s.max_retries,
            idempotency_header: s.idempotency_header.clone(),
            default_headers: s
                .default_headers
                .iter()
                .map(|h| (h.name.clone(), h.value.clone()))
                .collect(),
            array_format: match s.array_format {
                n::ArrayFormat::Comma => py::ArrayFormat::Comma,
                n::ArrayFormat::Repeat => py::ArrayFormat::Repeat,
                n::ArrayFormat::Indices => py::ArrayFormat::Indices,
                n::ArrayFormat::Brackets => py::ArrayFormat::Brackets,
            },
            nested_format: match s.nested_format {
                n::NestedFormat::Brackets => py::NestedFormat::Brackets,
                n::NestedFormat::Dots => py::NestedFormat::Dots,
            },
        }
    }

    fn model(&self, d: &n::ModelDecl) -> py::SdkModelDecl {
        let (ty, alias) = match self.ty(&d.ty) {
            py::SdkType::Object(o) => (o, None),
            // An object without properties: an empty class, as Stainless prints it.
            py::SdkType::Unknown => (py::SdkObject::default(), None),
            other => (py::SdkObject::default(), Some(other)),
        };
        py::SdkModelDecl {
            r#ref: Self::sdk_ref(&d.r#ref),
            description: d.description.clone(),
            ty,
            alias,
        }
    }
}

fn py_opt(o: &n::ClientOpt) -> py::SdkClientOpt {
    py::SdkClientOpt {
        name: o.name.written.clone(),
        ty: match o.ty {
            n::OptType::String => py::OptType::String,
            n::OptType::Number => py::OptType::Number,
            n::OptType::Integer => py::OptType::Integer,
            n::OptType::Boolean => py::OptType::Boolean,
        },
        nullable: o.nullable,
        default: o.default.clone(),
        read_env: o.read_env.clone(),
        description: o.description.clone(),
        auth: o.auth.as_ref().map(|a| py::SdkAuth {
            security_scheme: a.security_scheme.clone(),
            role: match a.role {
                n::AuthRole::Value => py::AuthRole::Value,
                n::AuthRole::Username => py::AuthRole::Username,
                n::AuthRole::Password => py::AuthRole::Password,
                n::AuthRole::ClientId => py::AuthRole::ClientId,
                n::AuthRole::ClientSecret => py::AuthRole::ClientSecret,
            },
        }),
        send: o.send.as_ref().map(|s| py::SdkSend {
            location: match s.location {
                n::SendLocation::Header => py::SendLocation::Header,
                n::SendLocation::Query => py::SendLocation::Query,
                n::SendLocation::Body => py::SendLocation::Body,
                n::SendLocation::Path => py::SendLocation::Path,
            },
            name: s.name.clone(),
        }),
    }
}

fn py_scheme(s: &n::SecurityScheme) -> py::SdkSecurityScheme {
    match s {
        n::SecurityScheme::Http { scheme, keyword } => py::SdkSecurityScheme::Http {
            scheme: match scheme {
                n::HttpAuthScheme::Bearer => py::HttpAuthScheme::Bearer,
                n::HttpAuthScheme::Digest => py::HttpAuthScheme::Digest,
                n::HttpAuthScheme::Basic => py::HttpAuthScheme::Basic,
            },
            keyword: keyword.clone(),
        },
        n::SecurityScheme::ApiKey { location, name } => py::SdkSecurityScheme::ApiKey {
            location: match location {
                n::ApiKeyLocation::Header => py::ApiKeyLocation::Header,
                n::ApiKeyLocation::Query => py::ApiKeyLocation::Query,
                n::ApiKeyLocation::Cookie => py::ApiKeyLocation::Cookie,
            },
            name: name.clone(),
        },
        n::SecurityScheme::OAuth2 => py::SdkSecurityScheme::OAuth2,
        n::SecurityScheme::OpenIdConnect => py::SdkSecurityScheme::OpenIdConnect,
    }
}

/// `tree` without the resources that have no methods, themselves or below.
fn with_methods(tree: &[py::SdkResource]) -> Vec<py::SdkResource> {
    tree.iter()
        .filter_map(|r| {
            let subresources = with_methods(&r.subresources);
            (!r.methods.is_empty() || !subresources.is_empty()).then(|| py::SdkResource {
                subresources,
                ..r.clone()
            })
        })
        .collect()
}

/// The Python plugin's `SdkClient`, derived from the neutral model alone.
fn to_python(client: &n::Client) -> py::SdkClient {
    let p = Python {
        client,
        casings: client
            .casings
            .iter()
            .map(|c| {
                let casing = match &c.casing {
                    n::Casing::Initialism(b) => PyCasing::Initialism(*b),
                    n::Casing::Pascal(s) => PyCasing::Pascal(s.clone()),
                };
                (c.word.clone(), casing)
            })
            .collect(),
    };
    let settings = client.settings.as_ref().map(|s| p.settings(s));
    let resource_tree: Vec<py::SdkResource> = client.resources.iter().map(|r| p.resource(r)).collect();
    let mut models: Vec<py::SdkModelDecl> = client.models.iter().map(|d| p.model(d)).collect();
    models.sort_by(|a, b| ferrule_python_plugin::js::cmp(&a.r#ref.module, &b.r#ref.module));
    py::SdkClient {
        package: py::SdkPackage {
            name: client.package.name.clone(),
            repo: client.package.repo.clone(),
        },
        base_url: client.base_url.clone(),
        client_class: settings.as_ref().map(|s| s.class_name.clone()),
        env_prefix: settings.as_ref().map(|s| s.env_prefix.clone()),
        settings,
        resources: with_methods(&resource_tree),
        client_methods: client.client_methods.iter().map(|m| p.method(m)).collect(),
        models,
        resource_tree,
    }
}

/// `Ok(true)` when the Python adapter accepts `json` and the derivation equals its model;
/// `Ok(false)` when the Python adapter rejects it.
fn cross_check(name: &str, json: &str) -> bool {
    let forge = common::forge(json);
    let Ok(expected) = ferrule_python_plugin::adapter::forge_to_model(&forge) else {
        return false;
    };
    let neutral = ferrule_sdk_model::forge_to_model(&forge).unwrap_or_else(|e| panic!("{name}: {e}"));
    let derived = neutral.as_ref().map(to_python);
    assert!(
        derived == expected,
        "{name}: the Python model derived from the neutral one differs"
    );
    true
}

#[test]
fn derives_the_python_model_of_onebusaway() {
    let fixtures = common::python_fixtures();
    let (_, oba) = fixtures.iter().find(|(n, _)| n == "onebusaway").expect("fixture");
    assert!(cross_check("onebusaway", oba));
}

#[test]
fn derives_the_python_model_of_every_python_fixture() {
    for (name, json) in common::python_fixtures() {
        assert!(cross_check(&name, &json), "{name}: the Python adapter rejects it");
    }
}

/// Every cached config oracle the Python adapter accepts (oracles/cache is gitignored, so CI
/// without it checks the fixtures only).
#[test]
fn derives_the_python_model_of_every_cached_config_oracle() {
    for pair in common::config_pairs() {
        let Some(json) = common::annotated(&pair) else {
            eprintln!("skipped {}: oracle files not cached", pair.name);
            continue;
        };
        if !cross_check(&pair.name, &json) {
            eprintln!("skipped {}: the Python adapter rejects it", pair.name);
        }
    }
}
