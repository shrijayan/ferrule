//! Example values for a method's parameters, as Stainless writes them in `@example` blocks,
//! tests and the README: the spec's example when it has one, else a value derived from the type
//! (the wire name for strings, `0`, `true`, the first enum member, a fixed date).

use ferrule_sdk_model::model::{Literal, Method, Scalar, Type};
use serde_json::Value;

use crate::ctx::Ctx;
use crate::js;
use crate::pretty::Expr;

/// Which parameters an example fills in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    Required,
    All,
}

/// A JSON value as a JavaScript expression.
#[must_use]
pub fn json_expr(v: &Value) -> Expr {
    match v {
        Value::Null => Expr::Null,
        Value::Bool(b) => Expr::Bool(*b),
        Value::Number(n) => Expr::Num(js::json_number(n)),
        Value::String(s) => Expr::Str(s.clone()),
        Value::Array(items) => Expr::Array(items.iter().map(json_expr).collect()),
        Value::Object(map) => Expr::object(map.iter().map(|(k, v)| (js::property_key(k), json_expr(v))).collect()),
    }
}

/// One positional argument or object property of a call.
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    /// Path parameters, in path order.
    pub positional: Vec<Expr>,
    /// The params object's properties, empty when there are none.
    pub props: Vec<(String, Expr)>,
}

impl Args {
    /// The call's arguments: path parameters, then the params object when it has properties.
    #[must_use]
    pub fn exprs(&self) -> Vec<Expr> {
        let mut out = self.positional.clone();
        if !self.props.is_empty() {
            out.push(Expr::object(self.props.clone()));
        }
        out
    }
}

pub struct Examples<'c, 'a> {
    pub ctx: &'c Ctx<'a>,
}

impl Examples<'_, '_> {
    /// The value of a parameter or property of type `ty`, `schema` its spec schema.
    #[must_use]
    pub fn value(&self, ty: &Type, schema: Option<&Value>, wire: &str, fill: Fill) -> Expr {
        let extras = self.ctx.extras;
        if let Some(ex) = schema.and_then(|s| extras.example(s)) {
            return json_expr(&ex);
        }
        match ty {
            Type::Scalar { scalar } => match scalar {
                Scalar::String | Scalar::Bytes => Expr::str(wire),
                Scalar::Uuid => Expr::str("182bd5e5-6e1a-4fe4-a799-aa6d9a6ab26e"),
                Scalar::DateTime => Expr::str("2019-12-27T18:11:19.117Z"),
                Scalar::Date => Expr::str("2019-12-27"),
                Scalar::Boolean => Expr::Bool(true),
                Scalar::Null => Expr::Null,
                Scalar::Unknown | Scalar::Binary => Expr::Object(crate::pretty::Object {
                    props: Vec::new(),
                    expanded: false,
                }),
                _ => Expr::Num("0".to_owned()),
            },
            Type::Enum { members } => match members.first().map(|m| &m.value) {
                Some(Literal::String(s)) => Expr::Str(s.clone()),
                Some(Literal::Number(n)) => Expr::Num(js::number_to_string(*n)),
                Some(Literal::Boolean(b)) => Expr::Bool(*b),
                None => Expr::Null,
            },
            Type::Array { items } => {
                let item_schema = schema.and_then(|s| extras.items(s));
                Expr::Array(vec![self.value(items, item_schema, "string", fill)])
            }
            Type::Map { values } => Expr::object(vec![("foo".to_owned(), self.value(values, None, "bar", fill))]),
            Type::Nullable { inner } => self.value(inner, schema, wire, fill),
            Type::Union(u) => u
                .variants
                .first()
                .map_or(Expr::Null, |v| self.value(&v.ty, schema, wire, fill)),
            Type::Ref(r) => match self.ctx.client.model(r) {
                Some(decl) => self.value(&decl.ty, schema, wire, fill),
                None => Expr::Null,
            },
            Type::Object(o) => {
                let mapper = crate::types::Mapper::new(self.ctx, "");
                let props = mapper
                    .ordered(&o.fields)
                    .into_iter()
                    .filter(|f| fill == Fill::All || f.required)
                    .map(|f| {
                        let s = schema.and_then(|s| extras.property(s, &f.wire_name));
                        (js::property_key(&f.wire_name), self.value(&f.ty, s, &f.wire_name, fill))
                    })
                    .collect();
                Expr::object(props)
            }
        }
    }

    /// The arguments of a call to `m`.
    #[must_use]
    pub fn args(&self, m: &Method, fill: Fill) -> Args {
        let extras = self.ctx.extras;
        let positional = m
            .path_params
            .iter()
            .map(|p| {
                let schema = extras.parameter(&m.pointer, "path", &p.field.wire_name);
                let ex = schema.and_then(|s| extras.parameter_example(s));
                ex.map_or_else(
                    || self.value(&p.field.ty, None, &p.field.wire_name, fill),
                    |v| json_expr(&v),
                )
            })
            .collect();
        let mut props = Vec::new();
        for p in &m.query_params {
            if fill == Fill::Required && !p.field.required {
                continue;
            }
            let schema = extras.parameter(&m.pointer, "query", &p.field.wire_name);
            let ex = schema.and_then(|s| extras.parameter_example(s));
            let value = ex.map_or_else(
                || self.value(&p.field.ty, None, &p.field.wire_name, fill),
                |v| json_expr(&v),
            );
            props.push((js::property_key(&p.field.wire_name), value));
        }
        if let Some(body) = &m.body
            && let Some(o) = self.ctx.client.object_of(&body.ty)
        {
            let body_schema = extras.body_schema(&m.pointer);
            // The request schema's own example wins over its properties' examples.
            let body_example = body_schema.and_then(|b| extras.example(b));
            let mapper = crate::types::Mapper::new(self.ctx, "");
            for f in mapper.ordered(&o.fields) {
                if fill == Fill::Required && !f.required {
                    continue;
                }
                let value = body_example.as_ref().and_then(|e| e.get(&f.wire_name)).map_or_else(
                    || {
                        let s = body_schema.and_then(|b| extras.property(b, &f.wire_name));
                        self.value(&f.ty, s, &f.wire_name, fill)
                    },
                    json_expr,
                );
                props.push((js::property_key(&f.wire_name), value));
            }
        }
        Args { positional, props }
    }
}
