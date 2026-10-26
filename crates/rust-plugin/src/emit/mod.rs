//! The emitters, and the order they run in: named models first (so that every module knows the
//! names taken in it), then resources, the client, the module tree and the crate's files.

pub mod client;
pub mod method;
pub mod project;
pub mod resource;

use ferrule_sdk_model::FerruleError;
use ferrule_sdk_model::model::Resource;

use crate::context::{Gen, Module, resource_module};
use crate::types::{Cx, Examples, declare_model};
use resource::{Collected, take};

/// Every generated file (not the runtime's), by path from the crate root.
///
/// # Errors
///
/// A FER diagnostic for something the target cannot generate.
pub fn generate(g: &Gen<'_>) -> Result<Vec<(String, String)>, FerruleError> {
    let mut modules: Vec<Module> = Vec::new();
    let mut examples = Examples::default();
    for d in &g.client.models {
        let Some(info) = g.models.get(&d.r#ref.schema) else {
            continue;
        };
        let mut module = take(&mut modules, &info.module).unwrap_or_else(|| g.new_module(info.module.clone()));
        let mut cx = Cx {
            g,
            module: &mut module,
            model: Some(&d.r#ref.schema),
        };
        let example = declare_model(&mut cx, d);
        examples.0.insert(d.r#ref.schema.clone(), example);
        modules.push(module);
    }
    let mut collected = Collected::default();
    let mut tests = Vec::new();
    for r in g.resources() {
        tests.push(resource::emit(g, r, &mut modules, &mut collected, "")?);
    }
    let (client_module, client_tests) = client::client_module(g, &mut collected)?;
    let top = tree(g, &mut modules);
    let options = client::required_options(g);
    let mut files: Vec<(String, String)> = modules.iter().map(|m| (m.file(), m.render())).collect();
    files.push((client_module.file(), client_module.render()));
    files.push(("src/blocking.rs".to_owned(), client::blocking_module(g)?));
    if !collected.pages.is_empty() {
        files.push(("src/pages.rs".to_owned(), method::pages_module(&collected.pages)));
    }
    files.push((
        "src/lib.rs".to_owned(),
        project::lib_rs(g, &top, !collected.pages.is_empty()),
    ));
    files.push(("src/testing/routes.rs".to_owned(), project::routes(&collected.routes)));
    files.push(("Cargo.toml".to_owned(), project::cargo_toml(g)));
    files.push(("rustfmt.toml".to_owned(), project::rustfmt_toml()));
    files.push(("README.md".to_owned(), project::readme(g, &tests, &examples, &options)));
    files.push((
        "tests/api.rs".to_owned(),
        project::api_tests(g, &client_tests, &tests, &examples, &options),
    ));
    Ok(files)
}

/// The doc of a module's `pub mod` line.
fn module_doc(g: &Gen<'_>, path: &[String]) -> String {
    if path == ["resources"] {
        return "/// The API's resources.\n".to_owned();
    }
    if path == ["shared"] {
        return "/// Models several resources share.\n".to_owned();
    }
    find(&g.client.resources, path).map_or_else(
        || format!("/// The `{}` models.\n", path.last().map_or("", String::as_str)),
        |r| format!("/// The `{}` resource.\n", crate::doc::inline(&r.name.written)),
    )
}

fn find<'r>(resources: &'r [Resource], path: &[String]) -> Option<&'r Resource> {
    let mut walk = resources.iter().collect::<Vec<_>>();
    while let Some(r) = walk.pop() {
        if resource_module(&r.path) == path {
            return Some(r);
        }
        walk.extend(&r.subresources);
    }
    None
}

/// The position of a module in stainless.yml order, so that `pub mod` lines follow it.
fn order(resources: &[Resource]) -> Vec<Vec<String>> {
    fn walk(r: &Resource, out: &mut Vec<Vec<String>>) {
        out.push(resource_module(&r.path));
        r.subresources.iter().for_each(|s| walk(s, out));
    }
    let mut out = Vec::new();
    for r in resources {
        walk(r, &mut out);
    }
    out
}

/// Adds every module's parents (`src/resources.rs` among them) and their `pub mod` lines;
/// returns the crate root's modules with their docs.
fn tree(g: &Gen<'_>, modules: &mut Vec<Module>) -> Vec<(String, String)> {
    let mut paths: Vec<Vec<String>> = modules.iter().map(|m| m.path.clone()).collect();
    let mut i = 0;
    while i < paths.len() {
        let path = paths[i].clone();
        if path.len() > 1 {
            let parent = path[..path.len() - 1].to_vec();
            if !paths.contains(&parent) {
                paths.push(parent);
            }
        }
        i += 1;
    }
    let order = order(&g.client.resources);
    let rank = |p: &Vec<String>| (order.iter().position(|o| o == p).unwrap_or(usize::MAX), p.clone());
    paths.sort_by_key(rank);
    for path in &paths {
        if !modules.iter().any(|m| &m.path == path) {
            let mut m = g.new_module(path.clone());
            if path.len() == 1 {
                m.doc = format!("The API's {}.", path[0]);
            }
            modules.push(m);
        }
    }
    let mut top = Vec::new();
    for path in &paths {
        let doc = module_doc(g, path);
        if let Some(module) = modules.iter_mut().find(|m| &m.path == path)
            && module.doc.is_empty()
        {
            doc.trim_start_matches("/// ").trim_end().clone_into(&mut module.doc);
        }
        match path.as_slice() {
            [one] => top.push((one.clone(), doc)),
            [.., last] => {
                let parent = &path[..path.len() - 1];
                if let Some(p) = modules.iter_mut().find(|m| m.path == parent) {
                    p.children.push((last.clone(), doc));
                }
            }
            [] => {}
        }
    }
    top
}
