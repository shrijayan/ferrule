//! Maps Forge's groups, methods, models and types to the neutral [`Client`] (ADR 0002, ADR 0017).
//! With [`super::extensions`], the only code that knows Forge's interface.
//!
//! The logic is the Python plugin's adapter, minus every spelling: names keep their words, wire
//! names and config spellings, and each language's namer spells them.

use std::collections::HashSet;

use forge::{
    Field as ForgeField, Forge, Group, HttpMethod as ForgeHttpMethod, Location, Method as ForgeMethod, Number,
    Object as ForgeObject, Scalar as ForgeScalar, Type as ForgeType,
};

use super::extensions::{MethodExtensions, RawClient, ResourcePosition, read_method_extensions, to_value};
use crate::diagnostics::{FerCode, FerruleError, operation_pointer};
use crate::js;
use crate::model::{
    Body, Client, ClientSettings, CustomCasing, Discriminator, DiscriminatorCase, EnumMember, Field, HttpMethod,
    Literal, Method, ModelDecl, Name, Object, Package, Pagination, Param, Resource, Response, Scalar, Streaming,
    StyleSwitch, TargetSettings, Type, TypeRef, Union, Variant,
};

/// The reserved resource name of methods on the client itself (ADR 0003).
pub const CLIENT_RESOURCE: &str = "$client";

/// The name written as `written`, with the words Forge splits it into.
#[must_use]
pub fn name(written: &str) -> Name {
    Name {
        words: forge::Name::new(written).words().to_vec(),
        written: written.to_owned(),
    }
}

/// A field's name: the words of `x-fern-property-name` or the wire name, written as sent.
fn field_name(field: ForgeField<'_>) -> Name {
    Name {
        words: field.name().words().to_vec(),
        written: field.wire_name().to_owned(),
    }
}

/// Generation state shared by every method converted into one [`Client`].
struct Context<'e> {
    /// Declarations in the order they were first referenced.
    decls: &'e mut Vec<ModelDecl>,
    /// Schema names being inlined, to reject recursive inline types.
    inlining: HashSet<String>,
    ext: &'e MethodExtensions,
    pointer: &'e str,
}

/// The `kind` of a Forge [`ForgeType`], for diagnostics.
fn forge_kind(t: ForgeType<'_>) -> &'static str {
    match t {
        ForgeType::Scalar(_) => "scalar",
        ForgeType::Enum(_) => "enum",
        ForgeType::Array(_) => "array",
        ForgeType::Object(_) => "object",
        ForgeType::Union(_) => "union",
        ForgeType::Intersection(_) => "intersection",
        ForgeType::Model(_) => "model",
    }
}

fn to_scalar(s: ForgeScalar) -> Scalar {
    match s {
        ForgeScalar::Unknown => Scalar::Unknown,
        ForgeScalar::Null => Scalar::Null,
        ForgeScalar::Boolean => Scalar::Boolean,
        ForgeScalar::String => Scalar::String,
        ForgeScalar::Uuid => Scalar::Uuid,
        ForgeScalar::DateTime => Scalar::DateTime,
        ForgeScalar::Date => Scalar::Date,
        ForgeScalar::Bytes => Scalar::Bytes,
        ForgeScalar::Binary => Scalar::Binary,
        ForgeScalar::Number(n) => match n {
            Number::Int => Scalar::Int,
            Number::Int8 => Scalar::Int8,
            Number::Int16 => Scalar::Int16,
            Number::Int32 => Scalar::Int32,
            Number::Int64 => Scalar::Int64,
            Number::Uint8 => Scalar::Uint8,
            Number::Uint16 => Scalar::Uint16,
            Number::Uint32 => Scalar::Uint32,
            Number::Uint64 => Scalar::Uint64,
            Number::Float => Scalar::Float,
            Number::Double => Scalar::Double,
        },
    }
}

fn to_literal(v: forge::EnumValue<'_>) -> EnumMember {
    let (value, text) = match v {
        forge::EnumValue::String(s) => (Literal::String(s.to_owned()), s.to_owned()),
        forge::EnumValue::Number(n) => (Literal::Number(n), n.to_string()),
        forge::EnumValue::Boolean(b) => (Literal::Boolean(b), b.to_string()),
    };
    EnumMember {
        value,
        name: name(&text),
    }
}

/// The value of `wire` in `ty` when it is a single-value string enum: what a discriminator says
/// for an object written in place.
fn single_value(ty: ForgeType<'_>, wire: &str) -> Option<String> {
    let ForgeType::Object(o) = ty else { return None };
    let field = o.fields().find(|f| f.wire_name() == wire)?;
    let ForgeType::Enum(e) = field.ty() else { return None };
    let mut values = e.values();
    match (values.next(), values.next()) {
        (Some(forge::EnumValue::String(s)), None) => Some(s.to_owned()),
        _ => None,
    }
}

impl Context<'_> {
    fn unsupported(&self, what: &str) -> FerruleError {
        FerruleError::new(
            FerCode::Fer201,
            self.pointer,
            format!("{what} is not generated yet"),
            "leave the operation out, or name the schema in x-ferrule-models",
        )
    }

    fn read_field(&mut self, field: ForgeField<'_>) -> Result<Field, FerruleError> {
        let ty = self.read_type(field.ty())?;
        Ok(Field {
            name: field_name(field),
            wire_name: field.wire_name().to_owned(),
            required: field.is_required(),
            deprecated: field.is_deprecated(),
            read_only: field.is_read_only(),
            description: field.description().to_owned(),
            default: field.default().map(to_value),
            ty: inline_described(ty, field),
        })
    }

    fn read_object(&mut self, obj: ForgeObject<'_>) -> Result<Type, FerruleError> {
        if obj.fields().len() == 0 {
            return Ok(match obj.extra() {
                // An object without properties and without other keys: the empty object.
                None => Type::Object(Object::default()),
                Some(extra) => Type::Map {
                    values: Box::new(self.read_type(extra)?),
                },
            });
        }
        let fields = obj.fields().map(|f| self.read_field(f)).collect::<Result<_, _>>()?;
        let extra = obj.extra().map(|e| self.read_type(e)).transpose()?;
        Ok(Type::Object(Object {
            fields,
            extra: extra.map(Box::new),
            ..Object::default()
        }))
    }

    /// `allOf`: named members become bases, object members contribute fields.
    fn read_intersection<'f>(&mut self, members: impl Iterator<Item = ForgeType<'f>>) -> Result<Object, FerruleError> {
        let mut out = Object::default();
        for member in members {
            match self.read_type(member)? {
                Type::Ref(r) => out.bases.push(r),
                Type::Object(o) => {
                    out.bases.extend(o.bases);
                    out.fields.extend(o.fields);
                    if out.extra.is_none() {
                        out.extra = o.extra;
                    }
                }
                // Members that give no shape a language can extend.
                Type::Scalar {
                    scalar: Scalar::Unknown | Scalar::Binary,
                } => {}
                t => return Err(self.unsupported(&format!("an allOf member of kind {}", t.kind()))),
            }
        }
        Ok(out)
    }

    fn read_union(&mut self, u: forge::Union<'_>) -> Result<Type, FerruleError> {
        let members: Vec<ForgeType<'_>> = u.members().collect();
        let discriminator = u.discriminator();
        let cases: Vec<(&str, ForgeType<'_>)> = discriminator.map(|d| d.cases().collect()).unwrap_or_default();
        // Forge puts null last (and never beside a discriminator): `T | null` is `nullable`.
        let nullable = members.len() >= 2 && matches!(members.last(), Some(ForgeType::Scalar(ForgeScalar::Null)));
        let shown = &members[..members.len() - usize::from(nullable)];
        let mut variants = Vec::with_capacity(shown.len());
        let mut mapping = Vec::new();
        for (index, &member) in shown.iter().enumerate() {
            let mut values: Vec<String> = match (discriminator, member) {
                (None, _) => Vec::new(),
                (Some(_), ForgeType::Model(m)) => cases
                    .iter()
                    .filter_map(|&(value, case)| match case {
                        ForgeType::Model(c) if c == m => Some(value.to_owned()),
                        _ => None,
                    })
                    .collect(),
                (Some(d), other) => single_value(other, d.wire_name()).into_iter().collect(),
            };
            let discriminator_value = values.first().cloned();
            values.sort_by(|a, b| js::cmp(a, b));
            mapping.extend(
                values
                    .into_iter()
                    .map(|value| DiscriminatorCase { value, variant: index }),
            );
            let variant_name = match member {
                ForgeType::Model(m) => Some(Name {
                    words: m.name().words().to_vec(),
                    written: m.schema_name().to_owned(),
                }),
                _ => discriminator_value.as_deref().map(name),
            };
            variants.push(Variant {
                name: variant_name,
                discriminator_value,
                ty: self.read_type(member)?,
            });
        }
        if nullable && variants.len() == 1 {
            let inner = variants.pop().map_or(Type::scalar(Scalar::Unknown), |v| v.ty);
            return Ok(Type::Nullable { inner: Box::new(inner) });
        }
        let union = Type::Union(Union {
            variants,
            discriminator: discriminator.map(|d| Discriminator {
                wire_name: d.wire_name().to_owned(),
                name: Name {
                    words: d.name().words().to_vec(),
                    written: d.wire_name().to_owned(),
                },
                mapping,
            }),
            exclusive: u.is_exclusive(),
        });
        Ok(if nullable {
            Type::Nullable { inner: Box::new(union) }
        } else {
            union
        })
    }

    fn read_type(&mut self, ty: ForgeType<'_>) -> Result<Type, FerruleError> {
        Ok(match ty {
            ForgeType::Scalar(s) => Type::scalar(to_scalar(s)),
            ForgeType::Enum(e) => Type::Enum {
                members: e.values().map(to_literal).collect(),
            },
            ForgeType::Array(a) => Type::Array {
                items: Box::new(self.read_type(a.elem())?),
            },
            ForgeType::Object(o) => self.read_object(o)?,
            ForgeType::Union(u) => self.read_union(u)?,
            ForgeType::Intersection(i) => Type::Object(self.read_intersection(i.members())?),
            ForgeType::Model(model) => {
                let schema = model.schema_name();
                if let Some(r) = self.ext.models.get(schema).cloned() {
                    if !self.decls.iter().any(|d| d.r#ref.schema == schema) {
                        // Reserve the slot first so a model that refers to itself terminates.
                        self.decls.push(ModelDecl {
                            r#ref: r.clone(),
                            description: model.description().to_owned(),
                            deprecated: model.is_deprecated(),
                            ty: Type::Object(Object::default()),
                            only: self.ext.model_only.get(schema).cloned(),
                        });
                        let body = self.read_type(model.ty())?;
                        if let Some(slot) = self.decls.iter_mut().find(|d| d.r#ref.schema == schema) {
                            slot.ty = body;
                        }
                    }
                    return Ok(Type::Ref(r));
                }
                if self.inlining.contains(schema) {
                    return Err(self.unsupported(&format!("recursive schema {schema} without x-ferrule-models")));
                }
                self.inlining.insert(schema.to_owned());
                let inlined = self.read_type(model.ty())?;
                self.inlining.remove(schema);
                match inlined {
                    Type::Object(o) => Type::Object(Object {
                        origin: Some(schema.to_owned()),
                        description: if model.description().is_empty() {
                            o.description
                        } else {
                            Some(model.description().to_owned())
                        },
                        ..o
                    }),
                    // The items of an array schema were inlined from it too, and Stainless names
                    // them after it (knock-mgmt `HeadersRequestTemplateHeadersArray`).
                    Type::Array { items } => match *items {
                        Type::Object(o) if o.origin.is_none() => Type::Array {
                            items: Box::new(Type::Object(Object {
                                origin: Some(schema.to_owned()),
                                ..o
                            })),
                        },
                        other => Type::Array { items: Box::new(other) },
                    },
                    other => other,
                }
            }
        })
    }

    fn read_param(&mut self, method: ForgeMethod<'_>, field: ForgeField<'_>) -> Result<Param, FerruleError> {
        Ok(Param {
            field: self.read_field(field)?,
            exploded: match method.location(field) {
                Some(Location::Query { exploded }) => Some(exploded),
                _ => None,
            },
        })
    }

    fn read_params(&mut self, method: ForgeMethod<'_>) -> Result<(Vec<Param>, Vec<Param>), FerruleError> {
        let mut path = Vec::new();
        let mut query = Vec::new();
        // Forge's request: parameters & body, the parameters alone, or (for a body that is not an
        // object, such as multipart content, which has no JSON type) the parameters and a `body`
        // field, or a body alone. Beside a body, only fields with a location are parameters.
        let (params, beside_body) = match (method.request(), method.body()) {
            (Some(ForgeType::Intersection(i)), Some(_)) => (i.members().next(), false),
            (Some(r @ ForgeType::Object(_)), Some(_)) => (Some(r), true),
            (request, None) => (request, false),
            _ => (None, false),
        };
        let Some(params) = params else { return Ok((path, query)) };
        let ForgeType::Object(obj) = params else {
            return Err(self.unsupported(&format!("parameters of kind {}", forge_kind(params))));
        };
        for field in obj.fields() {
            if beside_body && method.location(field).is_none() {
                continue;
            }
            let p = self.read_param(method, field)?;
            if method.location(field) == Some(Location::Path) {
                path.push(p);
            } else {
                query.push(p);
            }
        }
        // Path parameters in the order the path template names them.
        let at = |p: &Param| {
            method
                .path()
                .find(&format!("{{{}}}", p.field.wire_name))
                .map_or(-1, |i| i64::try_from(i).unwrap_or(i64::MAX))
        };
        path.sort_by_key(at);
        Ok((path, query))
    }

    /// The object `ty` is, following a reference to a model declared so far.
    fn object_of<'t>(&'t self, ty: Option<&'t Type>) -> Option<&'t Object> {
        match ty? {
            Type::Object(o) => Some(o),
            Type::Ref(r) => match &self.decls.iter().find(|d| d.r#ref == *r)?.ty {
                Type::Object(o) => Some(o),
                _ => None,
            },
            _ => None,
        }
    }

    fn to_pagination(&self, response: Option<&Type>) -> Result<Option<Pagination>, FerruleError> {
        let Some(raw) = &self.ext.pagination else {
            return Ok(None);
        };
        // Maps run spec field name -> purpose (converter contract); `items` marks the page items.
        let items_wire = raw
            .response
            .iter()
            .find(|p| p.purpose == "items")
            .map(|p| p.field.as_str());
        let items = self
            .object_of(response)
            .and_then(|o| o.fields.iter().find(|f| Some(f.wire_name.as_str()) == items_wire));
        let Some(Type::Array { items: item }) = items.map(|f| &f.ty) else {
            return Err(FerruleError::new(
                FerCode::Fer101,
                format!("{}/x-ferrule-pagination/response", self.pointer),
                format!(
                    "no array response field has purpose \"items\" (got {})",
                    items_wire.unwrap_or("none")
                ),
                "map the response property holding the page items to \"items\"",
            ));
        };
        Ok(Some(Pagination {
            scheme: raw.scheme,
            page: raw.page.clone(),
            request: raw.request.clone(),
            response: raw.response.clone(),
            param_location: raw.param_location,
            headers: raw.headers.clone(),
            item: (**item).clone(),
        }))
    }

    fn to_streaming(&self, response: Option<&Type>) -> Option<Streaming> {
        let raw = self.ext.streaming.as_ref()?;
        let event = match &raw.event_schema {
            None => response.cloned(),
            Some(schema) => self.ext.models.get(schema).cloned().map(Type::Ref),
        };
        Some(Streaming {
            format: raw.format,
            param_discriminator: raw.param_discriminator.clone(),
            event,
        })
    }

    fn read_method(&mut self, method: ForgeMethod<'_>) -> Result<Method, FerruleError> {
        let verb = match method.http_method() {
            ForgeHttpMethod::Get => HttpMethod::Get,
            ForgeHttpMethod::Put => HttpMethod::Put,
            ForgeHttpMethod::Post => HttpMethod::Post,
            ForgeHttpMethod::Delete => HttpMethod::Delete,
            ForgeHttpMethod::Options => HttpMethod::Options,
            ForgeHttpMethod::Head => HttpMethod::Head,
            ForgeHttpMethod::Patch => HttpMethod::Patch,
            ForgeHttpMethod::Trace => HttpMethod::Trace,
            ForgeHttpMethod::Query => HttpMethod::Query,
            ForgeHttpMethod::Connect => HttpMethod::Connect,
            other @ ForgeHttpMethod::Other(_) => {
                return Err(self.unsupported(&format!("HTTP method {}", other.as_str())));
            }
        };
        let (path_params, query_params) = self.read_params(method)?;
        let body = match method.body() {
            None => None,
            Some(b) => match b.json().or_else(|| b.contents().next()) {
                None => None,
                Some(c) => Some(Body {
                    media_type: c.media_type().to_owned(),
                    required: b.is_required(),
                    description: b.description().to_owned(),
                    schema_description: c.description().to_owned(),
                    ty: self.read_type(c.ty())?,
                }),
            },
        };
        let response = match method.responses().find_map(|r| r.json().map(|c| (r, c))) {
            None => None,
            Some((r, c)) => Some(Response {
                status: r.status().to_owned(),
                media_type: c.media_type().to_owned(),
                ty: self.read_type(c.ty())?,
            }),
        };
        // An event model the JSON response does not name (an always-streaming endpoint): declare
        // it from the streaming contents, such as `text/event-stream`.
        if let Some(schema) = self.ext.streaming.as_ref().and_then(|s| s.event_schema.as_deref())
            && !self.decls.iter().any(|d| d.r#ref.schema == schema)
        {
            for c in method.responses().flat_map(forge::Response::contents) {
                if !c.is_json() {
                    self.read_type(c.ty())?;
                }
            }
        }
        let header_params = match method.headers() {
            None => Vec::new(),
            Some(h) => h
                .fields()
                .map(|f| self.read_param(method, f))
                .collect::<Result<_, _>>()?,
        };
        let response_ty = response.as_ref().map(|r| &r.ty);
        let pagination = self.to_pagination(response_ty)?;
        let streaming = self.to_streaming(response_ty);
        Ok(Method {
            name: self.ext.method_name.clone(),
            pointer: self.pointer.to_owned(),
            http_method: verb,
            path: method.path().to_owned(),
            summary: method.summary().to_owned(),
            description: method.description().to_owned(),
            order: self.ext.method_order,
            deprecated: method.is_deprecated(),
            path_params,
            query_params,
            header_params,
            body,
            response,
            pagination,
            streaming,
            only: self.ext.only.clone(),
        })
    }
}

/// An object schema written inline under a property carries its description on the property
/// (Forge has no object descriptions); Stainless prints it as the type's doc comment too.
fn inline_described(ty: Type, field: ForgeField<'_>) -> Type {
    let description = field.description();
    if description.is_empty() {
        return ty;
    }
    match (field.ty(), ty) {
        (ForgeType::Object(_), Type::Object(o)) => Type::Object(Object {
            description: Some(description.to_owned()),
            ..o
        }),
        // A two-member union such as `T | null`: its object member.
        (ForgeType::Union(u), mut ty) if u.members().len() == 2 => {
            if let Some(i) = u.members().position(|m| matches!(m, ForgeType::Object(_))) {
                let slot = match &mut ty {
                    Type::Nullable { inner } if i == 0 => Some(&mut **inner),
                    Type::Union(un) => un.variants.get_mut(i).map(|v| &mut v.ty),
                    _ => None,
                };
                if let Some(Type::Object(o)) = slot {
                    o.description = Some(description.to_owned());
                }
            }
            ty
        }
        (_, ty) => ty,
    }
}

struct MutableResource {
    name: Name,
    path: Vec<Name>,
    order: usize,
    methods: Vec<Method>,
    children: Vec<MutableResource>,
    standalone: bool,
}

impl MutableResource {
    fn new(name: Name, path: Vec<Name>, order: usize) -> Self {
        Self {
            name,
            path,
            order,
            methods: Vec::new(),
            children: Vec::new(),
            standalone: false,
        }
    }

    /// The node at `written` (stainless.yml keys) below this one, made as needed. Keys with the
    /// same words are one resource, the first spelling kept.
    fn descend(&mut self, written: &[String], order: &[usize]) -> &mut Self {
        let mut node = self;
        for (i, w) in written.iter().enumerate() {
            let key = name(w);
            let pos = if let Some(pos) = node.children.iter().position(|c| c.name.words == key.words) {
                pos
            } else {
                let mut path = node.path.clone();
                path.push(key.clone());
                let child = Self::new(key, path, order.get(i).copied().unwrap_or(0));
                node.children.push(child);
                node.children.len() - 1
            };
            node = &mut node.children[pos];
        }
        node
    }
}

fn by_order<T>(mut items: Vec<T>, order: impl Fn(&T) -> usize) -> Vec<T> {
    items.sort_by_key(|t| order(t));
    items
}

fn freeze(r: MutableResource) -> Resource {
    Resource {
        name: r.name,
        path: r.path,
        order: r.order,
        methods: by_order(r.methods, |m| m.order),
        subresources: by_order(r.children.into_iter().map(freeze).collect(), |c| c.order),
        standalone: r.standalone,
    }
}

fn to_settings(raw: &RawClient) -> ClientSettings {
    ClientSettings {
        name: raw.name.clone(),
        env_prefix: raw.env_prefix.clone(),
        opts: raw.opts.clone(),
        environments: raw.environments.clone(),
        default_environment: raw.default_environment.clone(),
        security_schemes: raw.security_schemes.clone(),
        security: raw.security.clone(),
        timeout_seconds: raw.timeout_seconds,
        // `maxRetries` is any number in TypeScript; the model keeps a count, saturating.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        max_retries: raw.max_retries as u32,
        initial_retry_delay_seconds: raw.initial_retry_delay_seconds,
        max_retry_delay_seconds: raw.max_retry_delay_seconds,
        idempotency_header: raw.idempotency_header.clone(),
        default_headers: raw.default_headers.clone(),
        array_format: raw.array_format,
        nested_format: raw.nested_format,
    }
}

fn dotted(r: &TypeRef) -> String {
    r.path.iter().map(|n| n.written.as_str()).collect::<Vec<_>>().join(".")
}

/// Converts `methods` into one [`Client`]; `info` is `(version, description, base_url)`, empty
/// where Forge does not expose them.
fn to_client<'f>(
    methods: impl Iterator<Item = ForgeMethod<'f>>,
    info: (&str, &str, &str),
) -> Result<Option<Client>, FerruleError> {
    let mut root = MutableResource::new(Name::default(), Vec::new(), 0);
    let mut casings: Option<Vec<CustomCasing>> = None;
    let mut targets: Option<Vec<(String, TargetSettings)>> = None;
    let mut standalone: Vec<ResourcePosition> = Vec::new();
    let mut client_methods = Vec::new();
    let mut decls: Vec<ModelDecl> = Vec::new();
    let mut pkg: Option<Package> = None;
    let mut settings: Option<ClientSettings> = None;
    let mut style: Option<Vec<StyleSwitch>> = None;
    for method in methods {
        let pointer = operation_pointer(method.path(), method.http_method().as_str());
        let extensions: Vec<(&str, &forge::Json)> = method.extensions().collect();
        let ext = read_method_extensions(&extensions, &pointer)?;
        if pkg.as_ref().is_some_and(|p| *p != ext.package) {
            return Err(FerruleError::new(
                FerCode::Fer101,
                format!("{pointer}/x-ferrule-package"),
                "operations disagree on x-ferrule-package",
                "mirror the same root x-ferrule-config onto every operation",
            ));
        }
        pkg = Some(ext.package.clone());
        // Mirrored from the root, so the first operation's copy stands for all.
        if casings.is_none() {
            casings = Some(ext.casings.clone());
        }
        if style.is_none() {
            style = Some(ext.style.clone());
        }
        if targets.is_none() {
            targets = Some(ext.targets.clone());
        }
        if standalone.is_empty() {
            standalone.clone_from(&ext.standalone);
        }
        if settings.is_none()
            && let Some(client) = &ext.client
        {
            settings = Some(to_settings(client));
        }
        let mut ctx = Context {
            decls: &mut decls,
            inlining: HashSet::new(),
            ext: &ext,
            pointer: &pointer,
        };
        let converted = ctx.read_method(method)?;
        if ext.resource.first().is_some_and(|r| r == CLIENT_RESOURCE) {
            client_methods.push(converted);
            continue;
        }
        root.descend(&ext.resource, &ext.resource_order).methods.push(converted);
    }
    let Some(package) = pkg else { return Ok(None) };
    for (written, order) in &standalone {
        root.descend(written, order).standalone = true;
    }
    decls.sort_by(|a, b| js::cmp(&dotted(&a.r#ref), &dotted(&b.r#ref)));
    let (version, description, base_url) = info;
    Ok(Some(Client {
        package,
        targets: targets.unwrap_or_default(),
        version: version.to_owned(),
        description: description.to_owned(),
        base_url: base_url.to_owned(),
        settings,
        casings: casings.unwrap_or_default(),
        resources: freeze(root).subresources,
        client_methods: by_order(client_methods, |m| m.order),
        models: decls,
        style: style.unwrap_or_default(),
    }))
}

/// The whole document; `None` when it has no operations to read the package from.
///
/// # Errors
///
/// A FER diagnostic for a malformed extension or a spec the model cannot describe yet.
pub fn forge_to_model(forge: &Forge) -> Result<Option<Client>, FerruleError> {
    to_client(
        forge.groups().flat_map(Group::methods),
        (&forge.version, &forge.description, &forge.base_url),
    )
}

/// One group's own methods (not its children's).
///
/// # Errors
///
/// As [`forge_to_model`].
pub fn group_to_model(group: Group<'_>) -> Result<Option<Client>, FerruleError> {
    to_client(group.methods(), ("", "", ""))
}

/// [`group_to_model`] on any methods, for tests that combine methods of several documents.
///
/// # Errors
///
/// As [`forge_to_model`].
pub fn methods_to_model<'f>(methods: impl Iterator<Item = ForgeMethod<'f>>) -> Result<Option<Client>, FerruleError> {
    to_client(methods, ("", "", ""))
}
