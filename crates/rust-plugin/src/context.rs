//! What one generation shares: the names, where each named model lives, and the modules being
//! written.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

use ferrule_sdk_model::model::{Client, Name, Resource, Type};

use crate::meta::Names;
use crate::naming::{Taken, item, pascal};
use crate::types::Rendered;

/// Where a named model is declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// Module path from the crate root: `["shared"]`, `["resources", "stop"]`.
    pub module: Vec<String>,
    /// The type's name: `References`.
    pub name: String,
}

impl ModelInfo {
    /// `crate::shared::References`
    #[must_use]
    pub fn path(&self) -> String {
        format!("crate::{}::{}", self.module.join("::"), self.name)
    }
}

/// One generation.
#[derive(Debug)]
pub struct Gen<'c> {
    pub client: &'c Client,
    pub names: Names,
    /// Named models by schema.
    pub models: HashMap<String, ModelInfo>,
    /// For each model, every model it reaches through references.
    reaches: HashMap<String, HashSet<String>>,
    /// The fields of each model struct as declared, by wire name, so that a body of that model
    /// takes its fields' types rather than declaring them again.
    pub fields: RefCell<HashMap<String, Vec<(String, Rendered)>>>,
}

/// The module path of a resource: `["resources", "stop", "arrivals"]`.
#[must_use]
pub fn resource_module(path: &[Name]) -> Vec<String> {
    std::iter::once("resources".to_owned())
        .chain(path.iter().map(|n| item(&n.words)))
        .collect()
}

/// The module path of a model declared at stainless.yml `path` (resource keys, then the model
/// key): `$shared` models go to `shared`, the others to their resource's module.
fn model_module(path: &[Name]) -> Vec<String> {
    let resources = &path[..path.len().saturating_sub(1)];
    match resources.first() {
        None => vec!["shared".to_owned()],
        Some(first) if first.written == "shared" || first.written == "$shared" => vec!["shared".to_owned()],
        Some(_) => resource_module(resources),
    }
}

impl<'c> Gen<'c> {
    #[must_use]
    pub fn new(client: &'c Client, names: Names) -> Self {
        let mut taken: HashMap<Vec<String>, Taken> = HashMap::new();
        let mut models = HashMap::new();
        for d in &client.models {
            let module = model_module(&d.r#ref.path);
            let name = taken
                .entry(module.clone())
                .or_default()
                .claim(&pascal(&d.r#ref.name.words, "T", "Model"));
            models.insert(d.r#ref.schema.clone(), ModelInfo { module, name });
        }
        let reaches = client
            .models
            .iter()
            .map(|d| (d.r#ref.schema.clone(), reach(client, &d.ty)))
            .collect();
        Self {
            client,
            names,
            models,
            reaches,
            fields: RefCell::default(),
        }
    }

    /// Whether model `from` reaches model `to` through references.
    #[must_use]
    pub fn reaches(&self, from: &str, to: &str) -> bool {
        from == to || self.reaches.get(from).is_some_and(|r| r.contains(to))
    }

    /// Every module a model or resource lives in, with the model names declared there.
    #[must_use]
    pub fn new_module(&self, path: Vec<String>) -> Module {
        let mut taken = Taken::default();
        for info in self.models.values().filter(|i| i.module == path) {
            taken.claim(&info.name);
        }
        Module {
            path,
            taken,
            ..Module::default()
        }
    }

    /// Top-level resources with something to generate.
    pub fn resources(&self) -> impl Iterator<Item = &'c Resource> {
        self.client.resources.iter().filter(|r| has_content(r))
    }
}

/// Whether a resource has methods, or subresources that have.
#[must_use]
pub fn has_content(r: &Resource) -> bool {
    !r.methods.is_empty() || r.subresources.iter().any(has_content)
}

/// Every model `ty` reaches, directly or through other models.
fn reach(client: &Client, ty: &Type) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut pending = Vec::new();
    refs(ty, &mut pending);
    while let Some(schema) = pending.pop() {
        if seen.insert(schema.clone())
            && let Some(d) = client.models.iter().find(|d| d.r#ref.schema == schema)
        {
            refs(&d.ty, &mut pending);
        }
    }
    seen
}

fn refs(ty: &Type, out: &mut Vec<String>) {
    match ty {
        Type::Scalar { .. } | Type::Enum { .. } => {}
        Type::Array { items: inner } | Type::Map { values: inner } | Type::Nullable { inner } => refs(inner, out),
        Type::Object(o) => {
            out.extend(o.bases.iter().map(|b| b.schema.clone()));
            for f in &o.fields {
                refs(&f.ty, out);
            }
            if let Some(extra) = &o.extra {
                refs(extra, out);
            }
        }
        Type::Union(u) => u.variants.iter().for_each(|v| refs(&v.ty, out)),
        Type::Ref(r) => out.push(r.schema.clone()),
    }
}

/// One generated module (one file), filled item by item.
#[derive(Debug, Default)]
pub struct Module {
    /// From the crate root: `["resources", "agency"]`.
    pub path: Vec<String>,
    pub doc: String,
    /// Names taken by types declared here.
    pub taken: Taken,
    /// `pub mod` children, with their docs.
    pub children: Vec<(String, String)>,
    /// Names imported from `crate::rt`.
    pub rt: BTreeSet<&'static str>,
    /// Whether a type derives serde's traits.
    pub serde: bool,
    /// Items in order: resource struct, methods, then types as they are first used.
    pub items: Vec<String>,
}

impl Module {
    /// The file it is written to: `src/resources/agency.rs`.
    #[must_use]
    pub fn file(&self) -> String {
        format!("src/{}.rs", self.path.join("/"))
    }

    /// The module's text.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        for line in self.doc.lines() {
            if line.is_empty() {
                out.push_str("//!\n");
            } else {
                let _ = writeln!(out, "//! {line}");
            }
        }
        out.push('\n');
        for (child, doc) in &self.children {
            out.push_str(doc);
            let _ = writeln!(out, "pub mod {child};");
        }
        if !self.children.is_empty() {
            out.push('\n');
        }
        let mut uses: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        if !self.rt.is_empty() {
            uses.insert("crate::rt", self.rt.iter().copied().collect());
        }
        if self.serde {
            uses.insert("serde", vec!["Deserialize", "Serialize"]);
        }
        for (root, names) in &uses {
            if let [one] = names.as_slice() {
                let _ = writeln!(out, "use {root}::{one};");
            } else {
                let _ = writeln!(out, "use {root}::{{{}}};", names.join(", "));
            }
        }
        if !uses.is_empty() {
            out.push('\n');
        }
        out.push_str(&self.items.join("\n"));
        out
    }
}
