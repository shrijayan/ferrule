//! The one type mapper: model types to JVM types ([`Jt`]), and inline objects and enums to the
//! nested classes Stainless declares for them ([`Class`]), named by position (the field's name,
//! array items singular).

use ferrule_sdk_model::model::{Client, EnumMember, Field, Object, Scalar, Type, TypeRef};

use crate::meta::Flavor;
use crate::naming::{Casings, camel, pascal, singular, words};

/// What kind of class a [`Jt::Class`] is: each kind validates and scores itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Object,
    Enum,
}

/// A JVM type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Jt {
    String,
    Long,
    Int,
    Double,
    Float,
    Boolean,
    OffsetDateTime,
    LocalDate,
    /// Any JSON value.
    Value,
    List(Box<Jt>),
    /// A class: nested in the file (`import` `None`) or a model of its own file.
    Class {
        name: String,
        kind: ClassKind,
        import: Option<String>,
    },
}

impl Jt {
    /// The Kotlin spelling; `qualify_list` spells `kotlin.collections.List` where a nested class
    /// named `List` is in scope.
    #[must_use]
    pub fn kt(&self, qualify_list: bool) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Long => "Long".to_owned(),
            Self::Int => "Int".to_owned(),
            Self::Double => "Double".to_owned(),
            Self::Float => "Float".to_owned(),
            Self::Boolean => "Boolean".to_owned(),
            Self::OffsetDateTime => "OffsetDateTime".to_owned(),
            Self::LocalDate => "LocalDate".to_owned(),
            Self::Value => "JsonValue".to_owned(),
            Self::List(items) => {
                let list = if qualify_list {
                    "kotlin.collections.List"
                } else {
                    "List"
                };
                format!("{list}<{}>", items.kt(qualify_list))
            }
            Self::Class { name, .. } => name.clone(),
        }
    }

    /// How docs name the type: `[String]`, or `` `List<String>` `` for a generic one.
    #[must_use]
    pub fn doc(&self) -> String {
        match self {
            Self::List(_) => format!("`{}`", self.kt(false)),
            _ => format!("[{}]", self.kt(false)),
        }
    }

    /// An unboxed JVM primitive: builders add overloads taking it for nullable values.
    #[must_use]
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Self::Long | Self::Int | Self::Double | Self::Float | Self::Boolean
        )
    }

    /// The class kind, for a class.
    #[must_use]
    pub fn class_kind(&self) -> Option<ClassKind> {
        match self {
            Self::Class { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// The item type of a list.
    #[must_use]
    pub fn items(&self) -> Option<&Jt> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// The imports the type needs, as paths relative to the SDK root (`core.JsonValue`) or full
    /// (`java.time.OffsetDateTime`).
    pub fn imports(&self, out: &mut Vec<String>) {
        match self {
            Self::OffsetDateTime => out.push("java.time.OffsetDateTime".to_owned()),
            Self::LocalDate => out.push("java.time.LocalDate".to_owned()),
            Self::Value => out.push(".core.JsonValue".to_owned()),
            Self::List(items) => items.imports(out),
            Self::Class { import: Some(p), .. } => out.push(p.clone()),
            _ => {}
        }
    }
}

/// A class the emitters declare: a model, a response, or a nested class.
#[derive(Debug, Clone)]
pub struct Class<'a> {
    pub name: String,
    pub doc: String,
    pub shape: Shape<'a>,
    /// Classes declared inside this one, in field order.
    pub nested: Vec<Class<'a>>,
}

#[derive(Debug, Clone)]
pub enum Shape<'a> {
    Object {
        props: Vec<Prop<'a>>,
        /// Named models whose fields this one starts with (`allOf`).
        bases: Vec<TypeRef>,
    },
    Enum {
        members: Vec<&'a EnumMember>,
        /// The members' JVM type (`String` for string enums).
        ty: Jt,
    },
}

/// One field of an object class.
#[derive(Debug, Clone)]
pub struct Prop<'a> {
    pub field: &'a Field,
    /// `agencyId`.
    pub name: String,
    pub ty: Jt,
    /// The value may be JSON `null`.
    pub nullable: bool,
}

impl Prop<'_> {
    #[must_use]
    pub fn required(&self) -> bool {
        self.field.required
    }

    #[must_use]
    pub fn wire(&self) -> &str {
        &self.field.wire_name
    }
}

/// Maps model types for one SDK.
pub struct Mapper<'a> {
    pub client: &'a Client,
    pub casings: Casings<'a>,
    /// The SDK's root package (`org.onebusaway`).
    pub root: &'a str,
    pub flavor: Flavor,
    /// The package (below `models`) of each resource model, by schema: see `emit::place`.
    pub placement: Vec<(String, Vec<String>)>,
}

impl<'a> Mapper<'a> {
    /// The full import path of a named model's class.
    #[must_use]
    pub fn model_path(&self, r: &TypeRef) -> String {
        format!("{}.{}", self.model_package(r), self.model_name(r))
    }

    /// The class name of a named model.
    #[must_use]
    pub fn model_name(&self, r: &TypeRef) -> String {
        pascal(&words(&r.name), self.casings)
    }

    /// The package of a named model: `models` for a shared one, else `models.<resource path>`.
    #[must_use]
    pub fn model_package(&self, r: &TypeRef) -> String {
        let mut pkg = format!("{}.models", self.root);
        if let Some((_, segs)) = self.placement.iter().find(|(schema, _)| *schema == r.schema) {
            for seg in segs {
                pkg.push('.');
                pkg.push_str(seg);
            }
            return pkg;
        }
        let resources = &r.path[..r.path.len().saturating_sub(1)];
        if resources.first().is_some_and(|n| n.written != "shared") {
            for n in resources {
                pkg.push('.');
                pkg.push_str(&crate::naming::package(&words(n)));
            }
        }
        pkg
    }

    /// The JVM type of `ty` for a field named `field_words`, declaring the nested classes it
    /// needs in `nested`.
    pub fn jt(&self, ty: &'a Type, field_words: &[String], doc: &str, nested: &mut Vec<Class<'a>>) -> Jt {
        match ty {
            Type::Scalar { scalar } => scalar_jt(*scalar),
            Type::Nullable { inner } => self.jt(inner, field_words, doc, nested),
            Type::Array { items } => Jt::List(Box::new(self.jt(items, &singular(field_words), doc, nested))),
            Type::Map { .. } | Type::Union(_) => Jt::Value,
            Type::Enum { members } => {
                let name = pascal(field_words, self.casings);
                let member_ty = members.first().map_or(Jt::String, |m| match m.value {
                    ferrule_sdk_model::model::Literal::Number(n) if n.fract() == 0.0 => Jt::Long,
                    ferrule_sdk_model::model::Literal::Number(_) => Jt::Double,
                    ferrule_sdk_model::model::Literal::Boolean(_) => Jt::Boolean,
                    ferrule_sdk_model::model::Literal::String(_) => Jt::String,
                });
                nested.push(Class {
                    name: name.clone(),
                    doc: doc.trim().to_owned(),
                    shape: Shape::Enum {
                        members: members.iter().collect(),
                        ty: member_ty,
                    },
                    nested: Vec::new(),
                });
                Jt::Class {
                    name,
                    kind: ClassKind::Enum,
                    import: None,
                }
            }
            Type::Object(o) => {
                let name = pascal(field_words, self.casings);
                let class = self.object_class(name.clone(), o, o.description.clone().unwrap_or_default());
                nested.push(class);
                Jt::Class {
                    name,
                    kind: ClassKind::Object,
                    import: None,
                }
            }
            Type::Ref(r) => {
                let kind = match self.client.model(r).map(|d| &d.ty) {
                    Some(Type::Enum { .. }) => ClassKind::Enum,
                    _ => ClassKind::Object,
                };
                Jt::Class {
                    name: self.model_name(r),
                    kind,
                    import: Some(self.model_path(r)),
                }
            }
        }
    }

    /// The fields of `o` in Stainless's order: those of the models it extends first (`allOf`
    /// flattened), then its own; within each, required fields first, each sorted by wire name.
    #[must_use]
    pub fn all_fields(&self, o: &'a Object) -> Vec<&'a Field> {
        let mut out: Vec<&'a Field> = Vec::new();
        for base in &o.bases {
            if let Some(bo) = self.client.model(base).and_then(|d| self.client.object_of(&d.ty)) {
                for f in self.all_fields(bo) {
                    if !out.iter().any(|x| x.wire_name == f.wire_name) {
                        out.push(f);
                    }
                }
            }
        }
        let own: Vec<&'a Field> = o
            .fields
            .iter()
            .filter(|f| !out.iter().any(|x| x.wire_name == f.wire_name))
            .collect();
        out.extend(sorted_fields(own));
        out
    }

    /// The class of an object type, its fields in [`Mapper::all_fields`] order.
    #[must_use]
    pub fn object_class(&self, name: String, o: &'a Object, doc: String) -> Class<'a> {
        let mut nested = Vec::new();
        let mut props = Vec::new();
        for f in self.all_fields(o) {
            let w = words(&f.name);
            let ty = self.jt(&f.ty, &w, &f.description, &mut nested);
            props.push(Prop {
                field: f,
                name: camel(&w, self.casings),
                ty,
                nullable: matches!(f.ty, Type::Nullable { .. }),
            });
        }
        Class {
            name,
            doc,
            shape: Shape::Object {
                props,
                bases: o.bases.clone(),
            },
            nested,
        }
    }
}

/// Stainless's order of properties and parameters: required before optional; within each, `id`
/// first, then by wire name ignoring case.
#[must_use]
pub fn sorted_fields<'f>(fields: impl IntoIterator<Item = &'f Field>) -> Vec<&'f Field> {
    let mut v: Vec<&Field> = fields.into_iter().collect();
    v.sort_by(|a, b| {
        b.required
            .cmp(&a.required)
            .then_with(|| (b.wire_name == "id").cmp(&(a.wire_name == "id")))
            .then_with(|| ferrule_sdk_model::js::cmp(&a.wire_name.to_lowercase(), &b.wire_name.to_lowercase()))
    });
    v
}

fn scalar_jt(s: Scalar) -> Jt {
    match s {
        Scalar::String | Scalar::Uuid | Scalar::Bytes => Jt::String,
        Scalar::Boolean => Jt::Boolean,
        Scalar::Float => Jt::Float,
        Scalar::Double => Jt::Double,
        Scalar::DateTime => Jt::OffsetDateTime,
        Scalar::Date => Jt::LocalDate,
        Scalar::Unknown | Scalar::Null | Scalar::Binary => Jt::Value,
        Scalar::Int32 => Jt::Int,
        Scalar::Int
        | Scalar::Int8
        | Scalar::Int16
        | Scalar::Int64
        | Scalar::Uint8
        | Scalar::Uint16
        | Scalar::Uint32
        | Scalar::Uint64 => Jt::Long,
    }
}

/// Whether a class named `List` is visible from the innermost of `chain` (the classes from the
/// file's top-level class down): then Kotlin's `List` must be spelled in full.
#[must_use]
pub fn list_shadowed(chain: &[&Class<'_>]) -> bool {
    chain
        .iter()
        .any(|c| c.name == "List" || c.nested.iter().any(|n| n.name == "List"))
}
