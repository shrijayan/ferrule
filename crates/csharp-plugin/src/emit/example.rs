//! The values the generated service tests pass for required properties. The model carries no
//! schema examples, so a string is its wire name, a number 0 and a boolean `true`; an object is
//! built from its required properties, a union from its first variant that can be built, and an
//! enum from its first value. Stainless fills them from the spec's examples instead (see
//! `tests/oracle.rs`, "test values").

use std::collections::BTreeSet;

use crate::emit::Gen;
use crate::emit::model::literal;
use crate::tree::{Decl, ModelFile};
use crate::types::CsType;

/// How deep nested objects are built before a value is given up.
const MAX_DEPTH: usize = 6;

/// Builds example values, collecting the namespaces the explicitly named types need.
pub struct Examples<'a> {
    g: &'a Gen<'a>,
    /// `using` directives (`Acme.Models.Widgets`) for the types named in the values.
    pub usings: BTreeSet<String>,
}

impl<'a> Examples<'a> {
    #[must_use]
    pub fn new(g: &'a Gen<'a>) -> Self {
        Self {
            g,
            usings: BTreeSet::new(),
        }
    }

    /// The declaration of `name`: in `home` when `ns` is `None` (an inline type), else in the
    /// model file of namespace `ns` that declares it.
    fn find(&self, name: &str, ns: Option<&[String]>, home: &'a ModelFile) -> Option<(&'a Decl, &'a ModelFile)> {
        let name = name.rsplit('.').next().unwrap_or(name);
        let declares = |f: &'a ModelFile| {
            f.decls
                .iter()
                .find(|d| match d {
                    Decl::Object(o) => o.name == name,
                    Decl::Union(u) => u.name == name,
                    Decl::Enum(e) => e.name == name,
                    Decl::Member(_) => false,
                })
                .map(|d| (d, f))
        };
        match ns {
            // An inline type copied from another class of the namespace is declared in its file.
            None => declares(home).or_else(|| self.g.files.files.iter().filter(|f| f.ns == home.ns).find_map(declares)),
            Some(ns) => self.g.files.files.iter().filter(|f| f.ns == ns).find_map(declares),
        }
    }

    /// A type's name as the test spells it: `Models::X` (through an alias, as names such as
    /// `Task` would clash with `System`'s) for the root namespace, else imported.
    fn named(&mut self, name: &str, file: &ModelFile) -> String {
        let models = format!("{}.Models", self.g.target.ns);
        if file.ns.is_empty() {
            self.usings.insert(format!("Models = {models}"));
            return format!("Models::{name}");
        }
        self.usings.insert(format!("{models}.{}", file.ns.join(".")));
        name.to_owned()
    }

    /// A C# expression of type `ty` for a property sent as `wire`, declared in `home`.
    /// `typed`: the expression must name its type (it converts implicitly to a union).
    pub fn value(&mut self, ty: &CsType, wire: &str, home: &'a ModelFile, typed: bool, depth: usize) -> Option<String> {
        if depth > MAX_DEPTH {
            return None;
        }
        match ty {
            CsType::List(item) if !typed => Some(format!("[{}]", self.value(item, wire, home, false, depth + 1)?)),
            CsType::Class { name, ns } | CsType::Enum { name, ns, .. } => {
                let (decl, file) = self.find(name, ns.as_deref(), home)?;
                self.decl(decl, wire, file, typed, depth)
            }
            CsType::List(_) | CsType::Json | CsType::Dict(_) => None,
            scalar => scalar.example(wire),
        }
    }

    fn decl(&mut self, decl: &'a Decl, wire: &str, file: &'a ModelFile, typed: bool, depth: usize) -> Option<String> {
        match decl {
            Decl::Object(o) => {
                let mut inits = Vec::new();
                for p in o.props.iter().filter(|p| p.required && p.constant.is_none()) {
                    inits.push(format!(
                        "{} = {}",
                        p.name,
                        self.value(&p.ty, &p.wire, file, false, depth + 1)?
                    ));
                }
                let new = if typed {
                    format!("new {}()", self.named(&o.name, file))
                } else {
                    "new()".to_owned()
                };
                Some(if inits.is_empty() {
                    new
                } else {
                    format!("{new} {{ {} }}", inits.join(", "))
                })
            }
            Decl::Union(u) => u.variants.iter().find_map(|v| match &v.ty {
                CsType::Class { .. } => self.value(&v.ty, wire, file, true, depth + 1),
                CsType::String | CsType::Long | CsType::Double | CsType::Bool => v.ty.example(wire),
                _ => None,
            }),
            // An enum converts from its raw value, but not on to a union.
            Decl::Enum(e) if !typed => e.members.first().map(|(_, v)| literal(v)),
            Decl::Enum(_) | Decl::Member(_) => None,
        }
    }
}
