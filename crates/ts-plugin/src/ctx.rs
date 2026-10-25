//! The resource tree as the TypeScript SDK lays it out: class and property names, the file each
//! resource and model lives in, and how one file refers to another's types.

use std::collections::HashMap;

use ferrule_sdk_model::Client;
use ferrule_sdk_model::model::{Method, ModelDecl, Resource, Type, TypeRef};

use crate::extras::SpecExtras;
use crate::meta::{Layout, TsSettings};
use crate::naming::{camel_case, camel_case_plain, kebab_case, singular, type_name, words_of};

/// One resource, named.
#[derive(Debug, Clone)]
pub struct TsResource<'a> {
    pub res: &'a Resource,
    /// stainless.yml keys from the client down.
    pub keys: Vec<String>,
    /// `kebab-case` of each key.
    pub files: Vec<String>,
    /// The class, such as `Embeddings`, `QA`, `v1`.
    pub class: String,
    /// The property on the parent, such as `embeddings`, `urlEndpoints`.
    pub prop: String,
    /// Whether it gets a directory of its own (it has subresources, or is a standalone API).
    pub package: bool,
    pub children: Vec<TsResource<'a>>,
    /// The description of its first method's first tag.
    pub description: Option<String>,
}

impl TsResource<'_> {
    /// `resources/classifications/universal` for a package, else the path of its file.
    #[must_use]
    pub fn dir(&self) -> String {
        format!("resources/{}", self.files.join("/"))
    }

    /// Its module from `src/`, without `.ts`: `resources/embeddings`,
    /// `resources/classifications/universal/universal`.
    #[must_use]
    pub fn module(&self) -> String {
        if self.package {
            format!("{}/{}", self.dir(), self.files.last().map_or("", String::as_str))
        } else {
            self.dir()
        }
    }

    /// The directory its module is in, from `src/`.
    #[must_use]
    pub fn module_dir(&self) -> String {
        let m = self.module();
        m.rsplit_once('/').map_or_else(String::new, |(d, _)| d.to_owned())
    }

    /// `client.classifications.universal`.
    #[must_use]
    pub fn accessor(&self, chain: &[&TsResource<'_>]) -> String {
        let mut out = "client".to_owned();
        for r in chain {
            out.push('.');
            out.push_str(&r.prop);
        }
        out
    }

    /// The type-name prefix of its methods' params and responses: the singular class name,
    /// `Embedding` for `Embeddings`.
    #[must_use]
    pub fn singular(&self, casings: &[ferrule_sdk_model::model::CustomCasing]) -> String {
        let words = words_of(&self.res.name);
        type_name(&singular(&words), casings)
    }
}

/// Where a named model is declared.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Home {
    /// `$shared`: `src/resources/shared.ts`.
    Shared,
    /// A resource, by its keys.
    Resource(Vec<String>),
}

/// Everything the emitters share.
pub struct Ctx<'a> {
    pub client: &'a Client,
    pub settings: TsSettings,
    pub extras: &'a SpecExtras,
    pub resources: Vec<TsResource<'a>>,
    homes: HashMap<String, Home>,
}

/// A property, method or parameter name in `layout`'s spelling: `stopIDs` in the current layout,
/// `stopIds` in the `node` layout.
#[must_use]
pub fn member_name<S: AsRef<str>>(
    layout: Layout,
    words: &[S],
    casings: &[ferrule_sdk_model::model::CustomCasing],
) -> String {
    match layout {
        Layout::Typescript => camel_case(words, casings),
        Layout::Node => camel_case_plain(words, casings),
    }
}

fn build<'a>(
    client: &'a Client,
    layout: Layout,
    extras: &SpecExtras,
    res: &'a Resource,
    parent: &[String],
) -> TsResource<'a> {
    let mut keys = parent.to_vec();
    keys.push(res.name.written.clone());
    let files = keys
        .iter()
        .map(|k| kebab_case(&crate::naming::split_words(k)))
        .collect();
    let words = words_of(&res.name);
    let mut methods: Vec<&Method> = res.methods.iter().collect();
    methods.sort_by_key(|m| m.order);
    let description = methods.first().and_then(|m| extras.tag_description(&m.pointer));
    let mut subs: Vec<&Resource> = res.subresources.iter().collect();
    subs.sort_by_key(|r| r.order);
    TsResource {
        res,
        class: type_name(&words, &client.casings),
        prop: member_name(layout, &words, &client.casings),
        package: !res.subresources.is_empty() || res.standalone,
        children: subs
            .into_iter()
            .map(|s| build(client, layout, extras, s, &keys))
            .collect(),
        keys,
        files,
        description,
    }
}

/// Every type reference inside `ty`.
fn refs<'t>(ty: &'t Type, out: &mut Vec<&'t TypeRef>) {
    match ty {
        Type::Scalar { .. } | Type::Enum { .. } => {}
        Type::Array { items } => refs(items, out),
        Type::Map { values } => refs(values, out),
        Type::Nullable { inner } => refs(inner, out),
        Type::Ref(r) => out.push(r),
        Type::Union(u) => {
            for v in &u.variants {
                refs(&v.ty, out);
            }
        }
        Type::Object(o) => {
            out.extend(o.bases.iter());
            for f in &o.fields {
                refs(&f.ty, out);
            }
            if let Some(e) = &o.extra {
                refs(e, out);
            }
        }
    }
}

fn method_refs(m: &Method) -> Vec<&TypeRef> {
    let mut out = Vec::new();
    if let Some(r) = &m.response {
        refs(&r.ty, &mut out);
    }
    if let Some(b) = &m.body {
        refs(&b.ty, &mut out);
    }
    for p in m.path_params.iter().chain(&m.query_params).chain(&m.header_params) {
        refs(&p.field.ty, &mut out);
    }
    out
}

fn walk<'r, 'a>(rs: &'r [TsResource<'a>], out: &mut Vec<&'r TsResource<'a>>) {
    for r in rs {
        out.push(r);
        walk(&r.children, out);
    }
}

/// `TypeRef.path` without its last segment (the `models` key), as resource keys.
fn ref_dirs(r: &TypeRef) -> Vec<String> {
    r.path[..r.path.len().saturating_sub(1)]
        .iter()
        .map(|n| n.written.clone())
        .collect()
}

impl<'a> Ctx<'a> {
    #[must_use]
    pub fn new(client: &'a Client, settings: TsSettings, extras: &'a SpecExtras) -> Self {
        let mut tops: Vec<&Resource> = client.resources.iter().collect();
        tops.sort_by_key(|r| r.order);
        let resources = tops
            .into_iter()
            .map(|r| build(client, settings.layout, extras, r, &[]))
            .collect();
        let mut ctx = Self {
            client,
            settings,
            extras,
            resources,
            homes: HashMap::new(),
        };
        ctx.homes = ctx.find_homes();
        ctx
    }

    /// Whether the SDK has the legacy `node` layout.
    #[must_use]
    pub fn node(&self) -> bool {
        self.settings.layout == Layout::Node
    }

    /// A property, method or parameter name in the layout's spelling ([`member_name`]).
    #[must_use]
    pub fn member<S: AsRef<str>>(&self, words: &[S]) -> String {
        member_name(self.settings.layout, words, &self.client.casings)
    }

    /// Every resource, depth first.
    #[must_use]
    pub fn all(&self) -> Vec<&TsResource<'a>> {
        let mut out = Vec::new();
        walk(&self.resources, &mut out);
        out
    }

    /// The resource with these keys.
    #[must_use]
    pub fn find(&self, keys: &[String]) -> Option<&TsResource<'a>> {
        self.all().into_iter().find(|r| r.keys == keys)
    }

    /// The resources from the top down to `r`, `r` included.
    #[must_use]
    pub fn chain(&self, r: &TsResource<'_>) -> Vec<&TsResource<'a>> {
        (1..=r.keys.len()).filter_map(|n| self.find(&r.keys[..n])).collect()
    }

    /// Where each model is declared. `TypeRef.path` is the Python module the converter writes
    /// (`x-ferrule-models`): a standalone API's own keys, else its parent's keys, so a
    /// non-standalone resource's models are found by the methods that use them.
    fn find_homes(&self) -> HashMap<String, Home> {
        let all = self.all();
        let mut homes = HashMap::new();
        for decl in &self.client.models {
            let r = &decl.r#ref;
            let dirs = ref_dirs(r);
            let declared = self
                .settings
                .model_homes
                .iter()
                .find(|(s, _)| *s == r.schema)
                .map(|(_, k)| k);
            let home = if let Some(keys) = declared {
                if keys.first().is_some_and(|k| k == "$shared") {
                    Home::Shared
                } else {
                    Home::Resource(keys.clone())
                }
            } else if dirs.first().is_some_and(|d| d == "shared") {
                Home::Shared
            } else if let Some(standalone) = all.iter().find(|x| x.keys == dirs && x.res.standalone) {
                Home::Resource(standalone.keys.clone())
            } else {
                let candidates: Vec<&&TsResource<'_>> = all
                    .iter()
                    .filter(|x| x.keys.len() == dirs.len() + 1 && x.keys[..dirs.len()] == dirs[..] && !x.res.standalone)
                    .collect();
                let user = candidates.iter().find(|x| {
                    x.res
                        .methods
                        .iter()
                        .any(|m| method_refs(m).iter().any(|u| u.schema == r.schema))
                });
                let by_name = candidates.iter().find(|x| {
                    let key = r.path.last().map(|n| n.written.as_str()).unwrap_or_default();
                    key.starts_with(singular(&words_of(&x.res.name)).join("_").as_str())
                });
                user.or(by_name)
                    .or(candidates.first())
                    .map_or(Home::Shared, |x| Home::Resource(x.keys.clone()))
            };
            homes.insert(r.schema.clone(), home);
        }
        homes
    }

    /// A model's TypeScript name: its `models` key in type-name spelling (`URLEndpointRequest`
    /// for `url_endpoint_request`), as Stainless's TypeScript generator spells it.
    #[must_use]
    pub fn type_of(&self, r: &TypeRef) -> String {
        type_name(&words_of(&r.name), &self.client.casings)
    }

    /// Where `r` is declared.
    #[must_use]
    pub fn home(&self, r: &TypeRef) -> Home {
        self.homes.get(&r.schema).cloned().unwrap_or(Home::Shared)
    }

    /// The models declared in a resource's file, sorted by name.
    #[must_use]
    pub fn models_of(&self, keys: &[String]) -> Vec<&'a ModelDecl> {
        let mut out: Vec<&ModelDecl> = self
            .client
            .models
            .iter()
            .filter(|d| self.home(&d.r#ref) == Home::Resource(keys.to_vec()))
            .collect();
        out.sort_by_key(|d| self.type_of(&d.r#ref));
        out
    }

    /// The `$shared` models, sorted by name.
    #[must_use]
    pub fn shared_models(&self) -> Vec<&'a ModelDecl> {
        let mut out: Vec<&ModelDecl> = self
            .client
            .models
            .iter()
            .filter(|d| self.home(&d.r#ref) == Home::Shared)
            .collect();
        out.sort_by_key(|d| self.type_of(&d.r#ref));
        out
    }

    /// The module (from `src/`, no `.ts`) that declares `r`.
    #[must_use]
    pub fn model_module(&self, r: &TypeRef) -> String {
        match self.home(r) {
            Home::Shared => "resources/shared".to_owned(),
            Home::Resource(keys) => self
                .find(&keys)
                .map_or_else(|| "resources/shared".to_owned(), TsResource::module),
        }
    }

    /// The namespace import another file uses for the models of `r`'s file: `v1API`, `Shared`.
    #[must_use]
    pub fn model_namespace(&self, r: &TypeRef) -> String {
        match self.home(r) {
            Home::Shared => "Shared".to_owned(),
            Home::Resource(keys) => self
                .find(&keys)
                .map_or_else(|| "Shared".to_owned(), |x| format!("{}API", x.class)),
        }
    }
}

/// `./x` or `../a/b`: the import specifier of module `target` from a file in `from_dir` (both
/// from `src/`).
#[must_use]
pub fn relative(from_dir: &str, target: &str) -> String {
    let from: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let to: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let ups = from.len() - common;
    let rest = to[common..].join("/");
    if ups == 0 {
        format!("./{rest}")
    } else {
        format!("{}{rest}", "../".repeat(ups))
    }
}

#[cfg(test)]
mod tests {
    use super::relative;

    #[test]
    fn computes_relative_imports() {
        assert_eq!(relative("resources", "core/resource"), "../core/resource");
        assert_eq!(relative("resources/ilgs", "resources/ilgs/v1/v1"), "./v1/v1");
        assert_eq!(relative("resources", "resources/ilgs/v1/v1"), "./ilgs/v1/v1");
        assert_eq!(relative("resources/a/b", "core/x"), "../../../core/x");
    }
}
