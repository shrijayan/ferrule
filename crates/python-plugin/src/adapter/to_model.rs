//! Maps Forge's groups, methods, models and types to [`SdkClient`] (ADR 0002). With
//! [`super::extensions`], the only code that knows Forge's interface.

use std::collections::HashSet;

use forge::{Field, Forge, Group, Method, Number, Object, Scalar, Type};

use super::extensions::{MethodExtensions, RawClient, ResourcePosition, read_method_extensions};
use crate::diagnostics::{FerCode, FerruleError, operation_pointer};
use crate::js;
use crate::model::{
    HttpVerb, SdkBodyVariant, SdkClient, SdkClientSettings, SdkField, SdkLiteral, SdkMethod, SdkModelDecl, SdkObject,
    SdkPackage, SdkPagination, SdkParam, SdkPrimitive, SdkRef, SdkResource, SdkStreaming, SdkStyle, SdkType,
};
use crate::naming::{Casing, class_segment_with, model_attribute, pascal_case, singular, snake_case, split_words};

/// The reserved resource name of methods on the client class itself (ADR 0003).
pub const CLIENT_RESOURCE: &str = "$client";

/// Generation state shared by every method converted into one [`SdkClient`].
struct Context<'e> {
    /// Schema name -> declaration, in the order they were first referenced.
    decls: &'e mut Vec<(String, SdkModelDecl)>,
    /// Schema names being inlined, to reject recursive inline types.
    inlining: HashSet<String>,
    ext: &'e MethodExtensions,
    pointer: &'e str,
}

/// The TypeScript `kind` of an [`SdkType`], for diagnostics.
fn sdk_kind(t: &SdkType) -> &'static str {
    match t {
        SdkType::Primitive(_) => "primitive",
        SdkType::Unknown => "unknown",
        SdkType::Array(_) => "array",
        SdkType::Map(_) => "map",
        SdkType::Union { .. } => "union",
        SdkType::Literal(_) => "literal",
        SdkType::Ref(_) => "ref",
        SdkType::Object(_) => "object",
    }
}

/// The TypeScript `kind` of a Forge [`Type`], for diagnostics.
fn forge_kind(t: Type<'_>) -> &'static str {
    match t {
        Type::Scalar(_) => "scalar",
        Type::Enum(_) => "enum",
        Type::Array(_) => "array",
        Type::Object(_) => "object",
        Type::Union(_) => "union",
        Type::Intersection(_) => "intersection",
        Type::Model(_) => "model",
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

    fn ref_for(&self, schema_name: &str) -> Option<SdkRef> {
        let named = self.ext.models.get(schema_name)?;
        Some(SdkRef {
            name: named.name.clone(),
            module: named
                .module
                .clone()
                .unwrap_or_else(|| snake_case(&split_words(&named.name))),
        })
    }

    fn read_field(&mut self, field: Field<'_>) -> Result<SdkField, FerruleError> {
        let ty = self.read_type(field.ty())?;
        Ok(SdkField {
            wire_name: field.wire_name().to_owned(),
            py_name: snake_case(field.name().words()),
            required: field.is_required(),
            description: field.description().to_owned(),
            ty: inline_described(ty, field),
        })
    }

    fn read_param(&mut self, field: Field<'_>) -> Result<SdkParam, FerruleError> {
        let f = self.read_field(field)?;
        Ok(SdkParam {
            wire_name: f.wire_name,
            py_name: f.py_name,
            required: f.required,
            description: f.description,
            ty: f.ty,
            deprecated: field.is_deprecated(),
            example: None,
            minimum: None,
            maximum: None,
        })
    }

    fn read_object(&mut self, obj: Object<'_>) -> Result<SdkType, FerruleError> {
        if obj.fields().len() == 0 {
            return Ok(match obj.extra() {
                None => SdkType::Unknown,
                Some(extra) => SdkType::Map(Box::new(self.read_type(extra)?)),
            });
        }
        let fields = obj.fields().map(|f| self.read_field(f)).collect::<Result<_, _>>()?;
        let extra = match obj.extra() {
            Some(extra) => Some(Box::new(self.read_type(extra)?)),
            None => None,
        };
        Ok(SdkType::Object(SdkObject {
            fields,
            extra,
            ..SdkObject::default()
        }))
    }

    /// `allOf`: named members become base classes, object members contribute fields.
    fn read_intersection<'f>(&mut self, members: impl Iterator<Item = Type<'f>>) -> Result<SdkObject, FerruleError> {
        let mut out = SdkObject::default();
        for member in members {
            match self.read_type(member)? {
                SdkType::Ref(r) => out.bases.push(r),
                SdkType::Object(o) => {
                    out.bases.extend(o.bases);
                    out.fields.extend(o.fields);
                }
                SdkType::Unknown => {}
                t => return Err(self.unsupported(&format!("an allOf member of kind {}", sdk_kind(&t)))),
            }
        }
        Ok(out)
    }

    fn read_type(&mut self, ty: Type<'_>) -> Result<SdkType, FerruleError> {
        Ok(match ty {
            Type::Scalar(s) => to_scalar(s),
            Type::Enum(e) => SdkType::Literal(
                e.values()
                    .map(|v| match v {
                        forge::EnumValue::String(s) => SdkLiteral::String(s.to_owned()),
                        forge::EnumValue::Number(n) => SdkLiteral::Number(n),
                        forge::EnumValue::Boolean(b) => SdkLiteral::Boolean(b),
                    })
                    .collect(),
            ),
            Type::Array(a) => SdkType::Array(Box::new(self.read_type(a.elem())?)),
            Type::Object(o) => self.read_object(o)?,
            Type::Union(u) => SdkType::Union {
                variants: u.members().map(|m| self.read_type(m)).collect::<Result<_, _>>()?,
                discriminator: u.discriminator().map(|d| d.wire_name().to_owned()),
            },
            Type::Intersection(i) => SdkType::Object(self.read_intersection(i.members())?),
            Type::Model(model) => {
                let schema = model.schema_name();
                if let Some(r) = self.ref_for(schema) {
                    if !self.decls.iter().any(|(k, _)| k == schema) {
                        // Reserve the slot first so a model that refers to itself terminates.
                        let decl = SdkModelDecl {
                            r#ref: r.clone(),
                            description: model.description().to_owned(),
                            ty: SdkObject::default(),
                            alias: None,
                        };
                        self.decls.push((schema.to_owned(), decl));
                        let body = self.read_type(model.ty())?;
                        if let Some(clash) = attribute_clash(&body) {
                            let pointer =
                                format!("#/components/schemas/{}", schema.replace('~', "~0").replace('/', "~1"));
                            return Err(clash_error(&pointer, &clash));
                        }
                        if let Some((_, slot)) = self.decls.iter_mut().find(|(k, _)| k == schema) {
                            match body {
                                SdkType::Object(o) => slot.ty = o,
                                // An object without properties: an empty class, as Stainless prints it.
                                SdkType::Unknown => {}
                                other => slot.alias = Some(other),
                            }
                        }
                    }
                    return Ok(SdkType::Ref(r));
                }
                if self.inlining.contains(schema) {
                    return Err(self.unsupported(&format!("recursive schema {schema} without x-ferrule-models")));
                }
                self.inlining.insert(schema.to_owned());
                let inlined = self.read_type(model.ty())?;
                self.inlining.remove(schema);
                match inlined {
                    SdkType::Object(o) => SdkType::Object(SdkObject {
                        hint: Some(schema.to_owned()),
                        description: if model.description().is_empty() {
                            o.description
                        } else {
                            Some(model.description().to_owned())
                        },
                        ..o
                    }),
                    // The items of an array schema take its name where Stainless names them after
                    // it: as a union member (knock-mgmt `HeadersRequestTemplateHeadersArray`).
                    SdkType::Array(items) => match *items {
                        SdkType::Object(o) if o.hint.is_none() => {
                            SdkType::Array(Box::new(SdkType::Object(SdkObject {
                                hint: Some(schema.to_owned()),
                                ..o
                            })))
                        }
                        other => SdkType::Array(Box::new(other)),
                    },
                    other => other,
                }
            }
        })
    }

    fn read_params(&mut self, method: Method<'_>) -> Result<(Vec<SdkParam>, Vec<SdkParam>), FerruleError> {
        let mut path = Vec::new();
        let mut query = Vec::new();
        let request = method.request();
        // Forge's request: parameters & body, the parameters alone, or (for a body that is not an
        // object, such as multipart content, which has no JSON type) the parameters and a `body`
        // field, or a body alone. Beside a body, only fields with a location are parameters.
        let (params, beside_body) = match (request, method.body()) {
            (Some(Type::Intersection(i)), Some(_)) => (i.members().next(), false),
            (Some(r @ Type::Object(_)), Some(_)) => (Some(r), true),
            (request, None) => (request, false),
            _ => (None, false),
        };
        let Some(params) = params else { return Ok((path, query)) };
        let Type::Object(obj) = params else {
            return Err(self.unsupported(&format!("parameters of kind {}", forge_kind(params))));
        };
        for field in obj.fields() {
            if beside_body && method.location(field).is_none() {
                continue;
            }
            let p = self.read_param(field)?;
            if method.location(field) == Some(forge::Location::Path) {
                path.push(p);
            } else {
                query.push(p);
            }
        }
        // Path parameters in the order the path template names them.
        let at = |p: &SdkParam| {
            method
                .path()
                .find(&format!("{{{}}}", p.wire_name))
                .map_or(-1, |i| i64::try_from(i).unwrap_or(i64::MAX))
        };
        path.sort_by_key(at);
        Ok((path, query))
    }

    fn object_of<'t>(&'t self, ty: Option<&'t SdkType>) -> Option<&'t SdkObject> {
        match ty? {
            SdkType::Object(o) => Some(o),
            // `T | null` (FastAPI's optional body, honcho `workspaces.list`): T's fields.
            SdkType::Union { variants, .. } => {
                let mut rest = variants
                    .iter()
                    .filter(|v| !matches!(v, SdkType::Primitive(SdkPrimitive::Null)));
                match (rest.next(), rest.next()) {
                    (Some(only), None) if variants.len() == 2 => self.object_of(Some(only)),
                    _ => None,
                }
            }
            SdkType::Ref(r) => self
                .decls
                .iter()
                .find(|(_, d)| d.r#ref.module == r.module && d.r#ref.name == r.name && d.alias.is_none())
                .map(|(_, d)| &d.ty),
            _ => None,
        }
    }

    /// The object variants of a union body, written inline or as a named model whose schema is
    /// the union (imagekit `OriginRequest`), with the discriminator; empty for any other body.
    fn union_variants(&self, ty: Option<&SdkType>) -> (Vec<SdkBodyVariant>, Option<String>) {
        let union = match ty {
            Some(SdkType::Ref(r)) => self
                .decls
                .iter()
                .find(|(_, d)| d.r#ref == *r)
                .and_then(|(_, d)| d.alias.as_ref()),
            other => other,
        };
        let Some(SdkType::Union {
            variants,
            discriminator,
        }) = union
        else {
            return (Vec::new(), None);
        };
        let objects: Vec<&SdkObject> = variants
            .iter()
            .filter_map(|v| match v {
                SdkType::Object(o) => Some(o),
                _ => None,
            })
            .collect();
        if objects.len() < 2 || objects.len() != variants.len() {
            return (Vec::new(), None);
        }
        let out = objects
            .into_iter()
            .map(|o| SdkBodyVariant {
                object: o.clone(),
                params: o.fields.iter().map(field_param).collect(),
            })
            .collect();
        (out, discriminator.clone())
    }

    fn to_pagination(&self, response: Option<&SdkType>) -> Result<Option<SdkPagination>, FerruleError> {
        let Some(raw) = &self.ext.pagination else {
            return Ok(None);
        };
        // Maps run spec field name -> purpose (converter contract); `items` marks the page items.
        let items_wire = raw
            .response
            .iter()
            .find(|(_, purpose)| purpose == "items")
            .map(|(w, _)| w.as_str());
        let items = self
            .object_of(response)
            .and_then(|o| o.fields.iter().find(|f| Some(f.wire_name.as_str()) == items_wire));
        let Some(SdkType::Array(item)) = items.map(|f| &f.ty) else {
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
        Ok(Some(SdkPagination {
            scheme: raw.scheme,
            page: pascal_case(&split_words(&raw.page)),
            request: raw.request.clone(),
            response: raw.response.clone(),
            param_location: raw.param_location,
            headers: raw.headers.clone(),
            item: (**item).clone(),
        }))
    }

    fn to_streaming(&self, response: Option<&SdkType>) -> Option<SdkStreaming> {
        let raw = self.ext.streaming.as_ref()?;
        let event = match &raw.event_schema {
            None => response.cloned(),
            Some(schema) => self.ref_for(schema).map(SdkType::Ref),
        };
        Some(SdkStreaming {
            format: raw.format,
            param_discriminator: raw.param_discriminator.clone(),
            event,
        })
    }

    fn read_method(&mut self, method: Method<'_>) -> Result<SdkMethod, FerruleError> {
        let http = method.http_method().as_str();
        let verb = match http.to_lowercase().as_str() {
            "get" => HttpVerb::Get,
            "post" => HttpVerb::Post,
            "put" => HttpVerb::Put,
            "patch" => HttpVerb::Patch,
            "delete" => HttpVerb::Delete,
            _ => return Err(self.unsupported(&format!("HTTP method {http}"))),
        };
        let (path_params, query_params) = self.read_params(method)?;
        let content = method.body().and_then(|b| b.json().or_else(|| b.contents().next()));
        let body = content.map(|c| self.read_type(c.ty())).transpose()?;
        let response_type = method
            .responses()
            .find_map(forge::Response::json)
            .map(forge::Content::ty);
        let response = response_type.map(|t| self.read_type(t)).transpose()?;
        if let Some(clash) = response.as_ref().and_then(attribute_clash) {
            return Err(clash_error(self.pointer, &clash));
        }
        let header_params = match method.headers() {
            None => Vec::new(),
            Some(h) => h.fields().map(|f| self.read_param(f)).collect::<Result<_, _>>()?,
        };
        let (body_variants, body_discriminator) = self.union_variants(body.as_ref());
        let body_params = if body_variants.is_empty() {
            self.object_of(body.as_ref())
                .map(|o| o.fields.iter().map(field_param).collect())
                .unwrap_or_default()
        } else {
            merge_variants(&body_variants)
        };
        // A map or array body is one `body` argument, described by the body schema (Stainless).
        let whole = match (&body, content) {
            (Some(ty @ (SdkType::Map(_) | SdkType::Array(_))), Some(c)) => Some(SdkParam {
                wire_name: "body".to_owned(),
                py_name: "body".to_owned(),
                required: method.body().is_some_and(forge::Body::is_required),
                description: c.description().to_owned(),
                ty: ty.clone(),
                deprecated: false,
                example: None,
                minimum: None,
                maximum: None,
            }),
            _ => None,
        };
        let body_whole = whole.is_some();
        let body_params = whole.map_or(body_params, |p| vec![p]);
        let pagination = self.to_pagination(response.as_ref())?;
        let streaming = self.to_streaming(response.as_ref());
        Ok(SdkMethod {
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
            body_params,
            body_media_type: content.map(|c| c.media_type().to_owned()),
            body_whole,
            body_variants,
            body_discriminator,
            body_file_paths: content
                .filter(|c| c.media_type().eq_ignore_ascii_case("multipart/form-data"))
                .map(|c| file_paths(c.ty()))
                .unwrap_or_default(),
            response,
            pagination,
            streaming,
        })
    }
}

/// The paths of the binary fields of a multipart body's object, by Python name: `["file"]`, or
/// `["files", "<array>"]` for an array of them.
/// The first two properties of a model class that would be one attribute ([`model_attribute`]):
/// their wire names and the attribute. Inline objects are looked into; named models are checked
/// where they are declared.
fn attribute_clash(ty: &SdkType) -> Option<(String, String, String)> {
    match ty {
        SdkType::Object(o) => {
            let mut seen: Vec<(String, &str)> = Vec::new();
            for f in &o.fields {
                let attr = model_attribute(&f.wire_name, &f.py_name);
                if let Some((_, other)) = seen.iter().find(|(a, _)| *a == attr) {
                    return Some(((*other).to_owned(), f.wire_name.clone(), attr));
                }
                seen.push((attr, &f.wire_name));
            }
            o.fields
                .iter()
                .map(|f| &f.ty)
                .chain(o.extra.as_deref())
                .find_map(attribute_clash)
        }
        SdkType::Array(t) | SdkType::Map(t) => attribute_clash(t),
        SdkType::Union { variants, .. } => variants.iter().find_map(attribute_clash),
        SdkType::Ref(_) | SdkType::Primitive(_) | SdkType::Unknown | SdkType::Literal(_) => None,
    }
}

/// FER201 for an [`attribute_clash`] in the schema at `pointer`.
fn clash_error(pointer: &str, (a, b, attr): &(String, String, String)) -> FerruleError {
    FerruleError::new(
        FerCode::Fer201,
        pointer,
        format!(
            "a model with the properties \"{a}\" and \"{b}\", which would both be the attribute `{attr}`, is not generated yet"
        ),
        "rename one of the properties with an openapi transform in stainless.yml",
    )
}

fn file_paths(ty: Type<'_>) -> Vec<Vec<String>> {
    let obj = match ty {
        Type::Object(o) => o,
        Type::Model(m) => return file_paths(m.ty()),
        _ => return Vec::new(),
    };
    let is_binary = |t: Type<'_>| matches!(t, Type::Scalar(Scalar::Binary));
    obj.fields()
        .filter_map(|f| {
            let name = snake_case(f.name().words());
            match f.ty() {
                t if is_binary(t) => Some(vec![name]),
                Type::Array(a) if is_binary(a.elem()) => Some(vec![name, "<array>".to_owned()]),
                _ => None,
            }
        })
        .collect()
}

/// A body field as a keyword argument.
fn field_param(f: &SdkField) -> SdkParam {
    SdkParam {
        wire_name: f.wire_name.clone(),
        py_name: f.py_name.clone(),
        required: f.required,
        description: f.description.clone(),
        ty: f.ty.clone(),
        // `SdkField` does not carry it; no emitter reads it (see the port notes).
        deprecated: false,
        example: None,
        minimum: None,
        maximum: None,
    }
}

/// The fields of every variant once, in the order they first appear: required where every
/// variant requires them, typed as the first variant that has them types them.
fn merge_variants(variants: &[SdkBodyVariant]) -> Vec<SdkParam> {
    let mut out: Vec<SdkParam> = Vec::new();
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

/// An object schema written inline under a property carries its description on the property
/// (Forge has no object descriptions); Stainless prints it as the class docstring too.
fn inline_described(ty: SdkType, field: Field<'_>) -> SdkType {
    let description = field.description();
    if description.is_empty() {
        return ty;
    }
    match (field.ty(), ty) {
        (Type::Object(_), SdkType::Object(o)) => SdkType::Object(SdkObject {
            description: Some(description.to_owned()),
            ..o
        }),
        // `T | null`: the object variant of a nullable inline object.
        (
            Type::Union(u),
            SdkType::Union {
                mut variants,
                discriminator,
            },
        ) if variants.len() == 2 => {
            if let Some(i) = u.members().position(|m| matches!(m, Type::Object(_)))
                && let Some(SdkType::Object(o)) = variants.get_mut(i)
            {
                o.description = Some(description.to_owned());
            }
            SdkType::Union {
                variants,
                discriminator,
            }
        }
        (_, ty) => ty,
    }
}

fn to_scalar(s: Scalar) -> SdkType {
    match s {
        // File uploads (`FileTypes`) need a model kind the project-file emitters know too; later.
        Scalar::Unknown | Scalar::Binary => SdkType::Unknown,
        Scalar::Null => SdkType::Primitive(SdkPrimitive::Null),
        Scalar::Boolean => SdkType::Primitive(SdkPrimitive::Boolean),
        Scalar::Date => SdkType::Primitive(SdkPrimitive::Date),
        Scalar::DateTime => SdkType::Primitive(SdkPrimitive::Datetime),
        Scalar::String | Scalar::Uuid | Scalar::Bytes => SdkType::Primitive(SdkPrimitive::String),
        Scalar::Number(n) => SdkType::Primitive(if matches!(n, Number::Float | Number::Double) {
            SdkPrimitive::Number
        } else {
            SdkPrimitive::Integer
        }),
    }
}

struct MutableResource {
    path: Vec<String>,
    order: usize,
    methods: Vec<SdkMethod>,
    children: Vec<(String, MutableResource)>,
    standalone: bool,
}

impl MutableResource {
    fn new(path: Vec<String>, order: usize) -> Self {
        Self {
            path,
            order,
            methods: Vec::new(),
            children: Vec::new(),
            standalone: false,
        }
    }

    /// The node at `written` (stainless.yml names) below this one, made as needed.
    fn descend(&mut self, written: &[String], order: &[usize]) -> &mut Self {
        let mut node = self;
        for (i, w) in written.iter().enumerate() {
            let name = snake_case(&split_words(w));
            let pos = if let Some(pos) = node.children.iter().position(|(k, _)| *k == name) {
                pos
            } else {
                let mut path = node.path.clone();
                path.push(name.clone());
                let child = Self::new(path, order.get(i).copied().unwrap_or(0));
                node.children.push((name, child));
                node.children.len() - 1
            };
            node = &mut node.children[pos].1;
        }
        node
    }
}

fn by_order<T>(mut items: Vec<T>, order: impl Fn(&T) -> usize) -> Vec<T> {
    items.sort_by_key(|t| order(t));
    items
}

fn freeze(r: MutableResource, casings: &[(String, Casing)]) -> SdkResource {
    let words = split_words(r.path.last().map_or("", String::as_str));
    SdkResource {
        class_segment: Some(class_segment_with(&words, casings)),
        singular_class_segment: Some(class_segment_with(&singular(&words), casings)),
        standalone: r.standalone,
        path: r.path,
        order: r.order,
        methods: by_order(r.methods, |m| m.order),
        subresources: by_order(r.children.into_iter().map(|(_, c)| freeze(c, casings)).collect(), |c| {
            c.order
        }),
    }
}

/// `tree` without the resources that have no methods, themselves or below.
fn with_methods(tree: &[SdkResource]) -> Vec<SdkResource> {
    tree.iter()
        .filter_map(|r| {
            let subresources = with_methods(&r.subresources);
            (!r.methods.is_empty() || !subresources.is_empty()).then(|| SdkResource {
                subresources,
                ..r.clone()
            })
        })
        .collect()
}

/// The client class name: `onebusaway-sdk` -> `OnebusawaySDK`.
#[must_use]
pub fn client_class_name(name: &str) -> String {
    client_class_name_with(name, &[])
}

/// [`client_class_name`] with custom casings: `lumaai` -> `LumaAI`.
#[must_use]
pub fn client_class_name_with(name: &str, casings: &[(String, Casing)]) -> String {
    class_segment_with(&split_words(name), casings)
}

fn to_settings(raw: &RawClient, casings: &[(String, Casing)]) -> SdkClientSettings {
    SdkClientSettings {
        name: raw.name.clone(),
        class_name: client_class_name_with(&raw.name, casings),
        env_prefix: raw.env_prefix.clone(),
        opts: raw.opts.clone(),
        environments: raw.environments.clone(),
        security_schemes: raw.security_schemes.clone(),
        security: raw.security.clone(),
        timeout_seconds: raw.timeout_seconds,
        // `maxRetries` is any number in TypeScript; the model keeps a count, saturating.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        max_retries: raw.max_retries as u32,
        idempotency_header: raw.idempotency_header.clone(),
        default_headers: raw.default_headers.clone(),
        array_format: raw.array_format,
        nested_format: raw.nested_format,
    }
}

/// Converts `methods` into one [`SdkClient`]; `base_url` is empty where Forge doesn't expose it.
fn to_client<'f>(methods: impl Iterator<Item = Method<'f>>, base_url: &str) -> Result<Option<SdkClient>, FerruleError> {
    let mut root = MutableResource::new(Vec::new(), 0);
    let mut casings: Option<Vec<(String, Casing)>> = None;
    let mut standalone: Vec<ResourcePosition> = Vec::new();
    let mut client_methods = Vec::new();
    let mut decls: Vec<(String, SdkModelDecl)> = Vec::new();
    let mut pkg: Option<SdkPackage> = None;
    let mut settings: Option<SdkClientSettings> = None;
    let mut style: Option<SdkStyle> = None;
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
        let casings = casings.get_or_insert_with(|| ext.casings.clone());
        style.get_or_insert(ext.style);
        if standalone.is_empty() {
            standalone.clone_from(&ext.standalone);
        }
        if settings.is_none()
            && let Some(client) = &ext.client
        {
            settings = Some(to_settings(client, casings));
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
    let resource_tree = freeze(root, casings.as_deref().unwrap_or_default()).subresources;
    let mut models: Vec<SdkModelDecl> = decls.into_iter().map(|(_, d)| d).collect();
    models.sort_by(|a, b| js::cmp(&a.r#ref.module, &b.r#ref.module));
    let class_name = settings.as_ref().map(|s| s.class_name.clone());
    let env_prefix = settings.as_ref().map(|s| s.env_prefix.clone());
    Ok(Some(SdkClient {
        package,
        base_url: base_url.to_owned(),
        settings,
        resources: with_methods(&resource_tree),
        client_methods: by_order(client_methods, |m| m.order),
        client_class: class_name,
        env_prefix,
        models,
        resource_tree,
        style: style.unwrap_or_default(),
    }))
}

/// The whole document; `None` when it has no operations to read the package from.
///
/// # Errors
///
/// A FER diagnostic for a malformed extension or a spec the plugin cannot generate yet.
pub fn forge_to_model(forge: &Forge) -> Result<Option<SdkClient>, FerruleError> {
    to_client(forge.groups().flat_map(Group::methods), &forge.base_url)
}

/// One group's own methods (not its children's, which get calls of their own).
///
/// # Errors
///
/// As [`forge_to_model`].
pub fn group_to_model(group: Group<'_>) -> Result<Option<SdkClient>, FerruleError> {
    to_client(group.methods(), "")
}

/// [`group_to_model`] on any methods, for tests that combine methods of several documents.
///
/// # Errors
///
/// As [`forge_to_model`].
pub fn methods_to_model<'f>(methods: impl Iterator<Item = Method<'f>>) -> Result<Option<SdkClient>, FerruleError> {
    to_client(methods, "")
}
