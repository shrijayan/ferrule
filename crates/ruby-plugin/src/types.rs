//! The one type mapper: a field's Ruby type ([`RType`]) and its spelling in each view of the gem —
//! the converter declarations of `lib/**.rb` and their YARD tags, Sorbet's `rbi/**.rbi` and RBS's
//! `sig/**.rbs`.

/// A model class or module, by its constant path below `<Module>::Models`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassRef {
    /// `["References", "Agency"]` for `OnebusawaySDK::Models::References::Agency`.
    pub segs: Vec<String>,
    /// Whether its outermost class has an alias at the top of the module
    /// (`OnebusawaySDK::References = OnebusawaySDK::Models::References`): params and named models.
    /// Code refers to it through the alias; YARD always names the `Models` path.
    pub aliased: bool,
}

impl ClassRef {
    /// The constant as code names it: `OnebusawaySDK::References::Agency`.
    #[must_use]
    pub fn code(&self, module: &str) -> String {
        if self.aliased {
            format!("{module}::{}", self.segs.join("::"))
        } else {
            self.doc(module)
        }
    }

    /// The constant as YARD names it: `OnebusawaySDK::Models::References::Agency`.
    #[must_use]
    pub fn doc(&self, module: &str) -> String {
        format!("{module}::Models::{}", self.segs.join("::"))
    }

    /// The RBS type alias of an enum or union: `OnebusawaySDK::Models::Params::code`.
    #[must_use]
    pub fn rbs_alias(&self, module: &str) -> String {
        let mut path = vec![module.to_owned(), "Models".to_owned()];
        if let Some((last, parents)) = self.segs.split_last() {
            path.extend(parents.iter().cloned());
            path.push(crate::naming::rbs_alias(last));
        }
        path.join("::")
    }

    /// The last segment: the class name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.segs.last().map_or("", String::as_str)
    }

    /// A class nested in this one.
    #[must_use]
    pub fn child(&self, name: &str) -> Self {
        let mut segs = self.segs.clone();
        segs.push(name.to_owned());
        Self {
            segs,
            aliased: self.aliased,
        }
    }
}

/// A Ruby type of a field or parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RType {
    String,
    Integer,
    Float,
    Boolean,
    /// `date-time`.
    Time,
    Date,
    /// Any value.
    Unknown,
    /// A file upload.
    File,
    Array(Box<RType>),
    /// An object whose values have this type.
    Map(Box<RType>),
    /// A `BaseModel` subclass.
    Model(ClassRef),
    /// A module extending `Internal::Type::Enum`.
    Enum(ClassRef),
    /// A module extending `Internal::Type::Union`.
    Union(ClassRef),
    /// `inner` or nil, inside an array or map (a field's own nullability is
    /// [`crate::tree::RField::nullable`]).
    Nullable(Box<RType>),
}

/// How the Ruby views name one type; `m` is the top-level module (`OnebusawaySDK`).
impl RType {
    /// The converter of a `required`/`optional` declaration: `String`, `-> { X }`,
    /// `enum: -> { X }`, `-> { M::Internal::Type::ArrayOf[X] }`.
    #[must_use]
    pub fn rb_field(&self, m: &str) -> String {
        match self {
            Self::Model(c) => format!("-> {{ {} }}", c.code(m)),
            Self::Enum(c) => format!("enum: -> {{ {} }}", c.code(m)),
            Self::Union(c) => format!("union: -> {{ {} }}", c.code(m)),
            Self::Array(_) | Self::Map(_) if self.has_class() => format!("-> {{ {} }}", self.rb_inner(m)),
            _ => self.rb_inner(m),
        }
    }

    /// Whether a model, enum or union is inside, so the converter is wrapped in a lambda (the
    /// constant may not be defined yet when the file loads).
    #[must_use]
    pub fn has_class(&self) -> bool {
        match self {
            Self::Model(_) | Self::Enum(_) | Self::Union(_) => true,
            Self::Array(t) | Self::Map(t) | Self::Nullable(t) => t.has_class(),
            _ => false,
        }
    }

    /// The converter inside another one (`ArrayOf[...]`).
    #[must_use]
    pub fn rb_inner(&self, m: &str) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Integer => "Integer".to_owned(),
            Self::Float => "Float".to_owned(),
            Self::Boolean => format!("{m}::Internal::Type::Boolean"),
            Self::Time => "Time".to_owned(),
            Self::Date => "Date".to_owned(),
            Self::Unknown => format!("{m}::Internal::Type::Unknown"),
            Self::File => format!("{m}::Internal::Type::FileInput"),
            Self::Array(t) => format!("{m}::Internal::Type::ArrayOf[{}]", t.rb_element(m)),
            Self::Map(t) => format!("{m}::Internal::Type::HashOf[{}]", t.rb_element(m)),
            Self::Model(c) | Self::Enum(c) | Self::Union(c) => c.code(m),
            Self::Nullable(t) => t.rb_inner(m),
        }
    }

    /// The element of `ArrayOf[...]`/`HashOf[...]`: `enum: -> { X }`, `X, nil?: true`.
    fn rb_element(&self, m: &str) -> String {
        match self {
            Self::Enum(c) => format!("enum: -> {{ {} }}", c.code(m)),
            Self::Union(c) => format!("union: -> {{ {} }}", c.code(m)),
            Self::Nullable(t) => format!("{}, nil?: true", t.rb_element(m)),
            _ => self.rb_inner(m),
        }
    }

    /// YARD: `String`, `Array<X>`, `Hash{Symbol=>X}`, `Symbol, X` for an enum.
    #[must_use]
    pub fn yard(&self, m: &str) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Integer => "Integer".to_owned(),
            Self::Float => "Float".to_owned(),
            Self::Boolean => "Boolean".to_owned(),
            Self::Time => "Time".to_owned(),
            Self::Date => "Date".to_owned(),
            Self::Unknown => "Object".to_owned(),
            Self::File => "Pathname, StringIO, IO, String, #{m}::FilePart".replace("#{m}", m),
            Self::Array(t) => format!("Array<{}>", t.yard(m)),
            Self::Map(t) => format!("Hash{{Symbol=>{}}}", t.yard(m)),
            Self::Model(c) | Self::Union(c) => c.doc(m),
            Self::Enum(c) => format!("Symbol, {}", c.doc(m)),
            Self::Nullable(t) => format!("{}, nil", t.yard(m)),
        }
    }

    /// Sorbet, as read: `String`, `T::Array[X]`, `X::TaggedSymbol` for an enum.
    #[must_use]
    pub fn rbi(&self, m: &str) -> String {
        self.rbi_with(m, false, false)
    }

    /// Sorbet, as accepted: models also as hashes (`X::OrHash`), enums as `X::OrSymbol`.
    #[must_use]
    pub fn rbi_input(&self, m: &str) -> String {
        self.rbi_with(m, true, true)
    }

    /// Sorbet, as a params class holds it: enums as `X::OrSymbol`, models as themselves.
    #[must_use]
    pub fn rbi_params(&self, m: &str) -> String {
        self.rbi_with(m, false, true)
    }

    /// Sorbet with models as `OrHash` and enums as `OrSymbol` where asked.
    #[must_use]
    pub fn rbi_with(&self, m: &str, or_hash: bool, or_symbol: bool) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Integer => "Integer".to_owned(),
            Self::Float => "Float".to_owned(),
            Self::Boolean => "T::Boolean".to_owned(),
            Self::Time => "Time".to_owned(),
            Self::Date => "Date".to_owned(),
            Self::Unknown => "T.anything".to_owned(),
            Self::File => format!("{m}::Internal::FileInput"),
            Self::Array(t) => format!("T::Array[{}]", t.rbi_with(m, or_hash, or_symbol)),
            Self::Map(t) => format!("T::Hash[Symbol, {}]", t.rbi_with(m, or_hash, or_symbol)),
            Self::Model(c) => {
                if or_hash {
                    format!("{}::OrHash", c.code(m))
                } else {
                    c.code(m)
                }
            }
            Self::Enum(c) => {
                if or_symbol {
                    format!("{}::OrSymbol", c.code(m))
                } else {
                    format!("{}::TaggedSymbol", c.code(m))
                }
            }
            Self::Union(c) => format!("{}::Variants", c.code(m)),
            Self::Nullable(t) => format!("T.nilable({})", t.rbi_with(m, or_hash, or_symbol)),
        }
    }

    /// RBS: `String`, `::Array[X]`, `bool`, the alias `M::Models::X::code` of an enum.
    #[must_use]
    pub fn rbs(&self, m: &str) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Integer => "Integer".to_owned(),
            Self::Float => "Float".to_owned(),
            Self::Boolean => "bool".to_owned(),
            Self::Time => "Time".to_owned(),
            Self::Date => "Date".to_owned(),
            Self::Unknown => "top".to_owned(),
            Self::File => format!("{m}::Internal::file_input"),
            Self::Array(t) => format!("::Array[{}]", t.rbs(m)),
            Self::Map(t) => format!("::Hash[Symbol, {}]", t.rbs(m)),
            Self::Model(c) => c.code(m),
            Self::Enum(c) | Self::Union(c) => c.rbs_alias(m),
            Self::Nullable(t) => format!("{}?", t.rbs(m)),
        }
    }

    /// Whether Sorbet's reader and writer of a required field of this type differ, so the
    /// field gets `attr_reader` and `attr_writer` rather than `attr_accessor`.
    #[must_use]
    pub fn is_model(&self) -> bool {
        match self {
            Self::Model(_) => true,
            Self::Nullable(t) => t.is_model(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agency() -> ClassRef {
        ClassRef {
            segs: vec!["References".into(), "Agency".into()],
            aliased: true,
        }
    }

    #[test]
    fn spells_each_view() {
        let t = RType::Array(Box::new(RType::Model(agency())));
        assert_eq!(
            t.rb_field("M"),
            "-> { M::Internal::Type::ArrayOf[M::References::Agency] }"
        );
        assert_eq!(t.yard("M"), "Array<M::Models::References::Agency>");
        assert_eq!(t.rbi("M"), "T::Array[M::References::Agency]");
        assert_eq!(t.rbi_input("M"), "T::Array[M::References::Agency::OrHash]");
        assert_eq!(t.rbs("M"), "::Array[M::References::Agency]");
        assert_eq!(RType::Boolean.rb_field("M"), "M::Internal::Type::Boolean");
    }

    #[test]
    fn names_enum_aliases() {
        let code = ClassRef {
            segs: vec!["ReportProblemWithStopRetrieveParams".into(), "Code".into()],
            aliased: true,
        };
        assert_eq!(
            RType::Enum(code.clone()).rbs("M"),
            "M::Models::ReportProblemWithStopRetrieveParams::code"
        );
        assert_eq!(
            RType::Enum(code).rb_field("M"),
            "enum: -> { M::ReportProblemWithStopRetrieveParams::Code }"
        );
    }
}
