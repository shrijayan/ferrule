//! `api.md`: the shared types, then each service's types and methods with pkg.go.dev links.
//!
//! v2 lists the param variants of the models params use under "Params Types" before the
//! "Response Types" (imagekit-go, knock-mgmt-go); a model that is not an object is listed in
//! both when both use it.

use std::fmt::Write as _;

use ferrule_sdk_model::model::{ModelDecl, Type};

use super::resource::{Returns, returns};
use super::{Gen, Service, method_order};
use crate::types::{Pkg, model_name, model_pkg, v2_model_names};

fn link(module: &str, pkg: &str, rel: Option<&str>, name: &str) -> String {
    let base = rel.map_or_else(
        || format!("https://pkg.go.dev/{module}"),
        |r| format!("https://pkg.go.dev/{module}/{r}"),
    );
    format!("<a href=\"{base}\">{pkg}</a>.<a href=\"{base}#{name}\">{name}</a>")
}

fn builtin(name: &str) -> String {
    format!("<a href=\"https://pkg.go.dev/builtin#{name}\">{name}</a>")
}

/// A Go type in a method line: a root-package or `shared.` name, or a builtin; a slice links
/// its element (`[]imagekit.File`).
fn type_link(g: &Gen<'_>, ty: &str) -> String {
    let module = &g.target.module_path;
    if let Some(item) = ty.strip_prefix("[]") {
        format!("[]{}", type_link(g, item))
    } else if let Some(name) = ty.strip_prefix("shared.") {
        link(module, "shared", Some("shared"), name)
    } else if ty.starts_with(|c: char| c.is_ascii_uppercase()) {
        link(module, &g.target.package_name, None, ty)
    } else {
        builtin(ty)
    }
}

/// The models of `models` listed as params and as responses, by Go name.
fn model_lists(g: &Gen<'_>, models: &[&ModelDecl]) -> (Vec<String>, Vec<String>) {
    let mut params = Vec::new();
    let mut responses = Vec::new();
    for d in models {
        let (name, param) = if g.target.v2() {
            v2_model_names(g.client, &d.r#ref, g.casings)
        } else {
            let name = model_name(&d.r#ref, g.casings);
            (name.clone(), name)
        };
        let in_params = g.param_models.contains(&d.r#ref.schema);
        if g.target.v2() && in_params {
            params.push(param);
        }
        if !g.target.v2() || g.response_models.contains(&d.r#ref.schema) || !in_params {
            responses.push(name);
        }
    }
    (params, responses)
}

fn list(out: &mut String, title: &str, items: &[String], link: impl Fn(&str) -> String) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "{title}\n");
    for t in items {
        let _ = writeln!(out, "- {}", link(t));
    }
    out.push('\n');
}

fn section(out: &mut String, g: &Gen<'_>, s: &Service<'_>) {
    let hashes = "#".repeat(s.depth + 1);
    let _ = writeln!(out, "{hashes} {}\n", s.field);
    let mut ctx = g.ctx(Pkg::Root);
    let methods = method_order(&s.resource.methods);
    let (params, mut types) = model_lists(g, &g.models_of(s));
    let mut rets = Vec::new();
    for m in &methods {
        let (ret, _) = returns(g, s, m, &mut ctx);
        // A named model is listed with the models; a page's item type is listed as a response.
        let inline = match (&ret, &m.pagination) {
            (Returns::Page { item, .. }, Some(p)) => (!matches!(p.item, Type::Ref(_))).then_some(item),
            (Returns::Pointer(t), _) => m.response.as_ref().filter(|r| !matches!(r.ty, Type::Ref(_))).map(|_| t),
            _ => None,
        };
        if let Some(t) = inline.filter(|t| !t.contains('.')) {
            types.push(t.trim_start_matches("[]").to_owned());
        }
        rets.push(ret);
    }
    list(out, "Params Types:", &params, |t| type_link(g, t));
    list(out, "Response Types:", &types, |t| type_link(g, t));
    if methods.is_empty() {
        return;
    }
    out.push_str("Methods:\n\n");
    let module = &g.target.module_path;
    for (m, ret) in methods.iter().zip(&rets) {
        let name = g.method_name(m);
        let mut args = vec!["ctx <a href=\"https://pkg.go.dev/context\">context</a>.<a href=\"https://pkg.go.dev/context#Context\">Context</a>".to_owned()];
        for (n, t) in g.path_params(m) {
            args.push(format!("{n} {}", builtin(t)));
        }
        if let Some(a) = g.params_arg(m) {
            args.push(format!("{a} {}", type_link(g, &format!("{}{name}Params", s.prefix))));
        }
        let results = match ret {
            Returns::Pointer(t) => format!("(\\*{}, {})", type_link(g, t), builtin("error")),
            Returns::Page { page, item } => format!(
                "(\\*{}[{}], {})",
                link(module, "pagination", Some("packages/pagination"), page),
                type_link(g, item),
                builtin("error")
            ),
            Returns::None => builtin("error"),
        };
        let _ = writeln!(
            out,
            "- <code title=\"{} {}\">{}.<a href=\"https://pkg.go.dev/{module}#{}.{name}\">{name}</a>({}) {results}</code>",
            m.http_method.as_str(),
            m.path,
            s.accessor,
            s.service,
            args.join(", ")
        );
    }
    out.push('\n');
}

/// `api.md`.
#[must_use]
pub fn api_md(g: &Gen<'_>) -> (String, String) {
    let mut out = String::new();
    let shared: Vec<&ModelDecl> = g
        .client
        .models
        .iter()
        .filter(|d| model_pkg(&d.r#ref) == Pkg::Shared)
        .collect();
    let (params, responses) = model_lists(g, &shared);
    let module = &g.target.module_path;
    let shared_link = |name: &str| link(module, "shared", Some("shared"), name);
    list(&mut out, "# Shared Params Types", &params, shared_link);
    list(&mut out, "# Shared Response Types", &responses, shared_link);
    for s in &g.services {
        section(&mut out, g, s);
    }
    let text = out.trim_end_matches('\n').to_owned() + "\n";
    ("api.md".to_owned(), text)
}
