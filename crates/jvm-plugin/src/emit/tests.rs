//! The tests Stainless generates: a test per params class and model, a test per service
//! (blocking and async) against the mock server, error handling and params tests against
//! `WireMock`, the client options test and the `ProGuard` compatibility test.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{Literal, Method, ModelDecl, Type, TypeRef};

use crate::emit::params::{BodyKind, ParamProp, body_kind, param_props};
use crate::emit::service::response_type;
use crate::emit::{Gen, Service};
use crate::kt::{Imports, file, string};
use crate::naming::{capitalize, constant, decapitalize, split_words, title, words};
use crate::types::{Class, Jt, Shape};

/// Every test file.
#[must_use]
pub fn test_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for svc in &g.services {
        for m in svc.methods() {
            out.push(params_test(g, svc, m));
            out.extend(response_test(g, svc, m));
        }
        for async_ in [false, true] {
            out.extend(service_test(g, svc, async_));
        }
    }
    for d in &g.client.models {
        out.extend(model_test(g, d));
    }
    out.push(client_options_test(g));
    if let Some((svc, m)) = example(g) {
        out.push(error_handling_test(g, svc, m));
        out.push(service_params_test(g, svc, m));
    }
    out.push(proguard_test(g));
    out
}

/// The method the client tests call: [`crate::JvmMeta::example`], else the first method without
/// required parameters, else the first method.
fn example<'a>(g: &'a Gen<'a>) -> Option<(&'a Service<'a>, &'a Method)> {
    let all = || {
        g.services
            .iter()
            .flat_map(|svc| svc.methods().into_iter().map(move |m| (svc, m)))
    };
    if let Some((path, method)) = &g.target.example {
        let found = all().find(|(svc, m)| {
            m.name.written == *method
                && svc
                    .resource
                    .path
                    .iter()
                    .map(|n| n.written.as_str())
                    .eq(path.iter().map(String::as_str))
        });
        if found.is_some() {
            return found;
        }
    }
    all()
        .find(|(_, m)| m.path_params.is_empty() && !m.query_params.iter().any(|p| p.field.required) && m.body.is_none())
        .or_else(|| all().next())
}

/// A test value of a scalar type, or `None` for a class or list.
fn scalar_value(ty: &Jt, wire: &str, imp: &mut Imports) -> Option<String> {
    Some(match ty {
        Jt::String => string(wire),
        Jt::Long => "0L".to_owned(),
        Jt::Int => "0".to_owned(),
        Jt::Double => "0.0".to_owned(),
        Jt::Float => "0.0f".to_owned(),
        Jt::Boolean => "true".to_owned(),
        Jt::OffsetDateTime => {
            imp.add("java.time.OffsetDateTime");
            "OffsetDateTime.parse(\"2019-12-27T18:11:19.117Z\")".to_owned()
        }
        Jt::LocalDate => {
            imp.add("java.time.LocalDate");
            "LocalDate.parse(\"2019-12-27\")".to_owned()
        }
        Jt::Value => {
            imp.core("JsonValue");
            "JsonValue.from(mapOf<String, Any>())".to_owned()
        }
        Jt::List(_) | Jt::Class { .. } => return None,
    })
}

/// The query-string text of a scalar test value.
fn query_text(ty: &Jt, wire: &str, c: Option<&Class<'_>>) -> String {
    match ty {
        Jt::String => wire.to_owned(),
        Jt::Long | Jt::Int => "0".to_owned(),
        Jt::Double | Jt::Float => "0.0".to_owned(),
        Jt::Boolean => "true".to_owned(),
        Jt::OffsetDateTime => "2019-12-27T18:11:19.117Z".to_owned(),
        Jt::LocalDate => "2019-12-27".to_owned(),
        Jt::Class { .. } => match c.map(|c| &c.shape) {
            Some(Shape::Enum { members, .. }) => members.first().map_or_else(String::new, |m| match &m.value {
                Literal::String(svc) => svc.clone(),
                Literal::Number(n) => format!("{n}"),
                Literal::Boolean(b) => b.to_string(),
            }),
            _ => String::new(),
        },
        Jt::List(items) => query_text(items, "string", c),
        Jt::Value => String::new(),
    }
}

/// Builds test values of model classes, resolving nested classes and named models.
struct Values<'g, 'a> {
    g: &'g Gen<'a>,
}

impl<'a> Values<'_, 'a> {
    /// The class of a named model, if it is an object or an enum.
    fn model_class(&self, r: &TypeRef) -> Option<Class<'a>> {
        let d: &'a ModelDecl = self.g.client.model(r)?;
        let name = self.g.mapper.model_name(r);
        match &d.ty {
            Type::Object(out) => Some(self.g.mapper.object_class(name, out, d.description.clone())),
            _ => None,
        }
    }

    /// A test value of `ty`: `path` qualifies the classes nested in `scope`.
    fn value(&self, ty: &Jt, wire: &str, scope: &Class<'a>, path: &str, imp: &mut Imports) -> String {
        if let Some(v) = scalar_value(ty, wire, imp) {
            return v;
        }
        match ty {
            Jt::List(items) => self.value(items, "string", scope, path, imp),
            Jt::Class { name, import, .. } => {
                if let Some(p) = import {
                    imp.add(p.clone());
                    let r = self
                        .g
                        .client
                        .models
                        .iter()
                        .find(|d| self.g.mapper.model_path(&d.r#ref) == *p);
                    match r.and_then(|d| self.model_class(&d.r#ref)) {
                        Some(c) => self.build(&c, name, imp),
                        None => format!("{name}.builder().build()"),
                    }
                } else {
                    let qualified = format!("{path}.{name}");
                    match scope.nested.iter().find(|n| n.name == *name) {
                        Some(c) => self.build_or_member(c, &qualified, imp),
                        None => format!("{qualified}.builder().build()"),
                    }
                }
            }
            _ => String::new(),
        }
    }

    fn build_or_member(&self, c: &Class<'a>, qualified: &str, imp: &mut Imports) -> String {
        match &c.shape {
            Shape::Enum { members, .. } => members.first().map_or_else(
                || format!("{qualified}.of(\"\")"),
                |m| format!("{qualified}.{}", constant(&m.name.words)),
            ),
            Shape::Object { .. } => self.build(c, qualified, imp),
        }
    }

    /// `Class.builder().a(..)...build()`.
    fn build(&self, c: &Class<'a>, qualified: &str, imp: &mut Imports) -> String {
        let Shape::Object { props, .. } = &c.shape else {
            return self.build_or_member(c, qualified, imp);
        };
        let mut out = format!("{qualified}.builder()");
        for p in props {
            let v = self.value(&p.ty, p.wire(), c, qualified, imp);
            if p.ty.items().is_some() {
                let one = crate::naming::camel(&crate::naming::singular(&split_words(&p.name)), self.g.casings);
                let _ = write!(out, ".add{}({v})", capitalize(&one));
            } else {
                let _ = write!(out, ".{}({v})", p.name);
            }
        }
        out.push_str(".build()");
        out
    }
}

/// The test file of a method's params class.
// A Stainless test file's tests in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
fn params_test(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> (String, String) {
    let name = g.params_name(svc, m);
    let (props, nested) = param_props(g, m);
    let body = body_kind(g, m);
    let mut all_nested = Vec::new();
    if let BodyKind::Inline(c, hoisted) = &body {
        all_nested.push(c.clone());
        all_nested.extend(hoisted.iter().cloned());
    }
    all_nested.extend(nested);
    let holder = Class {
        name: name.clone(),
        doc: String::new(),
        shape: Shape::Object {
            props: Vec::new(),
            bases: Vec::new(),
        },
        nested: all_nested,
    };
    let mut imp = Imports::new(g.root());
    imp.add("org.junit.jupiter.api.Test");
    let vals = Values { g };
    // `.name(value)` calls for the params (`all`: every one, else the path and required ones).
    let calls = |all: bool, imp: &mut Imports| {
        let mut out = String::new();
        for p in props.iter().filter(|p| all || p.path || p.required()) {
            let v = vals.value(&p.ty, &p.field.wire_name, &holder, &name, imp);
            let _ = write!(out, ".{}({v})", p.name);
        }
        match &body {
            BodyKind::Inline(..) => {
                for p in body.props().iter().filter(|p| all || p.required()) {
                    let v = vals.value(&p.ty, p.wire(), &holder, &name, imp);
                    if p.ty.items().is_some() {
                        let one = crate::naming::camel(&crate::naming::singular(&split_words(&p.name)), g.casings);
                        let _ = write!(out, ".add{}({v})", capitalize(&one));
                    } else {
                        let _ = write!(out, ".{}({v})", p.name);
                    }
                }
            }
            BodyKind::Model {
                name: n, ty, required, ..
            } if all || *required => {
                let v = vals.value(ty, n, &holder, &name, imp);
                let _ = write!(out, ".{n}({v})");
            }
            _ => {}
        }
        out
    };
    let full = format!("{name}.builder(){}.build()", calls(true, &mut imp));
    let minimal = format!("{name}.builder(){}.build()", calls(false, &mut imp));
    let mut out = format!("internal class {name}Test {{\n\n@Test\nfun create() {{\n{full}\n}}\n\n");
    let paths: Vec<&ParamProp<'_>> = props.iter().filter(|p| p.path).collect();
    if !paths.is_empty() {
        imp.add("org.assertj.core.api.Assertions.assertThat");
        let _ = writeln!(out, "@Test\nfun pathParams() {{\nval params = {minimal}\n");
        for (i, p) in paths.iter().enumerate() {
            let _ = writeln!(
                out,
                "assertThat(params._pathParam({i})).isEqualTo({})",
                string(&query_text(&p.ty, &p.field.wire_name, None))
            );
        }
        let _ = writeln!(
            out,
            "// out-of-bound path param\nassertThat(params._pathParam({})).isEqualTo(\"\")\n}}\n",
            paths.len()
        );
    }
    let query: Vec<&ParamProp<'_>> = props.iter().filter(|p| !p.path).collect();
    if !query.is_empty() {
        imp.add("org.assertj.core.api.Assertions.assertThat");
        imp.core("http.QueryParams");
        let puts = |only_required: bool| {
            let mut out = String::from("QueryParams.builder()");
            for p in query.iter().filter(|p| !only_required || p.required()) {
                let class = match &p.ty {
                    Jt::Class { name: n, .. } => holder.nested.iter().find(|c| c.name == *n),
                    _ => None,
                };
                let _ = write!(
                    out,
                    ".put({}, {})",
                    string(&p.field.wire_name),
                    string(&query_text(&p.ty, &p.field.wire_name, class))
                );
            }
            out.push_str(".build()");
            out
        };
        let _ = writeln!(
            out,
            "@Test\nfun queryParams() {{\nval params = {full}\n\nval queryParams = params._queryParams()\n\nassertThat(queryParams).isEqualTo({})\n}}\n",
            puts(false)
        );
        if query.iter().any(|p| !p.required()) {
            let _ = writeln!(
                out,
                "@Test\nfun queryParamsWithoutOptionalFields() {{\nval params = {minimal}\n\nval queryParams = params._queryParams()\n\nassertThat(queryParams).isEqualTo({})\n}}\n",
                puts(true)
            );
        }
    }
    match &body {
        BodyKind::Inline(..) => {
            imp.add("org.assertj.core.api.Assertions.assertThat");
            let asserts = |all: bool, imp: &mut Imports| {
                let mut out = String::new();
                for p in body.props().iter().filter(|p| all || p.required()) {
                    let v = vals.value(&p.ty, p.wire(), &holder, &name, imp);
                    out.push_str(&assertion(g, "body", p, &v, imp));
                }
                out
            };
            let _ = writeln!(
                out,
                "@Test\nfun body() {{\nval params = {full}\n\nval body = params._body()\n\n{}}}\n",
                asserts(true, &mut imp)
            );
            if body.props().iter().any(|p| !p.required()) {
                let _ = writeln!(
                    out,
                    "@Test\nfun bodyWithoutOptionalFields() {{\nval params = {minimal}\n\nval body = params._body()\n\n{}}}\n",
                    asserts(false, &mut imp)
                );
            }
        }
        BodyKind::Model { name: n, ty, .. } => {
            imp.add("org.assertj.core.api.Assertions.assertThat");
            let v = vals.value(ty, n, &holder, &name, &mut imp);
            let _ = writeln!(
                out,
                "@Test\nfun body() {{\nval params = {full}\n\nval body = params._body()\n\nassertThat(body).isEqualTo({v})\n}}\n"
            );
        }
        _ => {}
    }
    out.push_str("}\n");
    let mut dir = format!("{}/models", g.core_test());
    for seg in &svc.model_pkg {
        dir.push('/');
        dir.push_str(seg);
    }
    (format!("{dir}/{name}Test.kt"), file(&g.models_package(svc), &imp, &out))
}

/// `assertThat(<var>.<prop>()).isEqualTo(<value>)`, or the list or `Optional` form.
fn assertion(g: &Gen<'_>, var: &str, p: &crate::types::Prop<'_>, v: &str, imp: &mut Imports) -> String {
    let present = p.required() && !p.nullable;
    let getter = format!("{var}.{}()", p.name);
    if p.ty.items().is_some() {
        if present || !g.java() {
            format!("assertThat({getter}).containsExactly({v})\n")
        } else {
            imp.add("kotlin.jvm.optionals.getOrNull");
            format!("assertThat({getter}.getOrNull()).containsExactly({v})\n")
        }
    } else if present || !g.java() {
        format!("assertThat({getter}).isEqualTo({v})\n")
    } else {
        format!("assertThat({getter}).contains({v})\n")
    }
}

/// The test of a model class: build it, read every field back, and round-trip it through JSON.
fn class_test(g: &Gen<'_>, c: &Class<'_>, pkg: &str, dir: &str, imp: &mut Imports) -> (String, String) {
    let name = &c.name;
    let var = decapitalize(name);
    imp.add("com.fasterxml.jackson.module.kotlin.jacksonTypeRef");
    imp.add("org.assertj.core.api.Assertions.assertThat");
    imp.add("org.junit.jupiter.api.Test");
    imp.core("jsonMapper");
    let vals = Values { g };
    let built = vals.build(c, name, imp);
    let mut out = format!("internal class {name}Test {{\n\n@Test\nfun create() {{\nval {var} = {built}\n\n");
    if let Shape::Object { props, .. } = &c.shape {
        for p in props {
            let v = vals.value(&p.ty, p.wire(), c, name, imp);
            out.push_str(&assertion(g, &var, p, &v, imp));
        }
    }
    let _ = writeln!(
        out,
        "}}\n\n@Test\nfun roundtrip() {{\nval jsonMapper = jsonMapper()\nval {var} = {built}\n\nval roundtripped{name} = jsonMapper.readValue(jsonMapper.writeValueAsString({var}), jacksonTypeRef<{name}>(),)\n\nassertThat(roundtripped{name}).isEqualTo({var})\n}}\n}}"
    );
    (format!("{dir}/{name}Test.kt"), file(pkg, imp, &out))
}

fn response_test(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> Option<(String, String)> {
    let Type::Object(out) = &m.response.as_ref()?.ty else {
        return None;
    };
    let name = g.response_name(svc, m);
    let c = g.mapper.object_class(name, out, String::new());
    let mut dir = format!("{}/models", g.core_test());
    for seg in &svc.model_pkg {
        dir.push('/');
        dir.push_str(seg);
    }
    let mut imp = Imports::new(g.root());
    Some(class_test(g, &c, &g.models_package(svc), &dir, &mut imp))
}

fn model_test(g: &Gen<'_>, d: &ModelDecl) -> Option<(String, String)> {
    let Type::Object(out) = &d.ty else { return None };
    let name = g.mapper.model_name(&d.r#ref);
    let c = g.mapper.object_class(name, out, d.description.clone());
    let pkg = g.mapper.model_package(&d.r#ref);
    let rel = pkg
        .strip_prefix(g.root())
        .unwrap_or(&pkg)
        .trim_start_matches('.')
        .replace('.', "/");
    let dir = format!("{}/{rel}", g.core_test());
    let mut imp = Imports::new(g.root());
    Some(class_test(g, &c, &pkg, &dir, &mut imp))
}

/// `.apiKey("My API Key")`: the client options a test client is built with.
fn test_options(g: &Gen<'_>) -> String {
    let mut out = String::new();
    let Some(settings) = g.client.settings.as_ref() else {
        return out;
    };
    for opt in &settings.opts {
        let required = !opt.nullable && opt.default.is_none();
        if !required && opt.auth.is_none() {
            continue;
        }
        let n = crate::naming::camel(&words(&opt.name), g.casings);
        let _ = write!(out, ".{n}({})", string(&format!("My {}", title(&words(&opt.name)))));
    }
    out
}

/// The plural of a variable name's last word: `agency` -> `agencies`.
fn plural(name: &str) -> String {
    if let Some(stem) = name.strip_suffix('y')
        && !stem.ends_with(['a', 'e', 'i', 'o', 'u'])
    {
        return format!("{stem}ies");
    }
    if name.ends_with('s') || name.ends_with('x') || name.ends_with("ch") || name.ends_with("sh") {
        return format!("{name}es");
    }
    format!("{name}s")
}

/// The variable a service test stores a method's result in.
fn result_var(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> String {
    match m.response.as_ref().map(|r| &r.ty) {
        Some(Type::Ref(r)) => return decapitalize(&g.mapper.model_name(r)),
        Some(Type::Array { items }) => {
            if let Type::Ref(r) = items.as_ref() {
                return plural(&decapitalize(&g.mapper.model_name(r)));
            }
        }
        _ => {}
    }
    let single = decapitalize(&svc.prefix);
    if m.name.written == "list" {
        plural(&single)
    } else {
        single
    }
}

/// The arguments a service test calls a method with.
fn call_args(g: &Gen<'_>, svc: &Service<'_>, m: &Method, imp: &mut Imports) -> String {
    let name = g.params_name(svc, m);
    let (props, nested) = param_props(g, m);
    let body = body_kind(g, m);
    let mut all_nested = Vec::new();
    if let BodyKind::Inline(c, hoisted) = &body {
        all_nested.push(c.clone());
        all_nested.extend(hoisted.iter().cloned());
    }
    all_nested.extend(nested);
    let holder = Class {
        name: name.clone(),
        doc: String::new(),
        shape: Shape::Object {
            props: Vec::new(),
            bases: Vec::new(),
        },
        nested: all_nested,
    };
    let vals = Values { g };
    let no_body = matches!(body, BodyKind::None | BodyKind::Extra);
    if no_body && props.is_empty() {
        return String::new();
    }
    if no_body && props.len() == 1 && props[0].path {
        return string(&query_text(&props[0].ty, &props[0].field.wire_name, None));
    }
    imp.add(format!("{}.{name}", g.models_package(svc)));
    let mut out = format!("{name}.builder()");
    for p in &props {
        let v = vals.value(&p.ty, &p.field.wire_name, &holder, &name, imp);
        let _ = write!(out, ".{}({v})", p.name);
    }
    match &body {
        BodyKind::Inline(..) => {
            for p in body.props() {
                let v = vals.value(&p.ty, p.wire(), &holder, &name, imp);
                if p.ty.items().is_some() {
                    let one = crate::naming::camel(&crate::naming::singular(&split_words(&p.name)), g.casings);
                    let _ = write!(out, ".add{}({v})", capitalize(&one));
                } else {
                    let _ = write!(out, ".{}({v})", p.name);
                }
            }
        }
        BodyKind::Model { name: n, ty, .. } => {
            let v = vals.value(ty, n, &holder, &name, imp);
            let _ = write!(out, ".{n}({v})");
        }
        _ => {}
    }
    out.push_str(".build()");
    out
}

/// `client.files().versions()`.
fn accessor_chain(g: &Gen<'_>, svc: &Service<'_>) -> String {
    let index = g.services.iter().position(|x| std::ptr::eq(x, svc)).unwrap_or_default();
    let mut chain = vec![index];
    let mut cur = index;
    while let Some(parent) = g.services.iter().position(|p| p.children.contains(&cur)) {
        chain.push(parent);
        cur = parent;
    }
    chain.reverse();
    let mut out = String::from("client");
    for i in chain {
        let _ = write!(out, ".{}()", g.services[i].accessor);
    }
    out
}

fn service_test(g: &Gen<'_>, svc: &Service<'_>, async_: bool) -> Option<(String, String)> {
    if svc.resource.methods.is_empty() {
        return None;
    }
    let java = g.java();
    let sfx = if async_ { "Async" } else { "" };
    let class_name = format!("{}Service{sfx}", svc.name);
    let mut imp = Imports::new(g.root());
    imp.add("org.junit.jupiter.api.Test");
    let okhttp = format!("{}OkHttpClient{sfx}", g.client_name());
    imp.sdk(&format!("client.okhttp.{okhttp}"));
    let svc_var = decapitalize(&class_name);
    let (annotation, base_url) = if g.target.mock_tests {
        imp.add("org.junit.jupiter.api.extension.ExtendWith");
        imp.sdk("TestServerExtension");
        (
            "@ExtendWith(TestServerExtension::class)\n",
            ".baseUrl(TestServerExtension.BASE_URL)",
        )
    } else {
        imp.add("org.junit.jupiter.api.Disabled");
        ("", "")
    };
    let disabled = if g.target.mock_tests {
        ""
    } else {
        "@Disabled(\"Mock server tests are disabled\")\n"
    };
    let mut out = format!("{annotation}internal class {class_name}Test {{\n\n");
    for m in svc.methods() {
        let mn = g.method_name(m);
        let fun = if async_ && !java { "suspend fun" } else { "fun" };
        let args = call_args(g, svc, m, &mut imp);
        let var = result_var(g, svc, m);
        let resp = response_type(g, svc, m);
        let validate = if resp.each {
            format!("{var}.forEach {{ it.validate() }}")
        } else {
            format!("{var}.validate()")
        };
        let _ = writeln!(
            out,
            "{disabled}@Test\n{fun} {mn}() {{\nval client = {okhttp}.builder(){base_url}{}.build()\nval {svc_var} = {}\n",
            test_options(g),
            accessor_chain(g, svc)
        );
        match (async_ && java, resp.ty.is_some()) {
            (true, true) => {
                let _ = writeln!(
                    out,
                    "val {var}Future = {svc_var}.{mn}({args})\n\nval {var} = {var}Future.get()\n{validate}\n}}\n"
                );
            }
            (true, false) => {
                let _ = writeln!(
                    out,
                    "val future = {svc_var}.{mn}({args})\n\nval response = future.get()\n}}\n"
                );
            }
            (false, true) => {
                let _ = writeln!(out, "val {var} = {svc_var}.{mn}({args})\n\n{validate}\n}}\n");
            }
            (false, false) => {
                let _ = writeln!(out, "{svc_var}.{mn}({args})\n}}\n");
            }
        }
    }
    out.push_str("}\n");
    let pkg = g.service_package(svc, async_);
    let path = format!(
        "{}/services/{}/{}{class_name}Test.kt",
        g.core_test(),
        if async_ { "async" } else { "blocking" },
        crate::emit::dir_prefix(&svc.service_pkg)
    );
    Some((path, file(&pkg, &imp, &out)))
}

/// The wiremock stub of a method's HTTP verb: `get`.
fn verb(m: &Method) -> &'static str {
    m.http_method.as_str()
}

fn error_handling_test(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> (String, String) {
    let client = g.client_name();
    let mut imp = Imports::new(g.root());
    for p in [
        "com.github.tomakehurst.wiremock.client.WireMock.anyUrl",
        "com.github.tomakehurst.wiremock.client.WireMock.status",
        "com.github.tomakehurst.wiremock.client.WireMock.stubFor",
        "com.github.tomakehurst.wiremock.junit5.WireMockRuntimeInfo",
        "com.github.tomakehurst.wiremock.junit5.WireMockTest",
        "org.assertj.core.api.Assertions.assertThat",
        "org.assertj.core.api.Assertions.entry",
        "org.junit.jupiter.api.BeforeEach",
        "org.junit.jupiter.api.Test",
        "org.junit.jupiter.api.assertThrows",
        "org.junit.jupiter.api.parallel.ResourceLock",
    ] {
        imp.add(p);
    }
    imp.add(format!("com.github.tomakehurst.wiremock.client.WireMock.{}", verb(m)));
    imp.sdk(&format!("client.{client}Client"));
    imp.sdk(&format!("client.okhttp.{client}OkHttpClient"));
    imp.core("JsonValue");
    imp.core("http.Headers");
    imp.core("jsonMapper");
    imp.sdk(&format!("errors.{client}Exception"));
    let svc_var = decapitalize(&format!("{}Service", svc.name));
    let args = call_args(g, svc, m, &mut imp);
    let prefix = format!("{}{}", decapitalize(&svc.prefix), capitalize(&g.method_name(m)));
    let call = format!("{svc_var}.{}({args})", g.method_name(m));
    let mut out = format!(
        "@WireMockTest\n@ResourceLock(\"https://github.com/wiremock/wiremock/issues/169\")\ninternal class ErrorHandlingTest {{\n\ncompanion object {{\n\nprivate val ERROR_JSON: JsonValue = JsonValue.from(mapOf(\"errorProperty\" to \"42\"))\n\nprivate val ERROR_JSON_BYTES: ByteArray = jsonMapper().writeValueAsBytes(ERROR_JSON)\n\nprivate const val HEADER_NAME: String = \"Error-Header\"\n\nprivate const val HEADER_VALUE: String = \"42\"\n\nprivate const val NOT_JSON: String = \"Not JSON\"\n}}\n\nprivate lateinit var client: {client}Client\n\n@BeforeEach\nfun beforeEach(wmRuntimeInfo: WireMockRuntimeInfo) {{\nclient = {client}OkHttpClient.builder().baseUrl(wmRuntimeInfo.httpBaseUrl){}.build()\n}}\n\n",
        test_options(g)
    );
    let chain = accessor_chain(g, svc);
    for (status, exc) in [
        (400, "BadRequestException"),
        (401, "UnauthorizedException"),
        (403, "PermissionDeniedException"),
        (404, "NotFoundException"),
        (422, "UnprocessableEntityException"),
        (429, "RateLimitException"),
        (500, "InternalServerException"),
        (999, "UnexpectedStatusCodeException"),
    ] {
        imp.sdk(&format!("errors.{exc}"));
        for raw in [false, true] {
            let (sfx, view) = if raw {
                ("WithRawResponse", ".withRawResponse()")
            } else {
                ("", "")
            };
            let _ = writeln!(
                out,
                "@Test\nfun {prefix}{status}{sfx}() {{\nval {svc_var} = {chain}{view}\nstubFor({}(anyUrl()).willReturn(status({status}).withHeader(HEADER_NAME, HEADER_VALUE).withBody(ERROR_JSON_BYTES)))\n\nval e = assertThrows<{exc}> {{ {call} }}\n\nassertThat(e.statusCode()).isEqualTo({status})\nassertThat(e.headers().toMap()).contains(entry(HEADER_NAME, listOf(HEADER_VALUE)))\nassertThat(e.body()).isEqualTo(ERROR_JSON)\n}}\n",
                verb(m)
            );
        }
    }
    let _ = writeln!(
        out,
        "@Test\nfun {prefix}InvalidJsonBody() {{\nval {svc_var} = {chain}\nstubFor({}(anyUrl()).willReturn(status(200).withHeader(HEADER_NAME, HEADER_VALUE).withBody(NOT_JSON)))\n\nval e = assertThrows<{client}Exception> {{ {call} }}\n\nassertThat(e).hasMessage(\"Error reading response\")\n}}\n",
        verb(m)
    );
    out.push_str("private fun Headers.toMap(): Map<String, List<String>> =\nmutableMapOf<String, List<String>>().also { map ->\nnames().forEach { map[it] = values(it) }\n}\n}\n");
    let path = format!("{}/services/ErrorHandlingTest.kt", g.core_test());
    (path, file(&format!("{}.services", g.root()), &imp, &out))
}

fn service_params_test(g: &Gen<'_>, svc: &Service<'_>, m: &Method) -> (String, String) {
    let client = g.client_name();
    let mut imp = Imports::new(g.root());
    let v = verb(m);
    for p in [
        "com.github.tomakehurst.wiremock.client.WireMock.anyUrl".to_owned(),
        "com.github.tomakehurst.wiremock.client.WireMock.equalTo".to_owned(),
        format!("com.github.tomakehurst.wiremock.client.WireMock.{v}"),
        format!("com.github.tomakehurst.wiremock.client.WireMock.{v}RequestedFor"),
        "com.github.tomakehurst.wiremock.client.WireMock.ok".to_owned(),
        "com.github.tomakehurst.wiremock.client.WireMock.stubFor".to_owned(),
        "com.github.tomakehurst.wiremock.client.WireMock.verify".to_owned(),
        "com.github.tomakehurst.wiremock.junit5.WireMockRuntimeInfo".to_owned(),
        "com.github.tomakehurst.wiremock.junit5.WireMockTest".to_owned(),
        "org.junit.jupiter.api.BeforeEach".to_owned(),
        "org.junit.jupiter.api.Test".to_owned(),
        "org.junit.jupiter.api.parallel.ResourceLock".to_owned(),
    ] {
        imp.add(p);
    }
    imp.sdk(&format!("client.{client}Client"));
    imp.sdk(&format!("client.okhttp.{client}OkHttpClient"));
    let name = g.params_name(svc, m);
    imp.add(format!("{}.{name}", g.models_package(svc)));
    let svc_var = decapitalize(&format!("{}Service", svc.name));
    let mn = g.method_name(m);
    let (props, nested) = param_props(g, m);
    let holder = Class {
        name: name.clone(),
        doc: String::new(),
        shape: Shape::Object {
            props: Vec::new(),
            bases: Vec::new(),
        },
        nested,
    };
    let vals = Values { g };
    let mut builder = format!("{name}.builder()");
    for p in props.iter().filter(|p| p.path || p.required()) {
        let value = vals.value(&p.ty, &p.field.wire_name, &holder, &name, &mut imp);
        let _ = write!(builder, ".{}({value})", p.name);
    }
    builder.push_str(".putAdditionalHeader(\"Secret-Header\", \"42\").putAdditionalQueryParam(\"secret_query_param\", \"42\").build()");
    let out = format!(
        "@WireMockTest\n@ResourceLock(\"https://github.com/wiremock/wiremock/issues/169\")\ninternal class ServiceParamsTest {{\n\nprivate lateinit var client: {client}Client\n\n@BeforeEach\nfun beforeEach(wmRuntimeInfo: WireMockRuntimeInfo) {{\nclient = {client}OkHttpClient.builder().baseUrl(wmRuntimeInfo.httpBaseUrl){}.build()\n}}\n\n@Test\nfun {mn}() {{\nval {svc_var} = {}\nstubFor({v}(anyUrl()).willReturn(ok(\"{{}}\")))\n\n{svc_var}.{mn}({builder})\n\nverify({v}RequestedFor(anyUrl()).withHeader(\"Secret-Header\", equalTo(\"42\")).withQueryParam(\"secret_query_param\", equalTo(\"42\")))\n}}\n}}\n",
        test_options(g),
        accessor_chain(g, svc)
    );
    let path = format!("{}/services/ServiceParamsTest.kt", g.core_test());
    (path, file(&format!("{}.services", g.root()), &imp, &out))
}

fn client_options_test(g: &Gen<'_>) -> (String, String) {
    let mut imp = Imports::new(g.root());
    for p in [
        "org.assertj.core.api.Assertions.assertThat",
        "org.junit.jupiter.api.Test",
        "org.junit.jupiter.api.extension.ExtendWith",
        "org.mockito.junit.jupiter.MockitoExtension",
        "org.mockito.kotlin.mock",
        "org.mockito.kotlin.never",
        "org.mockito.kotlin.verify",
    ] {
        imp.add(p);
    }
    imp.core("http.HttpClient");
    let opts = test_options(g);
    let out = format!(
        "@ExtendWith(MockitoExtension::class)\ninternal class ClientOptionsTest {{\n\nprivate val httpClient = mock<HttpClient>()\n\n@Test\nfun putHeader_canOverwriteDefaultHeader() {{\nval clientOptions = ClientOptions.builder().httpClient(httpClient).putHeader(\"User-Agent\", \"My User Agent\"){opts}.build()\n\nassertThat(clientOptions.headers.values(\"User-Agent\")).containsExactly(\"My User Agent\")\n}}\n\n@Test\nfun toBuilder_whenOriginalClientOptionsGarbageCollected_doesNotCloseOriginalClient() {{\nvar clientOptions = ClientOptions.builder().httpClient(httpClient){opts}.build()\nverify(httpClient, never()).close()\n\n// Overwrite the `clientOptions` variable so that the original `ClientOptions` is GC'd.\nclientOptions = clientOptions.toBuilder().build()\nSystem.gc()\nThread.sleep(100)\n\nverify(httpClient, never()).close()\n// This exists so that `clientOptions` is still reachable.\nassertThat(clientOptions).isEqualTo(clientOptions)\n}}\n}}\n"
    );
    let path = format!("{}/core/ClientOptionsTest.kt", g.core_test());
    (path, file(&format!("{}.core", g.root()), &imp, &out))
}

fn proguard_test(g: &Gen<'_>) -> (String, String) {
    let client = g.client_name();
    let mut imp = Imports::new(g.root());
    for p in [
        "com.fasterxml.jackson.module.kotlin.jacksonTypeRef",
        "kotlin.reflect.full.memberFunctions",
        "kotlin.reflect.jvm.javaMethod",
        "org.assertj.core.api.Assertions.assertThat",
        "org.junit.jupiter.api.Test",
    ] {
        imp.add(p);
    }
    imp.sdk(&format!("client.okhttp.{client}OkHttpClient"));
    imp.core("jsonMapper");
    let mut out = format!(
        "internal class ProGuardCompatibilityTest {{\n\ncompanion object {{\n\n@JvmStatic\nfun main(args: Array<String>) {{\n// To debug that we're using the right JAR.\nval jarPath = this::class.java.getProtectionDomain().codeSource.location\nprintln(\"JAR being used: $jarPath\")\n\n// We have to manually run the test methods instead of using the JUnit runner because it\n// seems impossible to get working with R8.\nval test = ProGuardCompatibilityTest()\ntest::class.memberFunctions.asSequence().filter {{ function ->\nfunction.javaMethod?.isAnnotationPresent(Test::class.java) == true\n}}.forEach {{ it.call(test) }}\n}}\n}}\n\n@Test\nfun proguardRules() {{\nval rulesFile =\njavaClass.classLoader.getResourceAsStream(\"META-INF/proguard/{}-core.pro\")\n\nassertThat(rulesFile).isNotNull()\n}}\n\n@Test\nfun client() {{\nval client = {client}OkHttpClient.builder(){}.build()\n\nassertThat(client).isNotNull()\n",
        g.target.artifact,
        test_options(g)
    );
    for i in g.top_services() {
        let _ = writeln!(out, "assertThat(client.{}()).isNotNull()", g.services[i].accessor);
    }
    out.push_str("}\n");
    let vals = Values { g };
    if let Some(d) = g.client.models.iter().find(|d| matches!(d.ty, Type::Object(_)))
        && let Some(c) = vals.model_class(&d.r#ref)
    {
        {
            let name = c.name.clone();
            imp.add(g.mapper.model_path(&d.r#ref));
            let var = decapitalize(&name);
            let built = vals.build(&c, &name, &mut imp);
            let _ = writeln!(
                out,
                "\n@Test\nfun {var}Roundtrip() {{\nval jsonMapper = jsonMapper()\nval {var} = {built}\n\nval roundtripped{name} = jsonMapper.readValue(jsonMapper.writeValueAsString({var}), jacksonTypeRef<{name}>(),)\n\nassertThat(roundtripped{name}).isEqualTo({var})\n}}"
            );
        }
    }
    out.push_str("}\n");
    let path = format!(
        "{}-proguard-test/src/test/kotlin/{}/proguard/ProGuardCompatibilityTest.kt",
        g.target.artifact,
        g.root_path()
    );
    (path, file(&format!("{}.proguard", g.root()), &imp, &out))
}
