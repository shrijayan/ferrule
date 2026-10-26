//! Reference cycles between named models, which Stainless breaks the way Pydantic needs:
//!
//! - a reference to a model that reaches a cycle (is in one, or refers to one through others) is
//!   a string (`List["WorkflowStep"]`) whose import goes to the bottom of the module, after the
//!   declarations, so that the modules of a cycle can import each other;
//! - a union model in a cycle is a `TypeAliasType` (under `if TYPE_CHECKING or not PYDANTIC_V1`);
//! - `types/__init__.py` rebuilds the classes that hold such references once every module is
//!   imported (`model_rebuild`, `update_forward_refs` on Pydantic v1).
//!
//! Derived from the knock-mgmt (`WorkflowStep`) and imagekit (`Overlay`, `Transformation`)
//! oracles.

use std::collections::{HashMap, HashSet};

use crate::emit::common::typed_extra;
use crate::model::{SdkModelDecl, SdkObject, SdkRef, SdkType};

/// Which named models take part in reference cycles.
#[derive(Debug, Default, Clone)]
pub struct Cycles {
    /// Models that reach a cycle: references to them are strings imported at the bottom.
    deferred: HashSet<SdkRef>,
    /// Models in a cycle: a union model among them is a `TypeAliasType`.
    cyclic: HashSet<SdkRef>,
}

/// Every named model `ty` refers to, through fields, bases, variants, items and values.
pub fn refs_of(ty: &SdkType, out: &mut Vec<SdkRef>) {
    match ty {
        SdkType::Ref(r) => out.push(r.clone()),
        SdkType::Array(t) | SdkType::Map(t) => refs_of(t, out),
        SdkType::Union { variants, .. } => {
            for v in variants {
                refs_of(v, out);
            }
        }
        SdkType::Object(o) => object_refs(o, out),
        SdkType::Primitive(_) | SdkType::Unknown | SdkType::Literal(_) => {}
    }
}

/// [`refs_of`] for an object: its bases, the types of its fields and of the extra values it
/// declares (`__pydantic_extra__`).
pub fn object_refs(obj: &SdkObject, out: &mut Vec<SdkRef>) {
    out.extend(obj.bases.iter().cloned());
    for f in &obj.fields {
        refs_of(&f.ty, out);
    }
    if let Some(extra) = typed_extra(obj) {
        refs_of(extra, out);
    }
}

/// The named models a declaration refers to.
fn decl_refs(decl: &SdkModelDecl) -> Vec<SdkRef> {
    let mut out = Vec::new();
    match &decl.alias {
        Some(ty) => refs_of(ty, &mut out),
        None => object_refs(&decl.ty, &mut out),
    }
    out
}

impl Cycles {
    /// The cycles among `models`.
    #[must_use]
    pub fn of(models: &[SdkModelDecl]) -> Self {
        let edges: HashMap<&SdkRef, Vec<SdkRef>> = models.iter().map(|d| (&d.r#ref, decl_refs(d))).collect();
        // What each model reaches in one or more steps (models are few, so a walk from each will do).
        let reach = |from: &SdkRef| -> HashSet<SdkRef> {
            let mut seen: HashSet<SdkRef> = HashSet::new();
            let mut stack: Vec<SdkRef> = edges.get(from).cloned().unwrap_or_default();
            while let Some(r) = stack.pop() {
                if seen.insert(r.clone())
                    && let Some(next) = edges.get(&r)
                {
                    stack.extend(next.iter().cloned());
                }
            }
            seen
        };
        let reaches: Vec<(&SdkRef, HashSet<SdkRef>)> = models.iter().map(|d| (&d.r#ref, reach(&d.r#ref))).collect();
        let cyclic: HashSet<SdkRef> = reaches
            .iter()
            .filter(|(r, seen)| seen.contains(*r))
            .map(|(r, _)| (*r).clone())
            .collect();
        let deferred = reaches
            .iter()
            .filter(|(r, seen)| cyclic.contains(*r) || seen.iter().any(|s| cyclic.contains(s)))
            .map(|(r, _)| (*r).clone())
            .collect();
        Self { deferred, cyclic }
    }

    /// Whether references to `r` are strings imported at the bottom of the module.
    #[must_use]
    pub fn is_deferred(&self, r: &SdkRef) -> bool {
        self.deferred.contains(r)
    }

    /// Whether `r` is in a cycle.
    #[must_use]
    pub fn is_cyclic(&self, r: &SdkRef) -> bool {
        self.cyclic.contains(r)
    }

    /// Whether the class `obj` (a module's root) must be rebuilt once every module is imported:
    /// it refers to a deferred model. A class with a base model is left out, as the imagekit
    /// oracle leaves out `ImageOverlay(BaseOverlay)`.
    #[must_use]
    pub fn needs_rebuild(&self, obj: &SdkObject) -> bool {
        if !obj.bases.is_empty() {
            return false;
        }
        let mut refs = Vec::new();
        object_refs(obj, &mut refs);
        refs.iter().any(|r| self.is_deferred(r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SdkField;

    fn r(name: &str) -> SdkRef {
        SdkRef {
            name: name.into(),
            module: name.to_lowercase(),
        }
    }

    fn object(refs: &[&str]) -> SdkObject {
        SdkObject {
            fields: refs
                .iter()
                .map(|n| SdkField {
                    wire_name: n.to_lowercase(),
                    py_name: n.to_lowercase(),
                    required: true,
                    description: String::new(),
                    ty: SdkType::Array(Box::new(SdkType::Ref(r(n)))),
                })
                .collect(),
            ..SdkObject::default()
        }
    }

    fn class(name: &str, refs: &[&str]) -> SdkModelDecl {
        SdkModelDecl {
            r#ref: r(name),
            description: String::new(),
            ty: object(refs),
            alias: None,
        }
    }

    #[test]
    fn defers_models_that_reach_a_cycle() {
        let step = SdkModelDecl {
            alias: Some(SdkType::Union {
                variants: vec![SdkType::Ref(r("Delay")), SdkType::Ref(r("Branch"))],
                discriminator: None,
            }),
            ..class("Step", &[])
        };
        let models = [
            step,
            class("Delay", &[]),
            class("Branch", &["Step"]),
            class("Workflow", &["Step"]),
            class("Plain", &["Delay"]),
        ];
        let c = Cycles::of(&models);
        let deferred: Vec<&str> = ["Step", "Delay", "Branch", "Workflow", "Plain"]
            .into_iter()
            .filter(|n| c.is_deferred(&r(n)))
            .collect();
        assert_eq!(deferred, ["Step", "Branch", "Workflow"]);
        assert!(c.is_cyclic(&r("Step")) && c.is_cyclic(&r("Branch")) && !c.is_cyclic(&r("Workflow")));
        assert!(c.needs_rebuild(&object(&["Workflow"])));
        assert!(!c.needs_rebuild(&object(&["Delay"])));
        let derived = SdkObject {
            bases: vec![r("Plain")],
            ..object(&["Step"])
        };
        assert!(!c.needs_rebuild(&derived));
    }
}
