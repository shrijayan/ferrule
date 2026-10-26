//! Model types as Rust types (ADR 0019), in one function: [`render`].
//!
//! Named models are declared once in their module ([`declare_model`]); inline objects, enums and
//! unions are declared in the module that uses them, named by position (`AgencyRetrieveResponse`
//! then its field `data` gives `AgencyRetrieveResponseData`). Each rendering also gives an
//! example value, used by the generated tests; it names types by their `crate::` path and
//! leaves each model's example as a placeholder, filled by [`Examples::resolve`].

use std::collections::HashMap;
use std::fmt::Write as _;

use ferrule_sdk_model::model::{EnumMember, Field, Literal, ModelDecl, Object, Scalar, Type, Union, Variant};

use crate::context::{Gen, Module};
use crate::doc::doc;
use crate::naming::{Taken, field as field_name, pascal};

/// A Rust type and an example value of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// As the declaring module spells it: `NoteContent`.
    pub ty: String,
    /// From anywhere in the crate: `crate::shared::NoteContent`.
    pub path: String,
    pub example: String,
    /// Whether the type is `String`, which setters take as `impl Into<String>`.
    pub string: bool,
}

impl Rendered {
    fn new(ty: impl Into<String>, example: impl Into<String>) -> Self {
        let ty = ty.into();
        Self {
            path: ty.clone(),
            ty,
            example: example.into(),
            string: false,
        }
    }

    /// The type with its path from anywhere in the crate.
    fn at(mut self, path: String) -> Self {
        self.path = path;
        self
    }
}

/// Where a type is rendered: the module that declares its inline types and, inside a named
/// model, that model (whose recursive references are boxed).
pub struct Cx<'g, 'c> {
    pub g: &'g Gen<'c>,
    pub module: &'g mut Module,
    pub model: Option<&'g str>,
}

/// Opens and closes a model's example, which [`Examples::resolve`] fills.
const OPEN: char = '\u{1}';
const CLOSE: char = '\u{2}';

fn placeholder(schema: &str) -> String {
    format!("{OPEN}{schema}{CLOSE}")
}

/// `ty` as a Rust type, declaring the inline types it needs under `name`.
pub fn render(cx: &mut Cx<'_, '_>, ty: &Type, name: &str, description: Option<&str>) -> Rendered {
    render_at(cx, ty, name, description, false)
}

/// `indirect`: behind a `Vec` or map already, so a recursive reference needs no box.
fn render_at(cx: &mut Cx<'_, '_>, ty: &Type, name: &str, description: Option<&str>, indirect: bool) -> Rendered {
    match ty {
        Type::Scalar { scalar } => scalar_type(*scalar),
        Type::Enum { members } => enumeration(cx, members, name, description),
        Type::Array { items } => {
            let inner = render_at(cx, items, name, description, true);
            Rendered::new(format!("Vec<{}>", inner.ty), "Vec::new()").at(format!("Vec<{}>", inner.path))
        }
        Type::Map { values } => map(&render_at(cx, values, name, description, true)),
        Type::Object(o) => object(cx, o, name, description),
        Type::Union(u) => union(cx, u, name, description),
        Type::Ref(r) => {
            let Some(info) = cx.g.models.get(&r.schema) else {
                return Rendered::new("serde_json::Value", "serde_json::Value::Null");
            };
            let path = info.path();
            if !indirect && cx.model.is_some_and(|m| cx.g.reaches(&r.schema, m)) {
                Rendered::new(format!("Box<{path}>"), format!("Box::new({})", placeholder(&r.schema)))
            } else {
                Rendered::new(path, placeholder(&r.schema))
            }
        }
        Type::Nullable { inner } => {
            let inner = render_at(cx, inner, name, description, indirect);
            Rendered::new(format!("Option<{}>", inner.ty), "None").at(format!("Option<{}>", inner.path))
        }
    }
}

fn map(inner: &Rendered) -> Rendered {
    let map = |t: &str| format!("std::collections::BTreeMap<String, {t}>");
    Rendered::new(map(&inner.ty), "std::collections::BTreeMap::new()").at(map(&inner.path))
}

fn scalar_type(scalar: Scalar) -> Rendered {
    let (ty, example) = match scalar {
        Scalar::Unknown | Scalar::Null => ("serde_json::Value", "serde_json::Value::Null"),
        Scalar::Boolean => ("bool", "true"),
        Scalar::String | Scalar::Uuid | Scalar::DateTime | Scalar::Date | Scalar::Bytes | Scalar::Binary => {
            return Rendered {
                ty: "String".to_owned(),
                path: "String".to_owned(),
                example: format!("{:?}.to_owned()", fake_string(scalar)),
                string: true,
            };
        }
        Scalar::Int | Scalar::Int64 => ("i64", "0"),
        Scalar::Int8 => ("i8", "0"),
        Scalar::Int16 => ("i16", "0"),
        Scalar::Int32 => ("i32", "0"),
        Scalar::Uint8 => ("u8", "0"),
        Scalar::Uint16 => ("u16", "0"),
        Scalar::Uint32 => ("u32", "0"),
        Scalar::Uint64 => ("u64", "0"),
        Scalar::Float => ("f32", "0.0"),
        Scalar::Double => ("f64", "0.0"),
    };
    Rendered::new(ty, example)
}

/// The string the fake server and the tests use for a string scalar.
#[must_use]
pub fn fake_string(scalar: Scalar) -> &'static str {
    match scalar {
        Scalar::Uuid => "182bd5e5-6e1a-4fe4-a799-aa6d9a6ab26e",
        Scalar::DateTime => "2019-12-27T18:11:19.117Z",
        Scalar::Date => "2019-12-27",
        _ => "string",
    }
}

/// The derives every declared type has, for a module that now uses serde; `copy` adds `Copy`,
/// `Eq` and `Hash` (field-less enums).
fn derives(module: &mut Module, copy: bool) -> String {
    module.serde = true;
    let (before, after) = if copy { ("Copy, ", "Eq, Hash, ") } else { ("", "") };
    format!("#[derive(Debug, Clone, {before}PartialEq, {after}Serialize, Deserialize)]\n")
}

fn enumeration(cx: &mut Cx<'_, '_>, members: &[EnumMember], name: &str, description: Option<&str>) -> Rendered {
    let strings: Vec<&str> = members
        .iter()
        .filter_map(|m| match &m.value {
            Literal::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    if strings.len() != members.len() || strings.is_empty() {
        return non_string_enum(members);
    }
    let name = cx.module.taken.claim(name);
    let mut variants = Taken::default();
    let mut body = String::new();
    let mut first = None;
    for m in members {
        let Literal::String(value) = &m.value else { continue };
        let variant = variants.claim(&pascal(&m.name.words, "V", "Empty"));
        first.get_or_insert_with(|| variant.clone());
        let _ = writeln!(body, "    #[serde(rename = {value:?})]\n    {variant},");
    }
    let unknown = variants.claim("Unknown");
    let _ = writeln!(
        body,
        "    /// A value this version of the SDK does not know.\n    #[serde(other, skip_serializing)]\n    {unknown},"
    );
    let mut item = doc(description.unwrap_or_default(), "");
    item.push_str(&derives(cx.module, true));
    let _ = write!(item, "pub enum {name} {{\n{body}}}\n");
    cx.module.items.push(item);
    let path = local_path(cx.module, &name);
    let example = format!("{path}::{}", first.unwrap_or_default());
    Rendered::new(name, example).at(path)
}

/// An enum of numbers or booleans is its scalar; a mixed one is any JSON value.
fn non_string_enum(members: &[EnumMember]) -> Rendered {
    let all = |f: fn(&Literal) -> bool| members.iter().all(|m| f(&m.value));
    if all(|l| matches!(l, Literal::Number(n) if n.fract() == 0.0)) {
        let first = members.first().map_or(0.0, |m| match m.value {
            Literal::Number(n) => n,
            _ => 0.0,
        });
        Rendered::new("i64", format!("{first}"))
    } else if all(|l| matches!(l, Literal::Number(_))) {
        Rendered::new("f64", "0.0")
    } else if all(|l| matches!(l, Literal::Boolean(_))) {
        Rendered::new("bool", "true")
    } else {
        Rendered::new("serde_json::Value", "serde_json::Value::Null")
    }
}

/// An empty item to fill once the types a declaration needs are declared after it, so that a
/// type comes before the types of its fields.
fn reserve(module: &mut Module) -> usize {
    module.items.push(String::new());
    module.items.len() - 1
}

/// `crate::<module>::<name>`.
fn local_path(module: &Module, name: &str) -> String {
    format!("crate::{}::{name}", module.path.join("::"))
}

fn object(cx: &mut Cx<'_, '_>, o: &Object, name: &str, description: Option<&str>) -> Rendered {
    if o.fields.is_empty() && o.bases.is_empty() {
        return match &o.extra {
            Some(extra) => map(&render_at(cx, extra, name, None, true)),
            None => Rendered::new("serde_json::Value", "serde_json::Value::Null"),
        };
    }
    let name = cx.module.taken.claim(name);
    let (example, _) = declare_struct(cx, o, &name, description.or(o.description.as_deref()));
    let path = local_path(cx.module, &name);
    Rendered::new(name, example).at(path)
}

/// The fields of a struct as declared: identifier, field and rendering.
struct StructField {
    ident: String,
    wire: String,
    rendered: Rendered,
    optional: bool,
    description: String,
}

/// Declares struct `name`; returns a literal of it with every optional field `None`, and its
/// fields.
fn declare_struct(
    cx: &mut Cx<'_, '_>,
    o: &Object,
    name: &str,
    description: Option<&str>,
) -> (String, Vec<StructField>) {
    let slot = reserve(cx.module);
    let mut idents = Taken::default();
    let mut lines = String::new();
    let mut example = Vec::new();
    for base in &o.bases {
        let Some(info) = cx.g.models.get(&base.schema) else {
            continue;
        };
        let ident = idents.claim(&field_name(&base.name.words));
        let _ = writeln!(lines, "    #[serde(flatten)]\n    pub {ident}: {},", info.path());
        example.push(format!("{ident}: {}", placeholder(&base.schema)));
    }
    let fields = struct_fields(cx, o, name, &mut idents);
    for f in &fields {
        if f.optional {
            example.push(format!("{}: None", f.ident));
        } else {
            example.push(format!("{}: {}", f.ident, f.rendered.example));
        }
        lines.push_str(&doc(&f.description, "    "));
        if f.ident.trim_start_matches("r#") != f.wire {
            let _ = writeln!(lines, "    #[serde(rename = {:?})]", f.wire);
        }
        if f.optional {
            lines.push_str("    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n");
            let _ = writeln!(lines, "    pub {}: Option<{}>,", f.ident, f.rendered.ty);
        } else {
            if f.rendered.ty == "serde_json::Value" {
                lines.push_str("    #[serde(default)]\n");
            }
            let _ = writeln!(lines, "    pub {}: {},", f.ident, f.rendered.ty);
        }
    }
    if let Some(extra) = &o.extra {
        let inner = render_at(cx, extra, &format!("{name}Extra"), None, true);
        let ident = idents.claim("extra");
        let _ = writeln!(
            lines,
            "    /// Properties the type does not declare.\n    #[serde(flatten)]\n    pub {ident}: std::collections::BTreeMap<String, {}>,",
            inner.ty
        );
        example.push(format!("{ident}: std::collections::BTreeMap::new()"));
    }
    let mut item = doc(description.unwrap_or_default(), "");
    item.push_str(&derives(cx.module, false));
    let _ = write!(item, "pub struct {name} {{\n{lines}}}\n");
    cx.module.items[slot] = item;
    (
        format!("{} {{ {} }}", local_path(cx.module, name), example.join(", ")),
        fields,
    )
}

fn struct_fields(cx: &mut Cx<'_, '_>, o: &Object, name: &str, idents: &mut Taken) -> Vec<StructField> {
    o.fields
        .iter()
        .map(|f: &Field| {
            let ident = idents.claim(&field_name(&f.name.words));
            let (ty, nullable) = match &f.ty {
                Type::Nullable { inner } => (inner.as_ref(), true),
                ty => (ty, false),
            };
            let child = format!("{name}{}", pascal(&f.name.words, "F", "Field"));
            let rendered = render(
                cx,
                ty,
                &child,
                Some(&f.description).filter(|d| !d.is_empty()).map(String::as_str),
            );
            StructField {
                ident,
                wire: f.wire_name.clone(),
                rendered,
                optional: !f.required || nullable,
                description: f.description.clone(),
            }
        })
        .collect()
}

fn union(cx: &mut Cx<'_, '_>, u: &Union, name: &str, description: Option<&str>) -> Rendered {
    let non_null: Vec<&Variant> = u
        .variants
        .iter()
        .filter(|v| !matches!(v.ty, Type::Scalar { scalar: Scalar::Null }))
        .collect();
    let nullable = non_null.len() < u.variants.len();
    let rendered = match non_null.as_slice() {
        [] => Rendered::new("serde_json::Value", "serde_json::Value::Null"),
        [one] => render(cx, &one.ty, name, description),
        _ => declare_union(cx, &non_null, name, description),
    };
    if nullable && !rendered.ty.starts_with("Option<") && rendered.ty != "serde_json::Value" {
        Rendered::new(format!("Option<{}>", rendered.ty), "None").at(format!("Option<{}>", rendered.path))
    } else {
        rendered
    }
}

fn declare_union(cx: &mut Cx<'_, '_>, variants: &[&Variant], name: &str, description: Option<&str>) -> Rendered {
    let name = cx.module.taken.claim(name);
    let slot = reserve(cx.module);
    let mut taken = Taken::default();
    let mut body = String::new();
    let mut first = None;
    for v in variants {
        let variant = taken.claim(&variant_name(cx, v));
        let rendered = render(cx, &v.ty, &format!("{name}{variant}"), None);
        let _ = writeln!(body, "    {variant}({}),", rendered.ty);
        if first.is_none() {
            first = Some((variant, rendered.example));
        }
    }
    let mut item = doc(description.unwrap_or_default(), "");
    item.push_str(&derives(cx.module, false));
    item.push_str("#[serde(untagged)]\n#[allow(clippy::large_enum_variant)]\n");
    let _ = write!(item, "pub enum {name} {{\n{body}}}\n");
    cx.module.items[slot] = item;
    let (variant, example) = first.unwrap_or_default();
    let path = local_path(cx.module, &name);
    Rendered::new(name, format!("{path}::{variant}({example})")).at(path)
}

fn variant_name(cx: &Cx<'_, '_>, v: &Variant) -> String {
    if let Some(name) = &v.name {
        return pascal(&name.words, "V", "Variant");
    }
    match &v.ty {
        Type::Scalar { scalar } => match scalar {
            Scalar::Boolean => "Bool",
            s if s.is_integer() => "Integer",
            Scalar::Float | Scalar::Double => "Number",
            Scalar::Unknown | Scalar::Null => "Unknown",
            _ => "String",
        }
        .to_owned(),
        Type::Ref(r) => {
            cx.g.models
                .get(&r.schema)
                .map_or_else(|| "Model".to_owned(), |i| i.name.clone())
        }
        Type::Array { .. } => "Array".to_owned(),
        Type::Map { .. } => "Map".to_owned(),
        Type::Enum { .. } => "Enum".to_owned(),
        Type::Object(o) => o.origin.as_deref().map_or_else(
            || "Object".to_owned(),
            |s| pascal(&crate::naming::split_words(s), "V", "Object"),
        ),
        Type::Union(_) | Type::Nullable { .. } => "Variant".to_owned(),
    }
}

/// Declares the named model `d` in its module; returns its example.
pub fn declare_model(cx: &mut Cx<'_, '_>, d: &ModelDecl) -> String {
    let Some(info) = cx.g.models.get(&d.r#ref.schema).cloned() else {
        return String::new();
    };
    let description = Some(d.description.as_str()).filter(|s| !s.is_empty());
    match &d.ty {
        Type::Object(o) if !o.fields.is_empty() || !o.bases.is_empty() => {
            let (example, fields) = declare_struct(cx, o, &info.name, description.or(o.description.as_deref()));
            let fields = fields.into_iter().map(|f| (f.wire, f.rendered)).collect();
            cx.g.fields.borrow_mut().insert(d.r#ref.schema.clone(), fields);
            example
        }
        ty => {
            // Anything else is an alias of the type it renders as, named for the model.
            let slot = reserve(cx.module);
            let rendered = render(cx, ty, &format!("{}Value", info.name), description);
            if rendered.ty == info.name {
                cx.module.items.remove(slot);
                return rendered.example;
            }
            let mut item = doc(description.unwrap_or_default(), "");
            let _ = writeln!(item, "pub type {} = {};", info.name, rendered.ty);
            cx.module.items[slot] = item;
            rendered.example
        }
    }
}

/// Every model's example, so that a placeholder can be filled.
#[derive(Debug, Default)]
pub struct Examples(pub HashMap<String, String>);

impl Examples {
    /// `example` with every model placeholder filled, `crate::` replaced by `krate`.
    #[must_use]
    pub fn resolve(&self, example: &str, krate: &str) -> String {
        self.fill(example, 0).replace("crate::", &format!("{krate}::"))
    }

    fn fill(&self, example: &str, depth: usize) -> String {
        let mut out = String::new();
        let mut rest = example;
        while let Some(start) = rest.find(OPEN) {
            out.push_str(&rest[..start]);
            let after = &rest[start + OPEN.len_utf8()..];
            let end = after.find(CLOSE).unwrap_or(after.len());
            let schema = &after[..end];
            let inner = self.0.get(schema).map_or("serde_json::Value::Null", String::as_str);
            // A recursive model's required self-reference has no finite example.
            out.push_str(&if depth < 16 {
                self.fill(inner, depth + 1)
            } else {
                String::new()
            });
            rest = after.get(end + CLOSE.len_utf8()..).unwrap_or_default();
        }
        out.push_str(rest);
        out
    }
}
