//! A method's params class: path and query parameters with a builder, additional headers and
//! query params, and the `Params` interface the services read.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Field, Method, Type};

use crate::emit::model::{class, required_block};
use crate::emit::{Gen, Service, method_doc};
use crate::kt::{Imports, doc, file, string};
use crate::naming::{camel, decapitalize, words};
use crate::types::{Class, Jt, list_shadowed, sorted_fields};

/// One parameter of a params class.
pub struct ParamProp<'a> {
    pub field: &'a Field,
    pub name: String,
    pub ty: Jt,
    pub path: bool,
}

impl ParamProp<'_> {
    /// Required in the builder (path params are checked by the service instead).
    #[must_use]
    pub fn required(&self) -> bool {
        !self.path && self.field.required
    }
}

/// The parameters of `m` in Stainless's order (path params in path order, then the query
/// params, required first, by name), and the classes they declare.
#[must_use]
pub fn param_props<'a>(g: &Gen<'a>, m: &'a Method) -> (Vec<ParamProp<'a>>, Vec<Class<'a>>) {
    let mut nested = Vec::new();
    let mut props = Vec::new();
    for p in &m.path_params {
        let w = words(&p.field.name);
        props.push(ParamProp {
            field: &p.field,
            name: camel(&w, g.casings),
            ty: g.mapper.jt(&p.field.ty, &w, &p.field.description, &mut nested),
            path: true,
        });
    }
    for f in sorted_fields(m.query_params.iter().map(|p| &p.field)) {
        let w = words(&f.name);
        props.push(ParamProp {
            field: f,
            name: camel(&w, g.casings),
            ty: g.mapper.jt(&f.ty, &w, &f.description, &mut nested),
            path: false,
        });
    }
    (props, nested)
}

/// Whether the params class of `m` can be built empty (`none()`).
#[must_use]
pub fn has_none(g: &Gen<'_>, m: &Method) -> bool {
    let (props, _) = param_props(g, m);
    !props.iter().any(ParamProp::required)
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

/// The params file of `m`: `(path, text)`.
// A Stainless file's members in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn params_file(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> (String, String) {
    let java = g.java();
    let name = g.params_name(svc, m);
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
    let (props, nested) = param_props(g, m);
    let holder = Class {
        name: name.clone(),
        doc: String::new(),
        shape: crate::types::Shape::Object {
            props: Vec::new(),
            bases: Vec::new(),
        },
        nested: nested.clone(),
    };
    let qualify = list_shadowed(&[&holder]);
    for p in &props {
        let mut paths = Vec::new();
        p.ty.imports(&mut paths);
        for path in paths {
            if let Some(rel) = path.strip_prefix('.') {
                imp.sdk(rel);
            } else {
                imp.add(path);
            }
        }
        if p.required() {
            imp.core("checkRequired");
        }
        if p.ty == Jt::OffsetDateTime && !p.path {
            imp.add("java.time.format.DateTimeFormatter");
        }
    }
    let field_doc = |p: &ParamProp<'_>| {
        if p.path {
            String::new()
        } else {
            doc(&p.field.description)
        }
    };

    let mut out = String::new();
    out.push_str(&doc(&method_doc(m)));
    let _ = writeln!(out, "class {name}\nprivate constructor(");
    for p in &props {
        let t = p.ty.kt(qualify);
        let opt = if p.required() { "" } else { "?" };
        let _ = writeln!(out, "private val {}: {t}{opt},", p.name);
    }
    out.push_str(
        "private val additionalHeaders: Headers,\nprivate val additionalQueryParams: QueryParams,\n) : Params {\n\n",
    );
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
    out.push_str("/** Additional headers to send with the request. */\nfun _additionalHeaders(): Headers = additionalHeaders\n\n");
    out.push_str("/** Additional query param to send with the request. */\nfun _additionalQueryParams(): QueryParams = additionalQueryParams\n\n");
    out.push_str("fun toBuilder() = Builder().from(this)\n\n");

    let required: Vec<String> = props.iter().filter(|p| p.required()).map(|p| p.name.clone()).collect();
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
    out.push_str("private var additionalHeaders: Headers.Builder = Headers.builder()\nprivate var additionalQueryParams: QueryParams.Builder = QueryParams.builder()\n\n");
    let var = decapitalize(&name);
    if java {
        out.push_str("@JvmSynthetic\n");
    }
    let _ = writeln!(out, "internal fun from({var}: {name}) = apply {{");
    for p in &props {
        let _ = writeln!(out, "{n} = {var}.{n}", n = p.name);
    }
    let _ = writeln!(
        out,
        "additionalHeaders = {var}.additionalHeaders.toBuilder()\nadditionalQueryParams = {var}.additionalQueryParams.toBuilder()\n}}\n"
    );
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
    out.push_str(HEADER_METHODS);
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
    out.push_str("additionalHeaders.build(), additionalQueryParams.build(),)\n}\n\n");

    let path_props: Vec<&ParamProp<'_>> = props.iter().filter(|p| p.path).collect();
    if !path_props.is_empty() {
        out.push_str("fun _pathParam(index: Int): String =\nwhen (index) {\n");
        for (i, p) in path_props.iter().enumerate() {
            let v = query_value(&p.ty, &p.name);
            if v == p.name {
                let _ = writeln!(out, "{i} -> {} ?: \"\"", p.name);
            } else {
                let _ = writeln!(out, "{i} -> {}?.toString() ?: \"\"", p.name);
            }
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
    for n in &nested {
        out.push_str(&class(g, n, &[&holder], &mut body_imp));
        out.push('\n');
    }
    imp.extend(&body_imp);

    let _ = write!(
        out,
        "override fun equals(other: Any?): Boolean {{\nif (this === other) {{\nreturn true\n}}\n\nreturn other is {name} && "
    );
    for p in &props {
        let _ = write!(out, "{n} == other.{n} && ", n = p.name);
    }
    out.push_str(
        "additionalHeaders == other.additionalHeaders && additionalQueryParams == other.additionalQueryParams\n}\n\n",
    );
    let mut hashed: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
    hashed.push("additionalHeaders".to_owned());
    hashed.push("additionalQueryParams".to_owned());
    let _ = writeln!(
        out,
        "override fun hashCode(): Int = Objects.hash({})\n",
        hashed.join(", ")
    );
    let mut shown: Vec<String> = props.iter().map(|p| format!("{n}=${n}", n = p.name)).collect();
    shown.push("additionalHeaders=$additionalHeaders".to_owned());
    shown.push("additionalQueryParams=$additionalQueryParams".to_owned());
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

/// Whether a param type needs no class of its own.
#[must_use]
pub fn is_simple(ty: &Type) -> bool {
    !matches!(ty, Type::Object(_) | Type::Enum { .. })
}
