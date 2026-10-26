//! The one type mapper: the model's types as C# spells them in a property, in the raw-data
//! accessor (`GetNotNullStruct<ImmutableArray<T>>`), in `Validate`, and in tests.

use ferrule_sdk_model::model::{Literal, Scalar};

/// A C# type of a property, a response or a list item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsType {
    String,
    Long,
    Double,
    Bool,
    /// `date-time`: `System::DateTimeOffset`.
    DateTime,
    /// `date`: a `string` such as `2019-12-27`.
    Date,
    /// Any JSON value: `JsonElement`.
    Json,
    /// `IReadOnlyList<T>`, stored as `ImmutableArray<T>`.
    List(Box<CsType>),
    /// `IReadOnlyDictionary<string, T>`, stored as `FrozenDictionary<string, T>`.
    Dict(Box<CsType>),
    /// An enum: `ApiEnum<string, Status>`; its name, value type and, for a named model, its
    /// namespace below `Models` (`None`: declared in the same file).
    Enum {
        name: String,
        value: EnumValue,
        ns: Option<Vec<String>>,
    },
    /// A class (an object model or a union): its name and, for a named model, its namespace.
    Class {
        name: String,
        ns: Option<Vec<String>>,
    },
}

/// What an enum's values are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumValue {
    String,
    Bool,
    Double,
}

impl EnumValue {
    /// The `ApiEnum<TRaw, TEnum>` raw type.
    #[must_use]
    pub fn raw(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Bool => "bool",
            Self::Double => "double",
        }
    }

    /// The value type of a literal.
    #[must_use]
    pub fn of(literal: &Literal) -> Self {
        match literal {
            Literal::String(_) => Self::String,
            Literal::Boolean(_) => Self::Bool,
            Literal::Number(_) => Self::Double,
        }
    }
}

impl CsType {
    /// The scalar's C# type, `None` for the ones C# models do not map yet.
    #[must_use]
    pub fn scalar(s: Scalar) -> Self {
        match s {
            Scalar::Boolean => Self::Bool,
            s if s.is_integer() => Self::Long,
            Scalar::Float | Scalar::Double => Self::Double,
            Scalar::DateTime => Self::DateTime,
            Scalar::Date => Self::Date,
            Scalar::Unknown | Scalar::Null => Self::Json,
            _ => Self::String,
        }
    }

    /// The type as a property declares it (without `?`).
    #[must_use]
    pub fn spell(&self) -> String {
        match self {
            Self::String | Self::Date => "string".to_owned(),
            Self::Long => "long".to_owned(),
            Self::Double => "double".to_owned(),
            Self::Bool => "bool".to_owned(),
            Self::DateTime => "System::DateTimeOffset".to_owned(),
            Self::Json => "JsonElement".to_owned(),
            Self::List(t) => format!("IReadOnlyList<{}>", t.spell()),
            Self::Dict(t) => format!("IReadOnlyDictionary<string, {}>", t.spell()),
            Self::Enum { name, value, .. } => format!("ApiEnum<{}, {name}>", value.raw()),
            Self::Class { name, .. } => name.clone(),
        }
    }

    /// The type the raw data stores: `ImmutableArray<T>` for lists, `FrozenDictionary` for maps.
    #[must_use]
    pub fn storage(&self) -> String {
        match self {
            Self::List(t) => format!("ImmutableArray<{}>", t.spell()),
            Self::Dict(t) => format!("FrozenDictionary<string, {}>", t.spell()),
            _ => self.spell(),
        }
    }

    /// Whether the stored type is a value type (`GetNotNullStruct`), not a class.
    #[must_use]
    pub fn is_struct(&self) -> bool {
        matches!(
            self,
            Self::Long | Self::Double | Self::Bool | Self::DateTime | Self::Json | Self::List(_)
        )
    }

    /// Whether `Validate` calls `.Validate()` on a value of this type.
    #[must_use]
    pub fn validates(&self) -> bool {
        matches!(self, Self::Enum { .. } | Self::Class { .. })
    }

    /// Whether the type mentions `System::` (so the file needs `using System = System;`).
    #[must_use]
    pub fn uses_system(&self) -> bool {
        match self {
            Self::DateTime => true,
            Self::List(t) | Self::Dict(t) => t.uses_system(),
            _ => false,
        }
    }

    /// The namespaces (below `Models`) of the named models the type mentions.
    pub fn namespaces(&self, out: &mut Vec<Vec<String>>) {
        match self {
            Self::Enum { ns: Some(ns), .. } | Self::Class { ns: Some(ns), .. } => out.push(ns.clone()),
            Self::List(t) | Self::Dict(t) => t.namespaces(out),
            _ => {}
        }
    }

    /// The value a generated test passes for a property `wire` of this type (the spec's
    /// examples are not in the model, Forge gap G18): the wire name for a string.
    #[must_use]
    pub fn example(&self, wire: &str) -> Option<String> {
        Some(match self {
            Self::String => format!("\"{wire}\""),
            Self::Date => "\"2019-12-27\"".to_owned(),
            Self::DateTime => "System::DateTimeOffset.Parse(\"2019-12-27T18:11:19.117Z\")".to_owned(),
            Self::Long | Self::Double => "0".to_owned(),
            Self::Bool => "true".to_owned(),
            Self::List(t) => format!("[{}]", t.example(wire)?),
            Self::Json | Self::Dict(_) | Self::Enum { .. } | Self::Class { .. } => return None,
        })
    }

    /// Whether the type is or holds a list (so the file needs `System.Collections.Immutable`).
    #[must_use]
    pub fn uses_list(&self) -> bool {
        match self {
            Self::List(_) => true,
            Self::Dict(t) => t.uses_list(),
            _ => false,
        }
    }
}
