//! Pagination: the config's page schemes as page classes (`EntriesCursor`), and what each list
//! method returns.
//!
//! A list method returns `PagePromise<BranchesEntriesCursor, Branch>`: the page class over its
//! item type, named for the items (the item type's plural, then the class). That name is a type
//! alias in the current layout (`export type BranchesEntriesCursor = EntriesCursor<Branch>`) and
//! a subclass in the `node` layout (`export class PeersPage extends Page<Peer> {}`). It is
//! declared once, in the module of the item model (knock-typescript's `MessagesItemsCursor` in
//! `messages/messages.ts`, used by `objects.listMessages`), else in the module of the first list
//! method that returns it; other modules import it and export it again.

use ferrule_sdk_model::model::{Field, Method, Name, Object, Scalar, Type, TypeRef};
use serde_json::Value;

use crate::ctx::{Ctx, Home};
use crate::meta::PageScheme;
use crate::naming::{split_words, type_name, words_of};

/// A page scheme with its class name.
#[derive(Debug, Clone, Copy)]
pub struct Page<'s> {
    pub scheme: &'s PageScheme,
}

/// The `x-stainless-pagination-property` purpose of a config field.
fn purpose(field: &Value) -> Option<&str> {
    field
        .get("x-stainless-pagination-property")
        .and_then(|p| p.get("purpose"))
        .and_then(Value::as_str)
}

fn is_type(field: &Value, ty: &str) -> bool {
    field.get("type").and_then(Value::as_str) == Some(ty)
}

/// The field path (top-level key first) with `purpose`, searching nested `properties` and array
/// `items`.
fn path_of(fields: &[(String, Value)], wanted: &str) -> Option<Vec<String>> {
    for (k, v) in fields {
        if purpose(v) == Some(wanted) {
            return Some(vec![k.clone()]);
        }
        // Into an object's properties, or an array's items (`data.[].id`).
        let inner = [
            (v.get("properties"), None),
            (v.get("items").and_then(|i| i.get("properties")), Some("[]")),
        ];
        for (props, step) in inner {
            let Some(props) = props.and_then(Value::as_object) else {
                continue;
            };
            let nested: Vec<(String, Value)> = props.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            if let Some(mut rest) = path_of(&nested, wanted) {
                if let Some(step) = step {
                    rest.insert(0, step.to_owned());
                }
                rest.insert(0, k.clone());
                return Some(rest);
            }
        }
    }
    None
}

impl<'s> Page<'s> {
    /// The class: `EntriesCursor` for `entries_cursor`.
    #[must_use]
    pub fn class(&self, ctx: &Ctx<'_>) -> String {
        type_name(&split_words(&self.scheme.name), &ctx.client.casings)
    }

    /// The scheme's `type`.
    #[must_use]
    pub fn kind(&self) -> &'s str {
        &self.scheme.kind
    }

    /// The response field holding the items: the one with purpose `items`, else the one
    /// top-level array without a purpose (Stainless's implicit items, knock-mgmt's `entries`).
    #[must_use]
    pub fn items(&self) -> Option<&'s str> {
        let fields = &self.scheme.response;
        if let Some((k, _)) = fields.iter().find(|(_, v)| purpose(v) == Some("items")) {
            return Some(k);
        }
        let mut arrays = fields
            .iter()
            .filter(|(_, v)| is_type(v, "array") && purpose(v).is_none());
        match (arrays.next(), arrays.next()) {
            (Some((k, _)), None) => Some(k),
            _ => None,
        }
    }

    /// The response field path with `purpose`, such as `page_info.after` for
    /// `next_cursor_field`.
    #[must_use]
    pub fn response_path(&self, purpose: &str) -> Option<Vec<String>> {
        path_of(&self.scheme.response, purpose)
    }

    /// The request param with `purpose`, such as `after` for `next_cursor_param`.
    #[must_use]
    pub fn request_param(&self, wanted: &str) -> Option<&'s str> {
        self.scheme
            .request
            .iter()
            .find(|(_, v)| purpose(v) == Some(wanted))
            .map(|(k, _)| k.as_str())
    }

    /// The boolean response field that says whether there is a next page: purpose
    /// `has_next_page`, else a boolean `has_more`. Unproven: no Apache-2.0 oracle has one.
    #[must_use]
    pub fn has_more(&self) -> Option<&'s str> {
        let fields = &self.scheme.response;
        fields
            .iter()
            .find(|(_, v)| purpose(v) == Some("has_next_page"))
            .or_else(|| fields.iter().find(|(k, v)| k == "has_more" && is_type(v, "boolean")))
            .map(|(k, _)| k.as_str())
    }

    /// Whether `wire` (a query or body param) is one of the scheme's request fields, which the
    /// method's params inherit from `<Class>Params` rather than declare.
    #[must_use]
    pub fn is_request_field(&self, wire: &str) -> bool {
        self.scheme.request.iter().any(|(k, _)| k == wire)
    }
}

/// The schemes of the config, in config order.
#[must_use]
pub fn pages<'s>(ctx: &'s Ctx<'_>) -> Vec<Page<'s>> {
    ctx.settings.pagination.iter().map(|scheme| Page { scheme }).collect()
}

/// The page a list method returns.
#[must_use]
pub fn page_of<'s>(ctx: &'s Ctx<'_>, m: &Method) -> Option<Page<'s>> {
    let p = m.pagination.as_ref()?;
    ctx.settings
        .pagination
        .iter()
        .find(|s| s.name == p.page.written)
        .map(|scheme| Page { scheme })
}

/// A config field's schema as a model type, for the page classes: the config declares only
/// JSON-schema keywords (`type`, `items`, `properties`, `required`, `description`, `nullable`).
#[must_use]
pub fn schema_type(v: &Value) -> Type {
    let ty = match v.get("type").and_then(Value::as_str) {
        Some("string") => Type::scalar(Scalar::String),
        Some("integer") => Type::scalar(Scalar::Int),
        Some("number") => Type::scalar(Scalar::Double),
        Some("boolean") => Type::scalar(Scalar::Boolean),
        Some("array") => Type::Array {
            items: Box::new(v.get("items").map_or(Type::scalar(Scalar::Unknown), schema_type)),
        },
        _ => match v.get("properties").and_then(Value::as_object) {
            Some(props) if !props.is_empty() => {
                let required: Vec<&str> = v
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|r| r.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                Type::Object(Object {
                    fields: props
                        .iter()
                        .map(|(k, p)| field(k, p, required.contains(&k.as_str())))
                        .collect(),
                    ..Object::default()
                })
            }
            _ => Type::scalar(Scalar::Unknown),
        },
    };
    if v.get("nullable").and_then(Value::as_bool) == Some(true) {
        Type::Nullable { inner: Box::new(ty) }
    } else {
        ty
    }
}

/// A config field as a model field.
#[must_use]
pub fn field(key: &str, v: &Value, required: bool) -> Field {
    Field {
        name: Name {
            written: key.to_owned(),
            words: split_words(key),
        },
        wire_name: key.to_owned(),
        required,
        deprecated: false,
        read_only: false,
        description: v
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        default: None,
        ty: schema_type(v),
    }
}

/// The value a page class falls back to when the response lacks a field: `body.x || ''`.
#[must_use]
pub fn default_value(v: &Value) -> Option<&'static str> {
    match v.get("type").and_then(Value::as_str) {
        Some("string") => Some("''"),
        Some("integer" | "number") => Some("0"),
        Some("boolean") => Some("false"),
        Some("array") => Some("[]"),
        Some("object") => Some("{}"),
        _ if v.get("properties").is_some() => Some("{}"),
        _ => None,
    }
}

/// `Branch` -> `Branches`, `Activity` -> `Activities`: the plural of a type name's last word.
#[must_use]
pub fn plural_type(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let vowel_before_y = lower.len() > 1 && lower[..lower.len() - 1].ends_with(['a', 'e', 'i', 'o', 'u']);
    if lower.ends_with('y') && !vowel_before_y && lower.len() > 1 {
        format!("{}ies", &name[..name.len() - 1])
    } else if ["s", "x", "ch", "sh"].iter().any(|s| lower.ends_with(s)) {
        format!("{name}es")
    } else {
        format!("{name}s")
    }
}

/// What a page's items are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item<'a> {
    /// A named model.
    Model(&'a TypeRef),
    /// An inline object or other type, declared with the method as `<Prefix><Method>Response`.
    Inline(String),
}

/// A list's page type, declared once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageAlias<'a> {
    /// `BranchesEntriesCursor`.
    pub name: String,
    /// The page class: `EntriesCursor`.
    pub class: String,
    /// The module that declares it (from `src/`, no `.ts`).
    pub module: String,
    pub item: Item<'a>,
}

/// The type name of a method's inline items: `GadgetListResponse`.
#[must_use]
pub fn inline_item_name(ctx: &Ctx<'_>, prefix: &str, m: &Method) -> String {
    format!("{prefix}{}Response", type_name(&words_of(&m.name), &ctx.client.casings))
}

/// The page type a list method returns, and where it is declared.
#[must_use]
pub fn alias_of<'a>(ctx: &Ctx<'a>, r: &crate::ctx::TsResource<'a>, m: &'a Method) -> Option<PageAlias<'a>> {
    let page = page_of(ctx, m)?;
    let class = page.class(ctx);
    let item_ty = &m.pagination.as_ref()?.item;
    let (item, item_name, module) = if let Type::Ref(t) = item_ty {
        let module = match ctx.home(t) {
            Home::Resource(_) => ctx.model_module(t),
            Home::Shared => r.module(),
        };
        (Item::Model(t), ctx.type_of(t), module)
    } else {
        let name = inline_item_name(ctx, &r.singular(&ctx.client.casings), m);
        (Item::Inline(name.clone()), name, r.module())
    };
    Some(PageAlias {
        name: format!("{}{class}", plural_type(&item_name)),
        class,
        module,
        item,
    })
}

/// Every page type, each once, in the order the resources' list methods first return them.
#[must_use]
pub fn aliases<'a>(ctx: &Ctx<'a>) -> Vec<PageAlias<'a>> {
    let mut out: Vec<PageAlias<'a>> = Vec::new();
    for r in ctx.all() {
        for m in crate::emit::resource::sorted_methods(r) {
            if let Some(a) = alias_of(ctx, r, m)
                && !out.iter().any(|o| o.name == a.name)
            {
                out.push(a);
            }
        }
    }
    out
}

/// The page types `module` declares, in order.
#[must_use]
pub fn declared_in<'a>(ctx: &Ctx<'a>, module: &str) -> Vec<PageAlias<'a>> {
    aliases(ctx).into_iter().filter(|a| a.module == module).collect()
}

#[cfg(test)]
mod tests {
    use super::plural_type;

    #[test]
    fn pluralizes_type_names() {
        assert_eq!(plural_type("Branch"), "Branches");
        assert_eq!(plural_type("Activity"), "Activities");
        assert_eq!(plural_type("Object"), "Objects");
        assert_eq!(plural_type("FeedListItemsResponse"), "FeedListItemsResponses");
        assert_eq!(plural_type("Key"), "Keys");
    }
}
