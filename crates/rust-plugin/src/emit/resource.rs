//! Resources: one module each (`src/resources/<key>.rs`, subresources nested), with a
//! `<Key>Resource` struct generic over the client's mode, its methods and their types.

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::{Method, Pagination, Resource};

use super::method::{self, Route, Site, TestCase};
use crate::context::{Gen, Module, has_content, resource_module};
use crate::doc::inline;
use crate::naming::{item, pascal};
use crate::types::Cx;

/// The tests of one resource (or the client), its subresources' nested.
#[derive(Debug, Clone, Default)]
pub struct ResourceTests {
    /// The test module's name: the resource's.
    pub name: String,
    pub cases: Vec<TestCase>,
    pub children: Vec<ResourceTests>,
}

/// What every method adds besides its module: the fake server's routes and the page schemes.
#[derive(Debug, Clone, Default)]
pub struct Collected {
    pub routes: Vec<Route>,
    pub pages: Vec<Pagination>,
}

/// The type of a resource: `AgencyResource`.
#[must_use]
pub fn resource_type(r: &Resource) -> String {
    format!("{}Resource", pascal(&r.name.words, "R", "Api"))
}

/// The accessor of a resource on its parent, and its module: `agency`.
#[must_use]
pub fn accessor(r: &Resource) -> String {
    item(&r.name.words)
}

/// The accessor functions of `resources` on a parent whose core and mode are `site`'s; `prefix`
/// is the path to their modules (`crate::resources::` from a client, empty from a resource).
#[must_use]
pub fn accessors(resources: &[Resource], prefix: &str, site: Site<'_>) -> Vec<String> {
    resources
        .iter()
        .filter(|r| has_content(r))
        .map(|r| {
            let path = format!("{prefix}{}::{}", accessor(r), resource_type(r));
            format!(
                "    /// The `{}` resource.\n    pub fn {}(&self) -> {path}<{}{}> {{\n        {path}::new({}, {})\n    }}\n",
                inline(&r.name.written),
                accessor(r),
                site.lifetime,
                site.mode_ty,
                site.core,
                site.mode
            )
        })
        .collect()
}

/// Emits the methods `methods` into `module` with `prefix` naming their types; returns the
/// functions that start each call, and their tests.
///
/// # Errors
///
/// A method's FER diagnostic.
pub fn methods(
    g: &Gen<'_>,
    module: &mut Module,
    prefix: &str,
    methods: &[Method],
    site: Site<'_>,
    call_prefix: &str,
    collected: &mut Collected,
) -> Result<(Vec<String>, Vec<TestCase>), FerruleError> {
    let mut functions = Vec::new();
    let mut tests = Vec::new();
    for m in methods {
        let mut cx = Cx { g, module, model: None };
        let out = method::emit(&mut cx, prefix, m, site, call_prefix)?;
        functions.extend(out.functions);
        tests.extend(out.tests);
        collected.routes.extend(out.route);
        collected.pages.extend(out.pages);
    }
    Ok((functions, tests))
}

/// Emits `r` and its subresources into `modules`.
///
/// # Errors
///
/// A method's FER diagnostic.
pub fn emit(
    g: &Gen<'_>,
    r: &Resource,
    modules: &mut Vec<Module>,
    collected: &mut Collected,
    call_prefix: &str,
) -> Result<ResourceTests, FerruleError> {
    let path = resource_module(&r.path);
    let mut module = take(modules, &path).unwrap_or_else(|| g.new_module(path.clone()));
    let ty = module.taken.claim(&resource_type(r));
    let site = Site {
        core: "self.core",
        mode: "self.mode",
        lifetime: "'a",
        mode_ty: ", M",
        params_path: "",
    };
    let mut tests = ResourceTests {
        name: accessor(r),
        ..ResourceTests::default()
    };
    let call_prefix = format!("{call_prefix}{}().", accessor(r));
    let mut functions = accessors(&r.subresources, "", site);
    let prefix = pascal(&r.name.words, "R", "Api");
    let (calls, cases) = methods(g, &mut module, &prefix, &r.methods, site, &call_prefix, collected)?;
    functions.extend(calls);
    tests.cases = cases;
    module.rt.extend(["Async", "Core"]);
    let header = format!(
        "/// The `{}` resource.\n#[derive(Debug, Clone, Copy)]\npub struct {ty}<'a, M = Async> {{\n    core: &'a Core,\n    mode: M,\n}}\n",
        inline(&r.name.written)
    );
    let mut body = format!(
        "impl<'a, M: Copy> {ty}<'a, M> {{\n    #[doc(hidden)]\n    pub fn new(core: &'a Core, mode: M) -> Self {{\n        Self {{ core, mode }}\n    }}\n"
    );
    for f in &functions {
        body.push('\n');
        body.push_str(f);
    }
    body.push_str("}\n");
    module.items.splice(0..0, [header, body]);
    modules.push(module);
    for s in r.subresources.iter().filter(|s| has_content(s)) {
        tests.children.push(emit(g, s, modules, collected, &call_prefix)?);
    }
    Ok(tests)
}

/// Removes and returns the module at `path`, if one was started.
pub fn take(modules: &mut Vec<Module>, path: &[String]) -> Option<Module> {
    let i = modules.iter().position(|m| m.path == path)?;
    Some(modules.remove(i))
}
