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
    /// The required ones and every body property the request's own example gives (an
    /// `@example`: lumaai-typescript's `generations.create`).
    Example,
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
    /// Whether a value comes from an example in the spec rather than from its type.
    pub from_spec: bool,
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
            return if self.ctx.node() {
                self.typed(ty, &ex)
            } else {
                json_expr(&ex)
            };
        }
        let format = schema
            .map(|s| extras.resolve(s))
            .and_then(|s| s.get("format"))
            .and_then(Value::as_str);
        match ty {
            Type::Scalar { scalar } => match scalar {
                Scalar::String if matches!(format, Some("uri" | "url")) => Expr::str("https://example.com"),
                Scalar::String if format == Some("email") => Expr::str("dev@stainless.com"),
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
                // A number is its `minimum`, else its `maximum`, else 0 (as for Python).
                _ => {
                    let bound = schema
                        .map(|s| extras.resolve(s))
                        .and_then(|s| s.get("minimum").or_else(|| s.get("maximum")))
                        .and_then(Value::as_f64);
                    Expr::Num(bound.map_or_else(|| "0".to_owned(), js::number_to_string))
                }
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
            // An open enum (`'5s' | '9s' | (string & {})`) is its first member.
            Type::Union(u) => u
                .variants
                .iter()
                .find(|v| self.ctx.node() && matches!(v.ty, Type::Enum { .. }))
                .or_else(|| u.variants.first())
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

    /// A spec example as a value of `ty`: an object's keys in the order its type lists them (the
    /// `node` layout's tests: `{ id: '...', type: 'generation' }` from `{type, id}`), others as
    /// written.
    fn typed(&self, ty: &Type, v: &Value) -> Expr {
        match (ty, v) {
            (Type::Nullable { inner }, _) => self.typed(inner, v),
            (Type::Ref(r), _) => self
                .ctx
                .client
                .model(r)
                .map_or_else(|| json_expr(v), |d| self.typed(&d.ty, v)),
            (Type::Array { items }, Value::Array(a)) => Expr::Array(a.iter().map(|x| self.typed(items, x)).collect()),
            // The variant whose properties include every key of the value.
            (Type::Union(u), Value::Object(map)) => u
                .variants
                .iter()
                .find(|v| {
                    self.ctx
                        .client
                        .object_of(&v.ty)
                        .is_some_and(|o| map.keys().all(|k| o.fields.iter().any(|f| &f.wire_name == k)))
                })
                .map_or_else(|| json_expr(v), |variant| self.typed(&variant.ty, v)),
            (_, Value::Object(map)) if self.ctx.client.object_of(ty).is_some() => {
                let Some(o) = self.ctx.client.object_of(ty) else {
                    return json_expr(v);
                };
                let mapper = crate::types::Mapper::new(self.ctx, "");
                let mut props: Vec<(String, Expr)> = mapper
                    .ordered(&o.fields)
                    .into_iter()
                    .filter_map(|f| {
                        map.get(&f.wire_name)
                            .map(|x| (js::property_key(&f.wire_name), self.typed(&f.ty, x)))
                    })
                    .collect();
                for (k, x) in map {
                    if !o.fields.iter().any(|f| &f.wire_name == k) {
                        props.push((js::property_key(k), json_expr(x)));
                    }
                }
                Expr::object(props)
            }
            _ => json_expr(v),
        }
    }

    /// The arguments of a call to `m`. The `node` layout uses a path parameter's example but not a
    /// query parameter's (onebusaway-typescript: `tripId: 'tripId'` beside a spec example).
    #[must_use]
    pub fn args(&self, m: &Method, fill: Fill) -> Args {
        let extras = self.ctx.extras;
        let mut from_spec = false;
        let positional = crate::emit::resource::positional(self.ctx, m)
            .iter()
            .map(|p| {
                let schema = extras.parameter(&m.pointer, "path", &p.field.wire_name);
                let ex = schema.and_then(|s| extras.parameter_example(s));
                from_spec |= ex.is_some();
                ex.map_or_else(
                    || self.value(&p.field.ty, None, &p.field.wire_name, fill),
                    |v| json_expr(&v),
                )
            })
            .collect();
        let mut props = Vec::new();
        for p in crate::emit::resource::path_in_params(self.ctx, m) {
            let schema = extras.parameter(&m.pointer, "path", &p.field.wire_name);
            let ex = schema.and_then(|s| extras.parameter_example(s));
            from_spec |= ex.is_some();
            let value = ex.map_or_else(
                || self.value(&p.field.ty, None, &p.field.wire_name, fill),
                |v| json_expr(&v),
            );
            props.push((js::property_key(&p.field.wire_name), value));
        }
        for p in crate::emit::resource::query_params(self.ctx, m) {
            if fill != Fill::All && !p.field.required {
                continue;
            }
            let schema = extras.parameter(&m.pointer, "query", &p.field.wire_name);
            let ex = schema
                .filter(|_| !self.ctx.node())
                .and_then(|s| extras.parameter_example(s));
            from_spec |= ex.is_some();
            let value = ex.map_or_else(
                || {
                    let s = schema.and_then(|s| s.get("schema"));
                    self.value(&p.field.ty, s, &p.field.wire_name, fill)
                },
                |v| json_expr(&v),
            );
            props.push((js::property_key(&p.field.wire_name), value));
        }
        if let Some(body) = &m.body
            && let Some(o) = self.ctx.client.object_of(&body.ty)
        {
            let body_schema = extras.body_schema(&m.pointer);
            // The request schema's own example wins over its properties' examples.
            let body_example = extras.body_example(&m.pointer);
            let mapper = crate::types::Mapper::new(self.ctx, "");
            for f in mapper.ordered(&o.fields) {
                let given = body_example.as_ref().and_then(|e| e.get(&f.wire_name));
                let wanted = match fill {
                    Fill::Required => f.required,
                    Fill::Example => f.required || given.is_some(),
                    Fill::All => true,
                };
                if !wanted {
                    continue;
                }
                let s = body_schema.and_then(|b| extras.property(b, &f.wire_name));
                from_spec |= given.is_some() || s.and_then(|s| extras.example(s)).is_some();
                // The `node` layout's tests key the request's example by its type; its
                // `@example` writes it as the spec does.
                let value = given.map_or_else(
                    || self.value(&f.ty, s, &f.wire_name, fill),
                    |g| {
                        if self.ctx.node() && fill != Fill::Example {
                            self.typed(&f.ty, g)
                        } else {
                            json_expr(g)
                        }
                    },
                );
                props.push((js::property_key(&f.wire_name), value));
            }
        }
        Args {
            positional,
            props,
            from_spec,
        }
    }
}
