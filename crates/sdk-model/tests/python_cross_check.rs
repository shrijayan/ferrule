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
    /// `client.models` mapped, for the body shapes that look into a named model.
    models: Vec<py::SdkModelDecl>,
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

/// A body field as a keyword argument; Python's body params never carry `deprecated`.
fn field_param(f: &py::SdkField) -> py::SdkParam {
    py::SdkParam {
        wire_name: f.wire_name.clone(),
        py_name: f.py_name.clone(),
        required: f.required,
        description: f.description.clone(),
        ty: f.ty.clone(),
        deprecated: false,
        example: None,
        minimum: None,
        maximum: None,
    }
}

/// The fields of every variant once, in the order they first appear: required where every
/// variant requires them, typed as the first variant that has them types them.
fn merge_variants(variants: &[py::SdkBodyVariant]) -> Vec<py::SdkParam> {
    let mut out: Vec<py::SdkParam> = Vec::new();
    for v in variants {
        for p in &v.params {
            if !out.iter().any(|q| q.wire_name == p.wire_name) {
                let mut merged = p.clone();
                merged.required = variants
                    .iter()
                    .all(|w| w.params.iter().any(|q| q.wire_name == p.wire_name && q.required));
                out.push(merged);
            }
        }
    }
    out
}

/// The paths of the binary fields of a multipart body's object, by Python name: `["file"]`, or
/// `["files", "<array>"]` for an array of them.
fn file_paths(client: &n::Client, ty: &n::Type) -> Vec<Vec<String>> {
    let binary = |t: &n::Type| {
        matches!(
            t,
            n::Type::Scalar {
                scalar: n::Scalar::Binary
            }
        )
    };
    let Some(o) = client.object_of(ty) else {
        return Vec::new();
    };
    o.fields
        .iter()
        .filter_map(|f| {
            let name = snake_case(&f.name.words);
            match &f.ty {
                t if binary(t) => Some(vec![name]),
                n::Type::Array { items } if binary(items) => Some(vec![name, "<array>".to_owned()]),
                _ => None,
            }
        })
        .collect()
}

/// `client.style`'s Python switches over today's style; the other targets' are not Python's.
fn py_style(client: &n::Client) -> py::SdkStyle {
    let mut style = py::SdkStyle::default();
    for s in client.style.iter().filter(|s| s.target == "python") {
        let (slot, today) = match s.key.as_str() {
            "sentinel" => (&mut style.omit, "omit"),
            "resources" => (&mut style.lazy_resources, "lazy"),
            "paths" => (&mut style.path_template, "path_template"),
            "multipart" => (&mut style.deepcopy_with_paths, "deepcopy_with_paths"),
            other => panic!("Python has no style switch {other}"),
        };
        *slot = s.value == today;
    }
    style
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
            extra: o.extra.as_ref().map(|e| Box::new(self.ty(e))),
        }
    }

    /// Union members as Python variants: a member that is itself a union is spliced in, its
    /// discriminator dropped, as Stainless does.
    fn flat<'t>(&self, members: impl Iterator<Item = &'t n::Type>) -> Vec<py::SdkType> {
        let mut out = Vec::new();
        for m in members {
            match self.ty(m) {
                py::SdkType::Union { variants, .. } => out.extend(variants),
                other => out.push(other),
            }
        }
        out
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
                variants: self.flat(u.variants.iter().map(|v| &v.ty)),
                discriminator: u.discriminator.as_ref().map(|d| d.wire_name.clone()),
            },
            n::Type::Ref(r) => py::SdkType::Ref(Self::sdk_ref(r)),
            // `T | null` is a union with null last, `T` spliced in if it is a union itself.
            n::Type::Nullable { inner } => {
                let mut variants = self.flat(std::iter::once(&**inner));
                variants.push(py_null());
                py::SdkType::Union {
                    variants,
                    discriminator: None,
                }
            }
        }
    }

    fn decl(&self, r: &py::SdkRef) -> Option<&py::SdkModelDecl> {
        self.models.iter().find(|d| d.r#ref == *r)
    }

    /// The object whose fields are the body params: an object, the object of `T | null`, or a
    /// named class.
    fn object_of<'t>(&'t self, ty: &'t py::SdkType) -> Option<&'t py::SdkObject> {
        match ty {
            py::SdkType::Object(o) => Some(o),
            py::SdkType::Union { variants, .. } => {
                let mut rest = variants.iter().filter(|v| **v != py_null());
                match (rest.next(), rest.next()) {
                    (Some(only), None) if variants.len() == 2 => self.object_of(only),
                    _ => None,
                }
            }
            py::SdkType::Ref(r) => self.decl(r).filter(|d| d.alias.is_none()).map(|d| &d.ty),
            _ => None,
        }
    }

    /// The object variants of a union body, written inline or as a named model whose type is the
    /// union, with its discriminator; empty for any other body.
    fn body_variants(&self, body: &py::SdkType) -> (Vec<py::SdkBodyVariant>, Option<String>) {
        let union = match body {
            py::SdkType::Ref(r) => self.decl(r).and_then(|d| d.alias.as_ref()),
            other => Some(other),
        };
        let Some(py::SdkType::Union {
            variants,
            discriminator,
        }) = union
        else {
            return (Vec::new(), None);
        };
        let objects: Vec<&py::SdkObject> = variants
            .iter()
            .filter_map(|v| match v {
                py::SdkType::Object(o) => Some(o),
                _ => None,
            })
            .collect();
        if objects.len() < 2 || objects.len() != variants.len() {
            return (Vec::new(), None);
        }
        let out = objects
            .into_iter()
            .map(|o| py::SdkBodyVariant {
                object: o.clone(),
                params: o.fields.iter().map(field_param).collect(),
            })
            .collect();
        (out, discriminator.clone())
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
        let body = m.body.as_ref().map(|b| self.ty(&b.ty));
        let (body_variants, body_discriminator) = body.as_ref().map(|b| self.body_variants(b)).unwrap_or_default();
        let body_params = if body_variants.is_empty() {
            body.as_ref()
                .and_then(|b| self.object_of(b))
                .map(|o| o.fields.iter().map(field_param).collect())
                .unwrap_or_default()
        } else {
            merge_variants(&body_variants)
        };
        // A map or array body is one `body` argument, described by the body's schema, and so is
        // a raw-bytes body.
        let body_binary = m.body.as_ref().is_some_and(|b| {
            matches!(
                b.ty,
                n::Type::Scalar {
                    scalar: n::Scalar::Binary
                }
            )
        });
        let whole = match (&body, &m.body) {
            (Some(ty @ (py::SdkType::Map(_) | py::SdkType::Array(_))), Some(b)) => Some((ty, b)),
            (Some(ty), Some(b)) if body_binary => Some((ty, b)),
            _ => None,
        }
        .map(|(ty, b)| py::SdkParam {
            wire_name: "body".to_owned(),
            py_name: "body".to_owned(),
            required: b.required,
            description: b.schema_description.clone(),
            ty: ty.clone(),
            deprecated: false,
            example: None,
            minimum: None,
            maximum: None,
        });
        let body_whole = whole.is_some() && !body_binary;
        let body_params = whole.map_or(body_params, |p| vec![p]);
        let body_file_paths = m
            .body
            .as_ref()
            .filter(|b| b.media_type.eq_ignore_ascii_case("multipart/form-data"))
            .map(|b| file_paths(self.client, &b.ty))
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
            body,
            body_params,
            body_media_type: m.body.as_ref().map(|b| b.media_type.clone()),
            body_whole,
            body_binary,
            body_variants,
            body_discriminator,
            body_file_paths,
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
        name: snake_case(&naming::split_words(&o.name.written)),
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
    let mut p = Python {
        client,
        models: Vec::new(),
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
    // Declarations need no other declaration; methods look into them.
    p.models = client.models.iter().map(|d| p.model(d)).collect();
    let settings = client.settings.as_ref().map(|s| p.settings(s));
    let resource_tree: Vec<py::SdkResource> = client.resources.iter().map(|r| p.resource(r)).collect();
    let client_methods = client.client_methods.iter().map(|m| p.method(m)).collect();
    let mut models = p.models;
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
        client_methods,
        models,
        resource_tree,
        style: py_style(client),
    }
}

/// `Ok(true)` when the Python adapter accepts `json` and the derivation equals its model;
/// `Ok(false)` when the Python adapter rejects it.
fn cross_check(name: &str, json: &str) -> bool {
    let forge = common::forge(json);
    let expected = match ferrule_python_plugin::adapter::forge_to_model(&forge) {
        Ok(expected) => expected,
        Err(e) => {
            let neutral = ferrule_sdk_model::forge_to_model(&forge).map_or_else(|e| e.to_string(), |_| "ok".into());
            eprintln!("{name}: the Python adapter rejects it ({e}); the neutral adapter: {neutral}");
            return false;
        }
    };
    let neutral = ferrule_sdk_model::forge_to_model(&forge).unwrap_or_else(|e| panic!("{name}: {e}"));
    // Python generates the models stainless.yml keeps for it; the rest it inlines.
    let derived = neutral.map(|c| to_python(&c.for_target("python")));
    assert!(
        derived == expected,
        "{name}: the Python model derived from the neutral one differs: {}",
        derived
            .as_ref()
            .zip(expected.as_ref())
            .map_or_else(|| "one is None".to_owned(), |(d, e)| first_difference(d, e))
    );
    true
}

/// Every method of `c`, client methods first, then the resource tree depth first.
fn methods(c: &py::SdkClient) -> Vec<&py::SdkMethod> {
    fn walk<'a>(rs: &'a [py::SdkResource], out: &mut Vec<&'a py::SdkMethod>) {
        for r in rs {
            out.extend(&r.methods);
            walk(&r.subresources, out);
        }
    }
    let mut out: Vec<&py::SdkMethod> = c.client_methods.iter().collect();
    walk(&c.resource_tree, &mut out);
    out
}

/// Where `derived` first differs from `expected`, for the failure message.
fn first_difference(derived: &py::SdkClient, expected: &py::SdkClient) -> String {
    for (d, e) in derived.models.iter().zip(&expected.models) {
        if d != e {
            return format!("model {}.{}:\n{d:#?}\nexpected:\n{e:#?}", e.r#ref.module, e.r#ref.name);
        }
    }
    for (d, e) in methods(derived).into_iter().zip(methods(expected)) {
        if d != e {
            return format!("method {}:\n{d:#?}\nexpected:\n{e:#?}", e.pointer);
        }
    }
    let parts = [
        ("models", derived.models.len() == expected.models.len()),
        ("settings", derived.settings == expected.settings),
        ("style", derived.style == expected.style),
        ("resources", derived.resources == expected.resources),
        ("resource_tree", derived.resource_tree == expected.resource_tree),
    ];
    let differ: Vec<&str> = parts.iter().filter(|(_, same)| !same).map(|(p, _)| *p).collect();
    format!("in {}", differ.join(", "))
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

/// The generator-era fixture with each `x-ferrule-style` an adopted runtime can select, on every
/// operation as the converter mirrors it: the Python style is derived from the target's switches.
#[test]
fn derives_the_python_style_of_every_older_runtime() {
    let fixtures = common::python_fixtures();
    let (_, eras) = fixtures.iter().find(|(n, _)| n == "generator_eras").expect("fixture");
    let styles = [
        serde_json::json!({ "sentinel": "omit", "resources": "lazy", "paths": "path_template" }),
        serde_json::json!({ "resources": "eager", "paths": "fstring" }),
        serde_json::json!({ "sentinel": "NOT_GIVEN" }),
        serde_json::json!({
            "multipart": "deepcopy_minimal", "paths": "fstring", "resources": "eager", "sentinel": "NOT_GIVEN",
        }),
    ];
    for style in styles {
        let mut doc: serde_json::Value = serde_json::from_str(eras).expect("JSON");
        for item in doc["paths"].as_object_mut().expect("paths").values_mut() {
            for op in item.as_object_mut().expect("path item").values_mut() {
                op["x-ferrule-style"] = style.clone();
            }
        }
        assert!(cross_check(&format!("generator_eras {style}"), &doc.to_string()));
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
        cross_check(&pair.name, &json);
    }
}
