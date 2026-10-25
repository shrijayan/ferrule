//! The one function mapping [`SdkType`] to a Python annotation (CLAUDE.md: type mapping lives
//! here).

use crate::emit::common::py_string;
use crate::emit::format::{Expr, bracket};
use crate::js;
use crate::model::{SdkLiteral, SdkObject, SdkPrimitive, SdkRef, SdkType};
use crate::printer::PythonPrinter;

/// How the file being printed spells inline objects and named models.
pub trait AnnotationContext {
    /// The class name the file gives an inline object.
    fn object_name(&self, obj: &SdkObject) -> String;
    /// The name a named model has in this file; imports it if needed.
    fn ref_name(&self, p: &mut PythonPrinter, r: &SdkRef) -> String;
    /// Imports `name` from the package module `module` (such as `_types`).
    fn runtime(&self, p: &mut PythonPrinter, module: &str, name: &str) {
        let _ = (p, module, name);
    }
    /// Request types (method arguments, `TypedDict` params) rather than response models: arrays are
    /// `Iterable` (`SequenceNotStr` for strings), dates also accept `str`.
    fn input(&self) -> bool {
        false
    }
    /// Whether one of the file's fields uses `name`, which a `from datetime import` would clash with.
    fn shadowed(&self, name: &str) -> bool {
        let _ = name;
        false
    }
    /// The alias the file declares for a union, if any.
    fn union_name(&self, union: &SdkType) -> Option<String> {
        let _ = union;
        None
    }
}

/// `{ ...ctx, unionName: () => undefined }`: the context without union aliases, for the
/// variants of the alias itself.
pub struct WithoutUnionNames<'a>(pub &'a dyn AnnotationContext);

impl AnnotationContext for WithoutUnionNames<'_> {
    fn object_name(&self, obj: &SdkObject) -> String {
        self.0.object_name(obj)
    }
    fn ref_name(&self, p: &mut PythonPrinter, r: &SdkRef) -> String {
        self.0.ref_name(p, r)
    }
    fn runtime(&self, p: &mut PythonPrinter, module: &str, name: &str) {
        self.0.runtime(p, module, name);
    }
    fn input(&self) -> bool {
        self.0.input()
    }
    fn shadowed(&self, name: &str) -> bool {
        self.0.shadowed(name)
    }
}

/// A Python literal of an enum value.
#[must_use]
pub fn py_literal(v: &SdkLiteral) -> String {
    match v {
        SdkLiteral::String(s) => py_string(s),
        SdkLiteral::Boolean(b) => if *b { "True" } else { "False" }.to_owned(),
        SdkLiteral::Number(n) => js::number_to_string(*n),
    }
}

fn is_null(t: &SdkType) -> bool {
    matches!(t, SdkType::Primitive(SdkPrimitive::Null))
}

/// Map an [`SdkType`] to a Python annotation, registering needed imports on `p`.
pub fn to_annotation(ty: &SdkType, p: &mut PythonPrinter, ctx: &dyn AnnotationContext) -> String {
    match ty {
        SdkType::Primitive(prim @ (SdkPrimitive::Datetime | SdkPrimitive::Date)) => {
            let base = prim.as_str();
            let name = if ctx.shadowed(base) {
                // A field of the same name would shadow the class: `import datetime`, `datetime.date`.
                p.import_module("datetime");
                format!("datetime.{base}")
            } else {
                p.import_from("datetime", base);
                base.to_owned()
            };
            if !ctx.input() {
                return name;
            }
            p.import_from("typing", "Union");
            format!("Union[str, {name}]")
        }
        SdkType::Primitive(prim) => match prim {
            SdkPrimitive::String => "str",
            SdkPrimitive::Integer => "int",
            SdkPrimitive::Number => "float",
            SdkPrimitive::Boolean => "bool",
            SdkPrimitive::Null | SdkPrimitive::Datetime | SdkPrimitive::Date => "None",
        }
        .to_owned(),
        SdkType::Unknown => "object".to_owned(),
        SdkType::Array(items) => {
            if !ctx.input() {
                p.import_from("typing", "List");
                return format!("List[{}]", to_annotation(items, p, ctx));
            }
            if matches!(**items, SdkType::Primitive(SdkPrimitive::String)) {
                ctx.runtime(p, "_types", "SequenceNotStr");
                return "SequenceNotStr[str]".to_owned();
            }
            p.import_from("typing", "Iterable");
            format!("Iterable[{}]", to_annotation(items, p, ctx))
        }
        SdkType::Map(values) => {
            p.import_from("typing", "Dict");
            format!("Dict[str, {}]", to_annotation(values, p, ctx))
        }
        SdkType::Union { variants, .. } => {
            if let Some(alias) = ctx.union_name(ty) {
                return alias;
            }
            let (rest, nullable) = union_members(variants);
            if rest.len() > 1 {
                p.import_from("typing", "Union");
                let mut parts: Vec<String> = rest.iter().map(|t| to_annotation(t, p, ctx)).collect();
                if nullable {
                    parts.push("None".to_owned());
                }
                return format!("Union[{}]", parts.join(", "));
            }
            let inner = rest
                .first()
                .map_or_else(|| "None".to_owned(), |t| to_annotation(t, p, ctx));
            if !nullable {
                return inner;
            }
            p.import_from("typing", "Optional");
            format!("Optional[{inner}]")
        }
        SdkType::Literal(values) => {
            p.import_from("typing_extensions", "Literal");
            format!(
                "Literal[{}]",
                values.iter().map(py_literal).collect::<Vec<_>>().join(", ")
            )
        }
        SdkType::Ref(r) => ctx.ref_name(p, r),
        SdkType::Object(obj) => ctx.object_name(obj),
    }
}

/// The members of a union as Python lists them, without `null`, and whether `null` is one.
/// Forge does not keep member order (gap G17); the oracles list enums before plain types.
#[must_use]
pub fn union_members(variants: &[SdkType]) -> (Vec<&SdkType>, bool) {
    let mut rest: Vec<&SdkType> = variants.iter().filter(|t| !is_null(t)).collect();
    rest.sort_by_key(|t| u8::from(!matches!(t, SdkType::Literal(_))));
    let nullable = rest.len() != variants.len();
    (rest, nullable)
}

/// The annotation of a union of named models or objects whose alias is being declared, as
/// bracketed members: `Union[A, B]`, or `Annotated[Union[A, B], PropertyInfo(discriminator=...)]`
/// when the spec names a discriminator and `discriminated` is set (responses, not params).
pub fn union_alias_expr(ty: &SdkType, p: &mut PythonPrinter, ctx: &dyn AnnotationContext, discriminated: bool) -> Expr {
    let SdkType::Union {
        variants,
        discriminator,
    } = ty
    else {
        return to_annotation(ty, p, ctx).into();
    };
    p.import_from("typing", "Union");
    let plain = WithoutUnionNames(ctx);
    let (rest, nullable) = union_members(variants);
    let mut members: Vec<Expr> = rest.iter().map(|v| to_annotation(v, p, &plain).into()).collect();
    if nullable {
        members.push("None".into());
    }
    let union = bracket("Union[", members, false, "]");
    let Some(d) = discriminator.as_ref().filter(|_| discriminated) else {
        return union.into();
    };
    p.import_from("typing_extensions", "Annotated");
    ctx.runtime(p, "_utils", "PropertyInfo");
    bracket(
        "Annotated[",
        vec![
            union.into(),
            format!("PropertyInfo(discriminator={})", py_string(d)).into(),
        ],
        false,
        "]",
    )
    .into()
}

/// The annotation of an alias's type as bracketed parts, so a long one splits as ruff splits it.
pub fn alias_expr(ty: &SdkType, p: &mut PythonPrinter, ctx: &dyn AnnotationContext, discriminated: bool) -> Expr {
    match ty {
        SdkType::Union { .. } => union_alias_expr(ty, p, ctx, discriminated),
        SdkType::Literal(values) => {
            p.import_from("typing_extensions", "Literal");
            bracket(
                "Literal[",
                values.iter().map(|v| py_literal(v).into()).collect(),
                false,
                "]",
            )
            .into()
        }
        other => to_annotation(other, p, ctx).into(),
    }
}

/// True for `T | null`, which a field types as `Optional[T]` already.
#[must_use]
pub fn is_nullable(ty: &SdkType) -> bool {
    matches!(ty, SdkType::Union { variants, .. } if variants.iter().any(is_null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SdkField, SdkObject};

    struct Plain {
        input: bool,
    }

    impl AnnotationContext for Plain {
        fn object_name(&self, _: &SdkObject) -> String {
            if self.input { "O" } else { "Obj" }.to_owned()
        }
        fn ref_name(&self, _: &mut PythonPrinter, _: &SdkRef) -> String {
            if self.input { "R" } else { "Ref" }.to_owned()
        }
        fn input(&self) -> bool {
            self.input
        }
    }

    fn prim(p: SdkPrimitive) -> SdkType {
        SdkType::Primitive(p)
    }

    #[test]
    fn maps_every_sdk_type_kind_and_collects_imports() {
        let obj = SdkType::Object(SdkObject {
            fields: vec![SdkField {
                wire_name: "x".into(),
                py_name: "x".into(),
                required: true,
                description: String::new(),
                ty: prim(SdkPrimitive::String),
            }],
            ..SdkObject::default()
        });
        let t = SdkType::Union {
            variants: vec![
                prim(SdkPrimitive::String),
                prim(SdkPrimitive::Integer),
                prim(SdkPrimitive::Number),
                prim(SdkPrimitive::Boolean),
                prim(SdkPrimitive::Datetime),
                prim(SdkPrimitive::Date),
                SdkType::Unknown,
                SdkType::Array(Box::new(SdkType::Ref(SdkRef {
                    name: "User".into(),
                    module: "user".into(),
                }))),
                SdkType::Map(Box::new(SdkType::Union {
                    variants: vec![prim(SdkPrimitive::String), prim(SdkPrimitive::Null)],
                    discriminator: None,
                })),
                SdkType::Literal(vec![
                    SdkLiteral::String("a".into()),
                    SdkLiteral::Number(1.0),
                    SdkLiteral::Boolean(true),
                    SdkLiteral::Boolean(false),
                ]),
                obj,
                prim(SdkPrimitive::Null),
            ],
            discriminator: None,
        };
        let mut p = PythonPrinter::default();
        assert_eq!(
            to_annotation(&t, &mut p, &Plain { input: false }),
            "Union[Literal[\"a\", 1, True, False], str, int, float, bool, datetime, date, object, List[Ref], Dict[str, Optional[str]], Obj, None]"
        );
        assert_eq!(
            p.render(),
            [
                "from typing import Dict, List, Union, Optional",
                "from datetime import date, datetime",
                "from typing_extensions import Literal",
                "",
            ]
            .join("\n")
        );
    }

    #[test]
    fn maps_input_arrays_and_dates() {
        let mut p = PythonPrinter::default();
        let ctx = Plain { input: true };
        let ints = SdkType::Array(Box::new(prim(SdkPrimitive::Integer)));
        assert_eq!(to_annotation(&ints, &mut p, &ctx), "Iterable[int]");
        assert_eq!(
            to_annotation(&prim(SdkPrimitive::Datetime), &mut p, &ctx),
            "Union[str, datetime]"
        );
    }
}
