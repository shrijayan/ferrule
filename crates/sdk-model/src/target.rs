//! One target's view of the model: [`Client::target`] and [`Client::for_target`] (ADR 0017).

use std::collections::HashSet;

use crate::model::{Client, Method, ModelDecl, Object, Resource, TargetSettings, Type};

impl Client {
    /// The settings of `target` (a stainless.yml `targets` key such as `typescript`).
    #[must_use]
    pub fn target(&self, target: &str) -> Option<&TargetSettings> {
        self.targets.iter().find(|(k, _)| k == target).map(|(_, t)| t)
    }

    /// The SDK `target` generates: without the methods and models stainless.yml leaves out for
    /// it, without the resources left with nothing, and with every reference to a model it does
    /// not generate inlined, as if the config had not named that model (an object keeps the
    /// schema as its `origin`). Pure.
    ///
    /// A recursive model left out for `target` stays a reference, which the target cannot
    /// generate. An `allOf` base left out contributes its fields ahead of the object's own.
    #[must_use]
    pub fn for_target(&self, target: &str) -> Client {
        let mut out = self.target_view(target);
        let reachable = reachable(&out);
        out.models.retain(|d| reachable.contains(&d.r#ref.schema));
        out
    }

    /// [`Self::for_target`], but with every model generated for `target` that no kept method
    /// reaches as well (Stainless declares them all), after the reachable ones and inlined as
    /// they are. Pure.
    #[must_use]
    pub fn for_target_with_models(&self, target: &str) -> Client {
        let view = self.target_view(target);
        let mut out = view.clone();
        let reachable = reachable(&out);
        out.models.retain(|d| reachable.contains(&d.r#ref.schema));
        for d in view.models {
            if !out.models.iter().any(|k| k.r#ref == d.r#ref) {
                out.models.push(d);
            }
        }
        out
    }

    /// What `target` generates and every model generated for it, inlined; before the models no
    /// method reaches are dropped.
    fn target_view(&self, target: &str) -> Client {
        let dropped: Vec<&ModelDecl> = self.models.iter().filter(|d| !d.is_for(target)).collect();
        let mut inliner = Inliner {
            dropped: &dropped,
            stack: Vec::new(),
        };
        let mut out = self.clone();
        out.resources = std::mem::take(&mut out.resources)
            .into_iter()
            .filter_map(|r| keep(r, target, &mut inliner))
            .collect();
        out.client_methods.retain(|m| m.is_for(target));
        for m in &mut out.client_methods {
            inliner.method(m);
        }
        out.models.retain(|d| d.is_for(target));
        for d in &mut out.models {
            inliner.ty(&mut d.ty);
        }
        out
    }
}

/// `r` with only what `target` generates; `None` when filtering left it empty. A resource that had
/// nothing to begin with (a standalone API that only declares models) is kept.
fn keep(mut r: Resource, target: &str, inliner: &mut Inliner<'_>) -> Option<Resource> {
    let had_any = !r.methods.is_empty() || !r.subresources.is_empty();
    r.methods.retain(|m| m.is_for(target));
    for m in &mut r.methods {
        inliner.method(m);
    }
    r.subresources = std::mem::take(&mut r.subresources)
        .into_iter()
        .filter_map(|s| keep(s, target, inliner))
        .collect();
    let empty = r.methods.is_empty() && r.subresources.is_empty();
    (!(had_any && empty) || r.standalone).then_some(r)
}

/// Replaces references to `dropped` models by their types.
struct Inliner<'d> {
    dropped: &'d [&'d ModelDecl],
    /// Models being inlined, so that a recursive one stays a reference.
    stack: Vec<String>,
}

impl Inliner<'_> {
    fn method(&mut self, m: &mut Method) {
        for p in m
            .path_params
            .iter_mut()
            .chain(&mut m.query_params)
            .chain(&mut m.header_params)
        {
            self.ty(&mut p.field.ty);
        }
        if let Some(b) = &mut m.body {
            self.ty(&mut b.ty);
        }
        if let Some(r) = &mut m.response {
            self.ty(&mut r.ty);
        }
        if let Some(p) = &mut m.pagination {
            self.ty(&mut p.item);
        }
        if let Some(Some(e)) = m.streaming.as_mut().map(|s| s.event.as_mut()) {
            self.ty(e);
        }
    }

    /// The type of the dropped model `schema`, itself inlined; `None` when it is not dropped or is
    /// being inlined already.
    fn inlined(&mut self, schema: &str) -> Option<Type> {
        if self.stack.iter().any(|s| s == schema) {
            return None;
        }
        let decl = *self.dropped.iter().find(|d| d.r#ref.schema == schema)?;
        let mut ty = decl.ty.clone();
        self.stack.push(schema.to_owned());
        self.ty(&mut ty);
        self.stack.pop();
        if let Type::Object(o) = &mut ty {
            o.origin = Some(schema.to_owned());
            if !decl.description.is_empty() {
                o.description = Some(decl.description.clone());
            }
        }
        Some(ty)
    }

    fn object(&mut self, o: &mut Object) {
        let mut bases = Vec::with_capacity(o.bases.len());
        let mut inherited = Vec::new();
        for base in std::mem::take(&mut o.bases) {
            match self.inlined(&base.schema) {
                None => bases.push(base),
                Some(Type::Object(b)) => {
                    bases.extend(b.bases);
                    inherited.extend(b.fields);
                    if o.extra.is_none() {
                        o.extra = b.extra;
                    }
                }
                // A member with no shape to extend, as the adapter skips it.
                Some(_) => {}
            }
        }
        o.bases = bases;
        if !inherited.is_empty() {
            inherited.append(&mut o.fields);
            o.fields = inherited;
        }
        for f in &mut o.fields {
            self.ty(&mut f.ty);
        }
        if let Some(extra) = &mut o.extra {
            self.ty(extra);
        }
    }

    fn ty(&mut self, ty: &mut Type) {
        match ty {
            Type::Scalar { .. } | Type::Enum { .. } => {}
            Type::Array { items: inner } | Type::Map { values: inner } | Type::Nullable { inner } => self.ty(inner),
            Type::Object(o) => self.object(o),
            Type::Union(u) => {
                for v in &mut u.variants {
                    self.ty(&mut v.ty);
                }
            }
            Type::Ref(r) => {
                if let Some(inlined) = self.inlined(&r.schema.clone()) {
                    *ty = inlined;
                }
            }
        }
    }
}

/// The schemas of every model the client's methods reach, directly or through other models.
fn reachable(client: &Client) -> HashSet<String> {
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
            Type::Union(u) => {
                for v in &u.variants {
                    refs(&v.ty, out);
                }
            }
            Type::Ref(r) => out.push(r.schema.clone()),
        }
    }
    fn method_refs(m: &Method, out: &mut Vec<String>) {
        for p in m.path_params.iter().chain(&m.query_params).chain(&m.header_params) {
            refs(&p.field.ty, out);
        }
        for ty in [
            m.body.as_ref().map(|b| &b.ty),
            m.response.as_ref().map(|r| &r.ty),
            m.pagination.as_ref().map(|p| &p.item),
            m.streaming.as_ref().and_then(|s| s.event.as_ref()),
        ]
        .into_iter()
        .flatten()
        {
            refs(ty, out);
        }
    }
    fn resource_refs(r: &Resource, out: &mut Vec<String>) {
        for m in &r.methods {
            method_refs(m, out);
        }
        for s in &r.subresources {
            resource_refs(s, out);
        }
    }
    let mut pending = Vec::new();
    for r in &client.resources {
        resource_refs(r, &mut pending);
    }
    for m in &client.client_methods {
        method_refs(m, &mut pending);
    }
    let mut seen = HashSet::new();
    while let Some(schema) = pending.pop() {
        if !seen.insert(schema.clone()) {
            continue;
        }
        if let Some(d) = client.models.iter().find(|d| d.r#ref.schema == schema) {
            refs(&d.ty, &mut pending);
        }
    }
    seen
}
