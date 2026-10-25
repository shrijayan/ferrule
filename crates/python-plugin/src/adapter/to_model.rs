//! Maps Forge's groups, methods, models and types to [`SdkClient`] (ADR 0002). With
//! [`super::extensions`], the only code that knows Forge's interface.

use std::collections::HashSet;

use forge::{Field, Forge, Group, Method, Number, Object, Scalar, Type};

use super::extensions::{MethodExtensions, RawClient, read_method_extensions};
use crate::diagnostics::{FerCode, FerruleError, operation_pointer};
use crate::js;
use crate::model::{
    HttpVerb, SdkClient, SdkClientSettings, SdkField, SdkLiteral, SdkMethod, SdkModelDecl, SdkObject, SdkPackage,
    SdkPagination, SdkParam, SdkPrimitive, SdkRef, SdkResource, SdkStreaming, SdkType,
};
use crate::naming::{class_segment, pascal_case, snake_case, split_words};

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
        Ok(SdkType::Object(SdkObject {
            fields,
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
                        };
                        self.decls.push((schema.to_owned(), decl));
                        let body = match self.read_type(model.ty())? {
                            SdkType::Object(o) => o,
                            t => return Err(self.unsupported(&format!("named model of kind {}", sdk_kind(&t)))),
                        };
                        if let Some((_, slot)) = self.decls.iter_mut().find(|(k, _)| k == schema) {
                            slot.ty = body;
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
                    other => other,
                }
            }
        })
    }

    fn read_params(&mut self, method: Method<'_>) -> Result<(Vec<SdkParam>, Vec<SdkParam>), FerruleError> {
        let mut path = Vec::new();
        let mut query = Vec::new();
        let request = method.request();
        let params = match (request, method.body()) {
            (Some(Type::Intersection(i)), Some(_)) => i.members().next(),
            (request, None) => request,
            _ => None,
        };
        let Some(params) = params else { return Ok((path, query)) };
        let Type::Object(obj) = params else {
            return Err(self.unsupported(&format!("parameters of kind {}", forge_kind(params))));
        };
        for field in obj.fields() {
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
            SdkType::Ref(r) => self
                .decls
                .iter()
                .find(|(_, d)| d.r#ref.module == r.module && d.r#ref.name == r.name)
                .map(|(_, d)| &d.ty),
            _ => None,
        }
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
        let header_params = match method.headers() {
            None => Vec::new(),
            Some(h) => h.fields().map(|f| self.read_param(f)).collect::<Result<_, _>>()?,
        };
        let body_params = self
            .object_of(body.as_ref())
            .map(|o| {
                o.fields
                    .iter()
                    .map(|f| SdkParam {
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
                    })
                    .collect()
            })
            .unwrap_or_default();
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
            response,
            pagination,
            streaming,
        })
    }
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
}

fn by_order<T>(mut items: Vec<T>, order: impl Fn(&T) -> usize) -> Vec<T> {
    items.sort_by_key(|t| order(t));
    items
}

fn freeze(r: MutableResource) -> SdkResource {
    SdkResource {
        path: r.path,
        order: r.order,
        methods: by_order(r.methods, |m| m.order),
        subresources: by_order(r.children.into_iter().map(|(_, c)| freeze(c)).collect(), |c| c.order),
    }
}

/// The client class name: `onebusaway-sdk` -> `OnebusawaySDK`.
#[must_use]
pub fn client_class_name(name: &str) -> String {
    class_segment(&split_words(name))
}

fn to_settings(raw: &RawClient) -> SdkClientSettings {
    SdkClientSettings {
        name: raw.name.clone(),
        class_name: client_class_name(&raw.name),
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
    let mut root = MutableResource {
        path: Vec::new(),
        order: 0,
        methods: Vec::new(),
        children: Vec::new(),
    };
    let mut client_methods = Vec::new();
    let mut decls: Vec<(String, SdkModelDecl)> = Vec::new();
    let mut pkg: Option<SdkPackage> = None;
    let mut settings: Option<SdkClientSettings> = None;
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
        let mut node = &mut root;
        for (i, written) in ext.resource.iter().enumerate() {
            let name = snake_case(&split_words(written));
            let pos = if let Some(pos) = node.children.iter().position(|(k, _)| *k == name) {
                pos
            } else {
                let mut path = node.path.clone();
                path.push(name.clone());
                let order = ext.resource_order.get(i).copied().unwrap_or(0);
                node.children.push((
                    name,
                    MutableResource {
                        path,
                        order,
                        methods: Vec::new(),
                        children: Vec::new(),
                    },
                ));
                node.children.len() - 1
            };
            node = &mut node.children[pos].1;
        }
        node.methods.push(converted);
    }
    let Some(package) = pkg else { return Ok(None) };
    let mut models: Vec<SdkModelDecl> = decls.into_iter().map(|(_, d)| d).collect();
    models.sort_by(|a, b| js::cmp(&a.r#ref.module, &b.r#ref.module));
    let class_name = settings.as_ref().map(|s| s.class_name.clone());
    let env_prefix = settings.as_ref().map(|s| s.env_prefix.clone());
    Ok(Some(SdkClient {
        package,
        base_url: base_url.to_owned(),
        settings,
        resources: freeze(root).subresources,
        client_methods: by_order(client_methods, |m| m.order),
        client_class: class_name,
        env_prefix,
        models,
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
