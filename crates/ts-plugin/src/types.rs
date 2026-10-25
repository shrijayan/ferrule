//! The one type mapping: neutral [`Type`]s to TypeScript types, inline objects declared as
//! interfaces in the namespace of the type that holds them (`EmbeddingResponse.Usage`).

use ferrule_sdk_model::model::{Field, Literal, Object, Scalar, Type, TypeRef};

use crate::ctx::Ctx;
use crate::js;
use crate::naming::{singular, type_name, words_of};
use crate::pretty::Ty;

/// A declaration to print: an interface or a type alias, with the interfaces its fields need.
#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub name: String,
    pub description: String,
    pub deprecated: bool,
    pub kind: DeclKind,
    /// Declared in `export namespace <name>`.
    pub nested: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    Interface {
        extends: Vec<String>,
        fields: Vec<FieldDecl>,
    },
    Alias(Ty),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    /// The key as written (quoted when it is not an identifier).
    pub key: String,
    pub optional: bool,
    pub ty: Ty,
    pub description: String,
    pub deprecated: bool,
}

/// Maps types for one file: references to models of other files go through their namespace
/// import (`v1API.Document`), which the file then imports.
pub struct Mapper<'c, 'a> {
    pub ctx: &'c Ctx<'a>,
    /// The module of the file being written (from `src/`, no `.ts`).
    pub module: String,
    /// Namespace imports the file needs: (namespace, module).
    pub imports: Vec<(String, String)>,
}

fn scalar(s: Scalar) -> &'static str {
    match s {
        Scalar::Unknown | Scalar::Binary => "unknown",
        Scalar::Null => "null",
        Scalar::Boolean => "boolean",
        Scalar::String | Scalar::Uuid | Scalar::DateTime | Scalar::Date | Scalar::Bytes => "string",
        _ => "number",
    }
}

fn literal(l: &Literal) -> String {
    match l {
        Literal::String(s) => js::string_literal(s),
        Literal::Number(n) => js::number_to_string(*n),
        Literal::Boolean(b) => b.to_string(),
    }
}

fn union_of(mut members: Vec<Ty>) -> Ty {
    if members.len() == 1 {
        members.remove(0)
    } else {
        Ty::Union(members)
    }
}

/// Members of a union, nested unions flattened.
fn push_member(out: &mut Vec<Ty>, t: Ty) {
    match t {
        Ty::Union(ms) => out.extend(ms),
        other => out.push(other),
    }
}

impl<'c, 'a> Mapper<'c, 'a> {
    #[must_use]
    pub fn new(ctx: &'c Ctx<'a>, module: &str) -> Self {
        Self {
            ctx,
            module: module.to_owned(),
            imports: Vec::new(),
        }
    }

    /// The name a model is referred to by from this file.
    pub fn model_name(&mut self, r: &TypeRef) -> String {
        let module = self.ctx.model_module(r);
        if module == self.module {
            return r.name.written.clone();
        }
        let ns = self.ctx.model_namespace(r);
        if !self.imports.iter().any(|(n, _)| *n == ns) {
            self.imports.push((ns.clone(), module));
        }
        format!("{ns}.{}", r.name.written)
    }

    /// The fields of an object in the order the spec writes them.
    #[must_use]
    pub fn ordered<'o>(&self, fields: &'o [Field]) -> Vec<&'o Field> {
        let names: Vec<&str> = fields.iter().map(|f| f.wire_name.as_str()).collect();
        match self.ctx.extras.property_order(&names) {
            Some(order) => order
                .iter()
                .filter_map(|n| fields.iter().find(|f| f.wire_name == *n))
                .collect(),
            None => fields.iter().collect(),
        }
    }

    /// The type of a value, inline objects declared into `nested` and named from `words`.
    pub fn ty(&mut self, t: &Type, ns: &str, words: &[String], nested: &mut Vec<Decl>) -> Ty {
        match t {
            Type::Scalar { scalar: s } => Ty::name(scalar(*s)),
            Type::Enum { members } => union_of(members.iter().map(|m| Ty::Name(literal(&m.value))).collect()),
            Type::Array { items } => {
                let item = self.ty(items, ns, &singular(words), nested);
                Ty::Generic("Array".to_owned(), vec![item])
            }
            Type::Map { values } => {
                let v = self.ty(values, ns, words, nested);
                Ty::Object(vec![("[key: string]".to_owned(), v)])
            }
            Type::Nullable { inner } => {
                let mut members = Vec::new();
                push_member(&mut members, self.ty(inner, ns, words, nested));
                members.push(Ty::name("null"));
                Ty::Union(members)
            }
            Type::Ref(r) => Ty::Name(self.model_name(r)),
            Type::Union(u) => {
                let mut members = Vec::new();
                for (i, v) in u.variants.iter().enumerate() {
                    let name = v.name.as_ref().map_or_else(
                        || vec!["union".to_owned(), "member".to_owned(), i.to_string()],
                        words_of,
                    );
                    push_member(&mut members, self.ty(&v.ty, ns, &name, nested));
                }
                union_of(members)
            }
            Type::Object(o) => self.object(o, ns, words, nested),
        }
    }

    fn object(&mut self, o: &Object, ns: &str, words: &[String], nested: &mut Vec<Decl>) -> Ty {
        if o.fields.is_empty() && o.bases.is_empty() {
            return match &o.extra {
                Some(extra) => {
                    let v = self.ty(extra, ns, words, nested);
                    Ty::Object(vec![("[key: string]".to_owned(), v)])
                }
                None => Ty::name("unknown"),
            };
        }
        let name = type_name(words, &self.ctx.client.casings);
        let decl = self.interface(&name, o, o.description.clone().unwrap_or_default(), false, None);
        nested.push(decl);
        Ty::Name(if ns.is_empty() { name } else { format!("{ns}.{name}") })
    }

    /// The spec schema of an object: the named component, else the first schema with exactly its
    /// properties.
    fn schema_of(&self, o: &Object, component: Option<&str>) -> Option<&'c serde_json::Value> {
        let extras = self.ctx.extras;
        component.and_then(|c| extras.component(c)).or_else(|| {
            let names: Vec<&str> = o.fields.iter().map(|f| f.wire_name.as_str()).collect();
            extras.object_schema(&names)
        })
    }

    /// An interface for an object, its inline objects nested.
    pub fn interface(
        &mut self,
        name: &str,
        o: &Object,
        mut description: String,
        deprecated: bool,
        component: Option<&str>,
    ) -> Decl {
        let extras = self.ctx.extras;
        let schema = self.schema_of(o, component);
        if schema.is_some_and(|s| extras.is_title_only(s, &description)) {
            description.clear();
        }
        let mut nested = Vec::new();
        let extends = o.bases.iter().map(|b| self.model_name(b)).collect();
        let fields = self
            .ordered(&o.fields)
            .into_iter()
            .map(|f| self.described_field(f, name, &mut nested, schema))
            .collect();
        Decl {
            name: name.to_owned(),
            description,
            deprecated,
            kind: DeclKind::Interface { extends, fields },
            nested,
        }
    }

    /// One interface member of an object whose spec schema is `schema`.
    pub fn described_field(
        &mut self,
        f: &Field,
        ns: &str,
        nested: &mut Vec<Decl>,
        schema: Option<&serde_json::Value>,
    ) -> FieldDecl {
        let extras = self.ctx.extras;
        let mut decl = self.field(f, ns, nested);
        if let Some(d) = schema
            .and_then(|s| extras.property(s, &f.wire_name))
            .and_then(|p| extras.property_description(p))
        {
            decl.description = d;
        }
        decl
    }

    /// One interface member.
    pub fn field(&mut self, f: &Field, ns: &str, nested: &mut Vec<Decl>) -> FieldDecl {
        let ty = self.ty(&f.ty, ns, &words_of(&f.name), nested);
        FieldDecl {
            key: js::property_key(&f.wire_name),
            optional: !f.required,
            ty,
            description: f.description.clone(),
            deprecated: f.deprecated,
        }
    }

    /// A model's declaration; `component` is its `components.schemas` key.
    pub fn model(
        &mut self,
        name: &str,
        ty: &Type,
        description: &str,
        deprecated: bool,
        component: Option<&str>,
    ) -> Decl {
        match ty {
            Type::Object(o) if !(o.fields.is_empty() && o.bases.is_empty()) => {
                self.interface(name, o, description.to_owned(), deprecated, component)
            }
            Type::Object(o) if o.extra.is_none() => Decl {
                name: name.to_owned(),
                description: description.to_owned(),
                deprecated,
                kind: DeclKind::Alias(Ty::name("unknown")),
                nested: Vec::new(),
            },
            other => {
                let mut nested = Vec::new();
                let t = self.ty(other, name, &[], &mut nested);
                Decl {
                    name: name.to_owned(),
                    description: description.to_owned(),
                    deprecated,
                    kind: DeclKind::Alias(t),
                    nested,
                }
            }
        }
    }
}
