//! Assembles the files of the SDK from one [`SdkClient`]: `_client.py`, `resources/**`, `types/**`
//! (`pagination.py` is rendered by the runtime), then the project files and the agent context.
//! Everything is emitted from Forge's `generate`, which sees every group; `generate_group` emits
//! nothing (Forge gaps G7, G10).

use super::agent::emit_agent_files;
use super::client::emit_client_file;
use super::common::{HEADER, method_params, method_ref, py_string, resource_module, return_model};
use super::files::emit_project_files;
use super::files_input::{ProjectMeta, to_project_files_input};
use super::models::{ModelFile, emit_alias_file, emit_model_file};
use super::package_init::{emit_package_init, emit_version};
use super::params::emit_params_file;
use super::resources::{Mode, class_name, emit_resource_file};
use crate::diagnostics::FerruleError;
use crate::js;
use crate::model::{SdkClient, SdkMethod, SdkObject, SdkResource, SdkType};
use crate::printer::PythonPrinter;

/// Output tree: relative path -> file contents, sorted by path.
pub type FileTree = Vec<(String, String)>;

/// A map that keeps first-insertion order, as a JavaScript `Map` does.
#[derive(Debug, Default)]
struct OrderedMap<V> {
    entries: Vec<(String, V)>,
}

impl<V> OrderedMap<V> {
    fn entry(&mut self, key: &str, default: impl FnOnce() -> V) -> &mut V {
        let i = if let Some(i) = self.entries.iter().position(|(k, _)| k == key) {
            i
        } else {
            self.entries.push((key.to_owned(), default()));
            self.entries.len() - 1
        };
        &mut self.entries[i].1
    }

    fn set_value(&mut self, key: &str, value: V) {
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => self.entries.push((key.to_owned(), value)),
        }
    }
}

fn module_path(pkg: &str, module: &str) -> String {
    format!("src/{pkg}/{}.py", module.replace('.', "/"))
}

#[must_use]
pub fn all_resources(resources: &[SdkResource]) -> Vec<&SdkResource> {
    resources
        .iter()
        .flat_map(|r| std::iter::once(r).chain(all_resources(&r.subresources)))
        .collect()
}

const WRAPPERS: [(Mode, &str); 6] = [
    (Mode::Sync, ""),
    (Mode::Async, ""),
    (Mode::Sync, "WithRawResponse"),
    (Mode::Async, "WithRawResponse"),
    (Mode::Sync, "WithStreamingResponse"),
    (Mode::Async, "WithStreamingResponse"),
];

/// `resources/__init__.py` or `resources/<a>/__init__.py`: the resource classes in order.
fn resources_init(members: &[&SdkResource]) -> String {
    let mut p = PythonPrinter::new(&[HEADER]);
    let mut names: Vec<String> = Vec::new();
    for r in members {
        let leaf = r.path.last().map_or("", String::as_str);
        for (mode, suffix) in WRAPPERS {
            let name = class_name(r, mode, suffix);
            p.import_from(&format!(".{leaf}"), &name);
            names.push(name);
        }
    }
    p.line("__all__ = [");
    for n in &names {
        p.line(&format!("    {},", py_string(n)));
    }
    p.line("]");
    p.render()
}

/// Files by path, and the `types` package exports: directory ('' for `types`) -> module -> names.
struct Out {
    files: OrderedMap<String>,
    exports: OrderedMap<OrderedMap<String>>,
    pkg: String,
}

impl Out {
    fn export(&mut self, module: &str, name: &str) {
        let parts: Vec<&str> = module.split('.').collect();
        let dir = parts[..parts.len() - 1].join(".");
        self.exports
            .entry(&dir, OrderedMap::default)
            .set_value(parts[parts.len() - 1], name.to_owned());
    }

    /// Emits the types a method synthesizes: its inline response model and its params `TypedDict`.
    fn method_types(&mut self, resource: Option<&SdkResource>, method: &SdkMethod) {
        let ret = return_model(resource, method);
        if let Some(ret) = &ret
            && let Some(inline) = ret.inline
        {
            let content = match inline {
                SdkType::Object(obj) => emit_model_file(&ModelFile {
                    r#ref: &ret.r#ref,
                    ty: obj,
                    prefix_children: true,
                }),
                other => emit_alias_file(&ret.r#ref, other),
            };
            self.files
                .set_value(&module_path(&self.pkg, &format!("types.{}", ret.r#ref.module)), content);
            self.export(&ret.r#ref.module, &ret.r#ref.name);
        }
        if !method_params(method).is_empty() {
            let r = method_ref(resource, method, "Params");
            self.files.set_value(
                &module_path(&self.pkg, &format!("types.{}", r.module)),
                emit_params_file(resource, method),
            );
            self.export(&r.module, &r.name);
        }
    }

    /// Every named model's module, and `types/shared/__init__.py`.
    fn named_models(&mut self, client: &SdkClient) {
        let pkg = self.pkg.clone();
        let shared: Vec<_> = client
            .models
            .iter()
            .filter(|d| d.r#ref.module.starts_with("shared."))
            .collect();
        if !shared.is_empty() {
            let names = shared
                .iter()
                .map(|d| d.r#ref.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            self.exports.entry("", OrderedMap::default).set_value("shared", names);
        }
        for decl in &client.models {
            // `{ description: decl.description, ...decl.type }`: the type's own description wins.
            let ty = SdkObject {
                description: decl.ty.description.clone().or_else(|| Some(decl.description.clone())),
                ..decl.ty.clone()
            };
            self.files.set_value(
                &module_path(&pkg, &format!("types.{}", decl.r#ref.module)),
                emit_model_file(&ModelFile {
                    r#ref: &decl.r#ref,
                    ty: &ty,
                    prefix_children: false,
                }),
            );
            if !decl.r#ref.module.starts_with("shared.") {
                self.export(&decl.r#ref.module, &decl.r#ref.name);
            }
        }
        if !shared.is_empty() {
            let mut p = PythonPrinter::new(&[HEADER]);
            for d in &shared {
                p.import_from(
                    &format!(".{}", &d.r#ref.module["shared.".len()..]),
                    &format!("{0} as {0}", d.r#ref.name),
                );
            }
            self.files
                .set_value(&format!("src/{pkg}/types/shared/__init__.py"), p.render());
        }
    }

    /// `types/__init__.py` and `types/<dir>/__init__.py`, re-exporting what each declares.
    fn types_inits(&mut self) {
        let pkg = self.pkg.clone();
        let exports = std::mem::take(&mut self.exports.entries);
        for (dir, modules) in exports {
            let init = if dir.is_empty() {
                "types.__init__".to_owned()
            } else {
                format!("types.{dir}.__init__")
            };
            let mut p = PythonPrinter::new(&[HEADER]);
            p.import_from("__future__", "annotations");
            for (module, names) in &modules.entries {
                for name in names.split(", ") {
                    p.import_from(&format!(".{module}"), &format!("{name} as {name}"));
                }
            }
            self.files.set_value(&module_path(&pkg, &init), p.render());
        }
    }
}

fn sorted(files: OrderedMap<String>) -> FileTree {
    let mut out = files.entries;
    out.sort_by(|(a, _), (b, _)| js::cmp(a, b));
    out
}

/// Every code file of the SDK (`src/<pkg>/**`) but the agent context, sorted by path.
///
/// # Errors
///
/// A FER diagnostic for what the emitters cannot generate.
pub fn emit_code_files(client: &SdkClient, meta: Option<&ProjectMeta>) -> Result<FileTree, FerruleError> {
    let pkg = client.package.name.clone();
    let mut out = Out {
        files: OrderedMap::default(),
        exports: OrderedMap::default(),
        pkg: pkg.clone(),
    };
    out.exports.set_value("", OrderedMap::default());

    out.named_models(client);

    for resource in all_resources(&client.resources) {
        out.files.set_value(
            &module_path(&pkg, &resource_module(resource)),
            emit_resource_file(client, resource)?,
        );
        if !resource.subresources.is_empty() {
            let mut members: Vec<&SdkResource> = resource.subresources.iter().collect();
            members.push(resource);
            out.files.set_value(
                &module_path(&pkg, &format!("resources.{}.__init__", resource.path.join("."))),
                resources_init(&members),
            );
        }
        for method in &resource.methods {
            out.method_types(Some(resource), method);
        }
    }
    for method in &client.client_methods {
        out.method_types(None, method);
    }
    if !client.resources.is_empty() {
        let top: Vec<&SdkResource> = client.resources.iter().collect();
        out.files
            .set_value(&module_path(&pkg, "resources.__init__"), resources_init(&top));
    }

    out.types_inits();

    if let Some(settings) = &client.settings {
        out.files
            .set_value(&module_path(&pkg, "_client"), emit_client_file(client)?);
        out.files.set_value(
            &module_path(&pkg, "__init__"),
            emit_package_init(&pkg, &settings.class_name),
        );
        out.files.set_value(
            &module_path(&pkg, "_version"),
            emit_version(&pkg, meta.map_or("0.0.1", |m| m.version.as_str())),
        );
    }
    Ok(sorted(out.files))
}

/// Every file of the SDK: the code files, the project files (with `meta`), and the agent
/// context, sorted by path.
///
/// # Errors
///
/// A FER diagnostic for what the emitters cannot generate.
pub fn emit_root_files(client: &SdkClient, meta: Option<&ProjectMeta>) -> Result<FileTree, FerruleError> {
    let mut files = OrderedMap {
        entries: emit_code_files(client, meta)?,
    };
    if let Some(meta) = meta
        && let Some(input) = to_project_files_input(client, meta)
    {
        for (path, text) in emit_project_files(&input) {
            files.set_value(&path, text);
        }
    }
    for (path, text) in emit_agent_files(client, meta) {
        files.set_value(&path, text);
    }
    Ok(sorted(files))
}
