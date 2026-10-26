//! A method's params class: path and query parameters and the request body with a builder,
//! additional headers, query params and body properties, and the `Params` interface the
//! services read.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Field, HttpMethod, Method, Type};

use crate::emit::model::{class, getter_doc, getter_type, raw_doc, required_block, setters};
use crate::emit::{Gen, Service, method_doc};
use crate::kt::{Imports, doc, file, string};
use crate::naming::{camel, decapitalize, words};
use crate::types::{Class, Jt, Prop, Shape, list_shadowed, sorted_fields};

/// One path or query parameter of a params class.
pub struct ParamProp<'a> {
    pub field: &'a Field,
    pub name: String,
    pub ty: Jt,
    pub path: bool,
    /// The path parameter the service methods take as their first argument (the last one).
    pub positional: bool,
}

impl ParamProp<'_> {
    /// Required in the builder: a required query param, or a path param before the positional
    /// one (the service checks the positional one instead).
    #[must_use]
    pub fn required(&self) -> bool {
        if self.path {
            !self.positional
        } else {
            self.field.required
        }
    }
}

/// What a params class sends as the request body.
pub enum BodyKind<'a> {
    /// Nothing (`get` and `head`).
    None,
    /// No body in the spec: only additional body properties.
    Extra,
    /// A JSON object: the nested `Body` class, and the classes its fields declare, which Stainless
    /// declares next to `Body` in the params class.
    Inline(Class<'a>, Vec<Class<'a>>),
    /// A named model, as one param.
    Model {
        name: String,
        ty: Jt,
        doc: String,
        required: bool,
        /// The model is an object, with additional properties of its own.
        object: bool,
    },
}

impl BodyKind<'_> {
    /// The body's own props, for an inline body.
    #[must_use]
    pub fn props(&self) -> &[Prop<'_>] {
        match self {
            Self::Inline(
                Class {
                    shape: Shape::Object { props, .. },
                    ..
                },
                _,
            ) => props,
            _ => &[],
        }
    }

    /// Whether the builder needs the body (a required body field, or a required model).
    #[must_use]
    pub fn required(&self) -> bool {
        match self {
            Self::Model { required, .. } => *required,
            _ => self.props().iter().any(Prop::required),
        }
    }
}

/// The body of `m`.
#[must_use]
pub fn body_kind<'a>(g: &Gen<'a>, method: &'a Method) -> BodyKind<'a> {
    match method.body.as_ref().map(|b| (&b.ty, b)) {
        Some((Type::Object(o), _)) => {
            let mut class = g.mapper.object_class("Body".to_owned(), o, String::new());
            let nested = std::mem::take(&mut class.nested);
            BodyKind::Inline(class, nested)
        }
        Some((Type::Ref(r), b)) => {
            let name = g.mapper.model_name(r);
            BodyKind::Model {
                name: camel(&crate::naming::split_words(&name), g.casings),
                ty: Jt::Class {
                    name,
                    kind: crate::types::ClassKind::Object,
                    import: Some(g.mapper.model_path(r)),
                },
                doc: g.client.model(r).map(|d| d.description.clone()).unwrap_or_default(),
                required: b.required,
                object: g.client.model(r).is_some_and(|d| matches!(d.ty, Type::Object(_))),
            }
        }
        None if matches!(method.http_method, HttpMethod::Get | HttpMethod::Head) => BodyKind::None,
        Some(_) | None => BodyKind::Extra,
    }
}

/// The parameters of `m` in Stainless's order (path params in path order, then the query
/// params, required first, by name), and the classes they declare.
#[must_use]
pub fn param_props<'a>(g: &Gen<'a>, method: &'a Method) -> (Vec<ParamProp<'a>>, Vec<Class<'a>>) {
    let mut nested = Vec::new();
    let mut props = Vec::new();
    let last = method.path_params.len().saturating_sub(1);
    for (i, p) in method.path_params.iter().enumerate() {
        let w = words(&p.field.name);
        props.push(ParamProp {
            field: &p.field,
            name: camel(&w, g.casings),
            ty: g.mapper.jt(&p.field.ty, &w, &p.field.description, &mut nested),
            path: true,
            positional: i == last,
        });
    }
    for f in sorted_fields(method.query_params.iter().map(|p| &p.field)) {
        let w = words(&f.name);
        props.push(ParamProp {
            field: f,
            name: camel(&w, g.casings),
            ty: g.mapper.jt(&f.ty, &w, &f.description, &mut nested),
            path: false,
            positional: false,
        });
    }
    (props, nested)
}

/// Whether the params class of `m` can be built empty (`none()`).
#[must_use]
pub fn has_none(g: &Gen<'_>, method: &Method) -> bool {
    let (props, _) = param_props(g, method);
    !props.iter().any(ParamProp::required) && !body_kind(g, method).required()
}

const HEADER_METHODS: &str = "fun additionalHeaders(additionalHeaders: Headers) = apply {
this.additionalHeaders.clear()
putAllAdditionalHeaders(additionalHeaders)
}

fun additionalHeaders(additionalHeaders: Map<String, Iterable<String>>) = apply {
this.additionalHeaders.clear()
putAllAdditionalHeaders(additionalHeaders)
}

fun putAdditionalHeader(name: String, value: String) = apply {
additionalHeaders.put(name, value)
}

fun putAdditionalHeaders(name: String, values: Iterable<String>) = apply {
additionalHeaders.put(name, values)
}

fun putAllAdditionalHeaders(additionalHeaders: Headers) = apply {
this.additionalHeaders.putAll(additionalHeaders)
}

fun putAllAdditionalHeaders(additionalHeaders: Map<String, Iterable<String>>) = apply {
this.additionalHeaders.putAll(additionalHeaders)
}

fun replaceAdditionalHeaders(name: String, value: String) = apply {
additionalHeaders.replace(name, value)
}

fun replaceAdditionalHeaders(name: String, values: Iterable<String>) = apply {
additionalHeaders.replace(name, values)
}

fun replaceAllAdditionalHeaders(additionalHeaders: Headers) = apply {
this.additionalHeaders.replaceAll(additionalHeaders)
}

fun replaceAllAdditionalHeaders(additionalHeaders: Map<String, Iterable<String>>) = apply {
this.additionalHeaders.replaceAll(additionalHeaders)
}

fun removeAdditionalHeaders(name: String) = apply { additionalHeaders.remove(name) }

fun removeAllAdditionalHeaders(names: Set<String>) = apply {
additionalHeaders.removeAll(names)
}

fun additionalQueryParams(additionalQueryParams: QueryParams) = apply {
this.additionalQueryParams.clear()
putAllAdditionalQueryParams(additionalQueryParams)
}

fun additionalQueryParams(additionalQueryParams: Map<String, Iterable<String>>) = apply {
this.additionalQueryParams.clear()
putAllAdditionalQueryParams(additionalQueryParams)
}

fun putAdditionalQueryParam(key: String, value: String) = apply {
additionalQueryParams.put(key, value)
}

fun putAdditionalQueryParams(key: String, values: Iterable<String>) = apply {
additionalQueryParams.put(key, values)
}

fun putAllAdditionalQueryParams(additionalQueryParams: QueryParams) = apply {
this.additionalQueryParams.putAll(additionalQueryParams)
}

fun putAllAdditionalQueryParams(additionalQueryParams: Map<String, Iterable<String>>) = apply {
this.additionalQueryParams.putAll(additionalQueryParams)
}

fun replaceAdditionalQueryParams(key: String, value: String) = apply {
additionalQueryParams.replace(key, value)
}

fun replaceAdditionalQueryParams(key: String, values: Iterable<String>) = apply {
additionalQueryParams.replace(key, values)
}

fun replaceAllAdditionalQueryParams(additionalQueryParams: QueryParams) = apply {
this.additionalQueryParams.replaceAll(additionalQueryParams)
}

fun replaceAllAdditionalQueryParams(additionalQueryParams: Map<String, Iterable<String>>) = apply {
this.additionalQueryParams.replaceAll(additionalQueryParams)
}

fun removeAdditionalQueryParams(key: String) = apply { additionalQueryParams.remove(key) }

fun removeAllAdditionalQueryParams(keys: Set<String>) = apply {
additionalQueryParams.removeAll(keys)
}

";

/// The additional-body-properties methods of a builder whose body is its `Body`.
const BODY_PROPERTY_METHODS: &str =
    "fun additionalBodyProperties(additionalBodyProperties: Map<String, JsonValue>) = apply {
body.additionalProperties(additionalBodyProperties)
}

fun putAdditionalBodyProperty(key: String, value: JsonValue) = apply {
body.putAdditionalProperty(key, value)
}

fun putAllAdditionalBodyProperties(additionalBodyProperties: Map<String, JsonValue>) = apply {
body.putAllAdditionalProperties(additionalBodyProperties)
}

fun removeAdditionalBodyProperty(key: String) = apply { body.removeAdditionalProperty(key) }

fun removeAllAdditionalBodyProperties(keys: Set<String>) = apply {
body.removeAllAdditionalProperties(keys)
}

";

/// The additional-body-properties methods of a builder without a body schema.
const EXTRA_PROPERTY_METHODS: &str =
    "fun additionalBodyProperties(additionalBodyProperties: Map<String, JsonValue>) = apply {
this.additionalBodyProperties.clear()
putAllAdditionalBodyProperties(additionalBodyProperties)
}

fun putAdditionalBodyProperty(key: String, value: JsonValue) = apply {
additionalBodyProperties.put(key, value)
}

fun putAllAdditionalBodyProperties(additionalBodyProperties: Map<String, JsonValue>) = apply {
this.additionalBodyProperties.putAll(additionalBodyProperties)
}

fun removeAdditionalBodyProperty(key: String) = apply {
additionalBodyProperties.remove(key)
}

fun removeAllAdditionalBodyProperties(keys: Set<String>) = apply {
keys.forEach(::removeAdditionalBodyProperty)
}

";

/// The query string value of a parameter `v` (an expression).
fn query_value(ty: &Jt, v: &str) -> String {
    match ty {
        Jt::String => v.to_owned(),
        Jt::List(items) => {
            let inner = if **items == Jt::String {
                String::new()
            } else {
                " { it.toString() }".to_owned()
            };
            format!("{v}.joinToString(\",\"){inner}")
        }
        Jt::OffsetDateTime => format!("DateTimeFormatter.ISO_OFFSET_DATE_TIME.format({v})"),
        _ => format!("{v}.toString()"),
    }
}

fn add_imports(ty: &Jt, imp: &mut Imports) {
    let mut paths = Vec::new();
    ty.imports(&mut paths);
    for path in paths {
        if let Some(rel) = path.strip_prefix('.') {
            imp.sdk(rel);
        } else {
            imp.add(path);
        }
    }
}

/// The params file of `m`: `(path, text)`.
// A Stainless file's members in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn params_file(g: &Gen<'_>, svc: &Service<'_>, method: &Method) -> (String, String) {
    let java = g.java();
    let name = g.params_name(svc, method);
    let pkg = g.models_package(svc);
    let mut imp = Imports::new(g.root());
    imp.add("java.util.Objects");
    imp.core("Params");
    imp.core("http.Headers");
    imp.core("http.QueryParams");
    if java {
        imp.add("java.util.Optional");
        imp.add("kotlin.jvm.optionals.getOrNull");
    }
    let (props, nested) = param_props(g, method);
    let body = body_kind(g, method);
    let mut all_nested = Vec::new();
    if let BodyKind::Inline(c, hoisted) = &body {
        all_nested.push(c.clone());
        all_nested.extend(hoisted.iter().cloned());
    }
    all_nested.extend(nested.iter().cloned());
    let holder = Class {
        name: name.clone(),
        doc: String::new(),
        shape: Shape::Object {
            props: Vec::new(),
            bases: Vec::new(),
        },
        nested: all_nested.clone(),
    };
    let qualify = list_shadowed(&[&holder]);
    let body_scope = match &body {
        BodyKind::Inline(c, _) => list_shadowed(&[&holder, c]),
        _ => qualify,
    };
    for p in &props {
        add_imports(&p.ty, &mut imp);
        if p.required() {
            imp.core("checkRequired");
        }
        if p.ty == Jt::OffsetDateTime && !p.path {
            imp.add("java.time.format.DateTimeFormatter");
        }
    }
    match &body {
        BodyKind::Inline(..) => {
            imp.core("JsonField");
            imp.core("JsonValue");
        }
        BodyKind::Extra => {
            imp.core("JsonValue");
            imp.core("toImmutable");
        }
        BodyKind::Model { ty, required, .. } => {
            add_imports(ty, &mut imp);
            if *required {
                imp.core("checkRequired");
            }
        }
        BodyKind::None => {}
    }
    let field_doc = |p: &ParamProp<'_>| {
        if p.path {
            String::new()
        } else {
            doc(&p.field.description)
        }
    };

    let mut out = String::new();
    out.push_str(&doc(&method_doc(method)));
    let _ = writeln!(out, "class {name}\nprivate constructor(");
    for p in &props {
        let t = p.ty.kt(qualify);
        let opt = if p.required() { "" } else { "?" };
        let _ = writeln!(out, "private val {}: {t}{opt},", p.name);
    }
    match &body {
        BodyKind::Inline(..) => out.push_str("private val body: Body,\n"),
        BodyKind::Model {
            name: n, ty, required, ..
        } => {
            let opt = if *required { "" } else { "?" };
            let _ = writeln!(out, "private val {n}: {}{opt},", ty.kt(qualify));
        }
        _ => {}
    }
    out.push_str("private val additionalHeaders: Headers,\nprivate val additionalQueryParams: QueryParams,\n");
    if matches!(body, BodyKind::Extra) {
        out.push_str("private val additionalBodyProperties: Map<String, JsonValue>,\n");
    }
    out.push_str(") : Params {\n\n");
    for p in &props {
        out.push_str(&field_doc(p));
        let t = p.ty.kt(qualify);
        let n = &p.name;
        if p.required() {
            let _ = writeln!(out, "fun {n}(): {t} = {n}\n");
        } else if java {
            let _ = writeln!(out, "fun {n}(): Optional<{t}> = Optional.ofNullable({n})\n");
        } else {
            let _ = writeln!(out, "fun {n}(): {t}? = {n}\n");
        }
    }
    match &body {
        BodyKind::Inline(..) => {
            for p in body.props() {
                if p.ty != Jt::Value {
                    out.push_str(&doc(&getter_doc(g, p)));
                    let _ = writeln!(
                        out,
                        "fun {n}(): {} = body.{n}()\n",
                        getter_type(g, p, body_scope),
                        n = p.name
                    );
                }
            }
            for p in body.props() {
                let n = &p.name;
                if p.ty == Jt::Value {
                    out.push_str(&doc(&p.field.description));
                    let _ = writeln!(out, "fun _{n}(): JsonValue = body._{n}()\n");
                } else {
                    out.push_str(&doc(&raw_doc(n)));
                    let _ = writeln!(
                        out,
                        "fun _{n}(): JsonField<{}> = body._{n}()\n",
                        p.ty.kt(body_scope)
                    );
                }
            }
            out.push_str("fun _additionalBodyProperties(): Map<String, JsonValue> = body._additionalProperties()\n\n");
        }
        BodyKind::Model {
            name: n,
            ty,
            doc: d,
            required,
            object,
        } => {
            out.push_str(&doc(d));
            let t = ty.kt(qualify);
            if *required {
                let _ = writeln!(out, "fun {n}(): {t} = {n}\n");
            } else if java {
                let _ = writeln!(out, "fun {n}(): Optional<{t}> = Optional.ofNullable({n})\n");
            } else {
                let _ = writeln!(out, "fun {n}(): {t}? = {n}\n");
            }
            if *object && *required {
                imp.core("JsonValue");
                let _ = writeln!(
                    out,
                    "fun _additionalBodyProperties(): Map<String, JsonValue> = {n}._additionalProperties()\n"
                );
            }
        }
        BodyKind::Extra => out.push_str(
            "/** Additional body properties to send with the request. */\nfun _additionalBodyProperties(): Map<String, JsonValue> = additionalBodyProperties\n\n",
        ),
        BodyKind::None => {}
    }
    out.push_str("/** Additional headers to send with the request. */\nfun _additionalHeaders(): Headers = additionalHeaders\n\n");
    out.push_str("/** Additional query param to send with the request. */\nfun _additionalQueryParams(): QueryParams = additionalQueryParams\n\n");
    out.push_str("fun toBuilder() = Builder().from(this)\n\n");

    let mut required: Vec<String> = props.iter().filter(|p| p.required()).map(|p| p.name.clone()).collect();
    match &body {
        BodyKind::Inline(..) => {
            required.extend(body.props().iter().filter(|p| p.required()).map(|p| p.name.clone()));
        }
        BodyKind::Model {
            name: n,
            required: true,
            ..
        } => required.push(n.clone()),
        _ => {}
    }
    let block = required_block(g, &required);
    let jvm_static = if java { "@JvmStatic " } else { "" };
    out.push_str("companion object {\n\n");
    if required.is_empty() {
        let _ = writeln!(out, "{jvm_static}fun none(): {name} = builder().build()\n");
    }
    let mut builder_doc = format!("Returns a mutable builder for constructing an instance of [{name}].");
    if !block.is_empty() {
        builder_doc.push_str("\n\n");
        builder_doc.push_str(&block);
    }
    out.push_str(&doc(&builder_doc));
    let _ = writeln!(out, "{jvm_static}fun builder() = Builder()\n}}\n");

    let _ = writeln!(
        out,
        "/** A builder for [{name}]. */\nclass Builder internal constructor() {{\n"
    );
    for p in &props {
        let _ = writeln!(out, "private var {}: {}? = null", p.name, p.ty.kt(qualify));
    }
    match &body {
        BodyKind::Inline(..) => out.push_str("private var body: Body.Builder = Body.builder()\n"),
        BodyKind::Model { name: n, ty, .. } => {
            let _ = writeln!(out, "private var {n}: {}? = null", ty.kt(qualify));
        }
        _ => {}
    }
    out.push_str("private var additionalHeaders: Headers.Builder = Headers.builder()\nprivate var additionalQueryParams: QueryParams.Builder = QueryParams.builder()\n");
    if matches!(body, BodyKind::Extra) {
        out.push_str("private var additionalBodyProperties: MutableMap<String, JsonValue> = mutableMapOf()\n");
    }
    out.push('\n');
    let var = decapitalize(&name);
    if java {
        out.push_str("@JvmSynthetic\n");
    }
    let _ = writeln!(out, "internal fun from({var}: {name}) = apply {{");
    for p in &props {
        let _ = writeln!(out, "{n} = {var}.{n}", n = p.name);
    }
    match &body {
        BodyKind::Inline(..) => {
            let _ = writeln!(out, "body = {var}.body.toBuilder()");
        }
        BodyKind::Model { name: n, .. } => {
            let _ = writeln!(out, "{n} = {var}.{n}");
        }
        _ => {}
    }
    let _ = writeln!(
        out,
        "additionalHeaders = {var}.additionalHeaders.toBuilder()\nadditionalQueryParams = {var}.additionalQueryParams.toBuilder()"
    );
    if matches!(body, BodyKind::Extra) {
        let _ = writeln!(
            out,
            "additionalBodyProperties = {var}.additionalBodyProperties.toMutableMap()"
        );
    }
    out.push_str("}\n\n");
    for p in &props {
        let n = &p.name;
        let t = p.ty.kt(qualify);
        out.push_str(&field_doc(p));
        if p.required() {
            let _ = writeln!(out, "fun {n}({n}: {t}) = apply {{ this.{n} = {n} }}\n");
            continue;
        }
        let _ = writeln!(out, "fun {n}({n}: {t}?) = apply {{ this.{n} = {n} }}\n");
        if p.ty.is_primitive() {
            out.push_str(&doc(&format!(
                "Alias for [Builder.{n}].\n\nThis unboxed primitive overload exists for backwards compatibility."
            )));
            let _ = writeln!(out, "fun {n}({n}: {t}) = {n}({n} as {t}?)\n");
        }
        if java {
            out.push_str(&doc(&format!(
                "Alias for calling [Builder.{n}] with `{n}.orElse(null)`."
            )));
            let _ = writeln!(out, "fun {n}({n}: Optional<{t}>) = {n}({n}.getOrNull())\n");
        }
    }
    match &body {
        BodyKind::Inline(..) => {
            let mut listed = String::new();
            for p in body.props() {
                let _ = write!(listed, "\n- [{}]", p.name);
            }
            out.push_str(&doc(&format!(
                "Sets the entire request body.\n\nThis is generally only useful if you are already constructing the body separately. Otherwise, it's more convenient to use the top-level setters instead:{listed}"
            )));
            out.push_str("fun body(body: Body) = apply { this.body = body.toBuilder() }\n\n");
            for p in body.props() {
                for setter in setters(g, p, body_scope) {
                    out.push_str(&setter.delegate("body"));
                }
            }
            out.push_str(BODY_PROPERTY_METHODS);
        }
        BodyKind::Model {
            name: n,
            ty,
            doc: d,
            required,
            ..
        } => {
            out.push_str(&doc(d));
            let t = ty.kt(qualify);
            if *required {
                let _ = writeln!(out, "fun {n}({n}: {t}) = apply {{ this.{n} = {n} }}\n");
            } else {
                let _ = writeln!(out, "fun {n}({n}: {t}?) = apply {{ this.{n} = {n} }}\n");
                if java {
                    out.push_str(&doc(&format!(
                        "Alias for calling [Builder.{n}] with `{n}.orElse(null)`."
                    )));
                    let _ = writeln!(out, "fun {n}({n}: Optional<{t}>) = {n}({n}.getOrNull())\n");
                }
            }
        }
        _ => {}
    }
    out.push_str(HEADER_METHODS);
    if matches!(body, BodyKind::Extra) {
        out.push_str(EXTRA_PROPERTY_METHODS);
    }
    let mut build_doc = format!(
        "Returns an immutable instance of [{name}].\n\nFurther updates to this [Builder] will not mutate the returned instance."
    );
    if !block.is_empty() {
        build_doc.push_str("\n\n");
        build_doc.push_str(&block);
        build_doc.push_str("\n\n@throws IllegalStateException if any required field is unset.");
    }
    out.push_str(&doc(&build_doc));
    let _ = write!(out, "fun build(): {name} = {name}(");
    for p in &props {
        if p.required() {
            let _ = write!(out, "checkRequired({}, {}), ", string(&p.name), p.name);
        } else {
            let _ = write!(out, "{}, ", p.name);
        }
    }
    match &body {
        BodyKind::Inline(..) => out.push_str("body.build(), "),
        BodyKind::Model {
            name: n,
            required: true,
            ..
        } => {
            let _ = write!(out, "checkRequired({}, {n}), ", string(n));
        }
        BodyKind::Model { name: n, .. } => {
            let _ = write!(out, "{n}, ");
        }
        _ => {}
    }
    out.push_str("additionalHeaders.build(), additionalQueryParams.build(), ");
    if matches!(body, BodyKind::Extra) {
        out.push_str("additionalBodyProperties.toImmutable(),");
    }
    out.push_str(")\n}\n\n");

    match &body {
        BodyKind::Inline(..) => out.push_str("fun _body(): Body = body\n\n"),
        BodyKind::Model { name: n, ty, required, .. } => {
            let t = ty.kt(qualify);
            if *required {
                let _ = writeln!(out, "fun _body(): {t} = {n}\n");
            } else if java {
                let _ = writeln!(out, "fun _body(): Optional<{t}> = Optional.ofNullable({n})\n");
            } else {
                let _ = writeln!(out, "fun _body(): {t}? = {n}\n");
            }
        }
        BodyKind::Extra if java => out.push_str(
            "fun _body(): Optional<Map<String, JsonValue>> =\nOptional.ofNullable(additionalBodyProperties.ifEmpty { null })\n\n",
        ),
        BodyKind::Extra => out.push_str(
            "fun _body(): Map<String, JsonValue>? = additionalBodyProperties.ifEmpty { null }\n\n",
        ),
        BodyKind::None => {}
    }
    let path_props: Vec<&ParamProp<'_>> = props.iter().filter(|p| p.path).collect();
    if !path_props.is_empty() {
        out.push_str("fun _pathParam(index: Int): String =\nwhen (index) {\n");
        for (i, p) in path_props.iter().enumerate() {
            let v = query_value(&p.ty, &p.name);
            let text = match (p.required(), v == p.name) {
                (true, _) => v,
                (false, true) => format!("{} ?: \"\"", p.name),
                (false, false) => format!("{}?.toString() ?: \"\"", p.name),
            };
            let _ = writeln!(out, "{i} -> {text}");
        }
        out.push_str("else -> \"\"\n}\n\n");
    }
    out.push_str("override fun _headers(): Headers = additionalHeaders\n\n");
    let query: Vec<&ParamProp<'_>> = props.iter().filter(|p| !p.path).collect();
    if query.is_empty() {
        out.push_str("override fun _queryParams(): QueryParams = additionalQueryParams\n\n");
    } else {
        out.push_str("override fun _queryParams(): QueryParams =\nQueryParams.builder()\n.apply {\n");
        for p in &query {
            let wire = string(&p.field.wire_name);
            if p.required() {
                let _ = writeln!(out, "put({wire}, {})", query_value(&p.ty, &p.name));
            } else {
                let _ = writeln!(out, "{}?.let {{ put({wire}, {}) }}", p.name, query_value(&p.ty, "it"));
            }
        }
        out.push_str("putAll(additionalQueryParams)\n}\n.build()\n\n");
    }
    let mut body_imp = Imports::new(g.root());
    for n in &all_nested {
        out.push_str(&class(g, n, &[&holder], &mut body_imp));
        out.push('\n');
    }
    imp.extend(&body_imp);

    let mut fields: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
    match &body {
        BodyKind::Inline(..) => fields.push("body".to_owned()),
        BodyKind::Model { name: n, .. } => fields.push(n.clone()),
        _ => {}
    }
    fields.push("additionalHeaders".to_owned());
    fields.push("additionalQueryParams".to_owned());
    if matches!(body, BodyKind::Extra) {
        fields.push("additionalBodyProperties".to_owned());
    }
    let _ = write!(
        out,
        "override fun equals(other: Any?): Boolean {{\nif (this === other) {{\nreturn true\n}}\n\nreturn other is {name} && "
    );
    let compared: Vec<String> = fields.iter().map(|f| format!("{f} == other.{f}")).collect();
    let _ = writeln!(out, "{}\n}}\n", compared.join(" && "));
    let _ = writeln!(
        out,
        "override fun hashCode(): Int = Objects.hash({})\n",
        fields.join(", ")
    );
    let shown: Vec<String> = fields.iter().map(|f| format!("{f}=${f}")).collect();
    let _ = writeln!(out, "override fun toString() =\n\"{name}{{{}}}\"\n}}", shown.join(", "));

    let path = format!("{}/{}.kt", models_dir(g, svc), name);
    (path, file(&pkg, &imp, &out))
}

/// The directory of a service's models: `<core main>/models/agency`.
#[must_use]
pub fn models_dir(g: &Gen<'_>, svc: &Service<'_>) -> String {
    let mut d = format!("{}/models", g.core_main());
    for seg in &svc.model_pkg {
        d.push('/');
        d.push_str(seg);
    }
    d
}
