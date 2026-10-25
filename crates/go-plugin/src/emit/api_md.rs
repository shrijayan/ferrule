//! `api.md`: the shared types, then each service's types and methods with pkg.go.dev links.

use std::fmt::Write as _;

use ferrule_sdk_model::model::Type;

use super::resource::{Returns, returns};
use super::{Gen, Service, method_order, params_arg};
use crate::types::{Pkg, model_pkg};

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

/// A Go type in a method line: a root-package or `shared.` name, or a builtin.
fn type_link(g: &Gen<'_>, ty: &str) -> String {
    let module = &g.target.module_path;
    if let Some(name) = ty.strip_prefix("shared.") {
        link(module, "shared", Some("shared"), name)
    } else if ty.starts_with(|c: char| c.is_ascii_uppercase()) {
        link(module, &g.target.package_name, None, ty)
    } else {
        builtin(ty)
    }
}

fn section(out: &mut String, g: &Gen<'_>, s: &Service<'_>) {
    let hashes = "#".repeat(s.depth + 1);
    let _ = writeln!(out, "{hashes} {}\n", s.field);
    let mut ctx = g.ctx(Pkg::Root);
    let methods = method_order(&s.resource.methods);
    let mut types: Vec<String> = g
        .models_of(s)
        .into_iter()
        .map(|d| d.r#ref.name.written.clone())
        .collect();
    let mut rets = Vec::new();
    for m in &methods {
        let (ret, _) = returns(g, s, m, &mut ctx);
        if let Some(resp) = &m.response
            && !matches!(resp.ty, Type::Ref(_))
            && let Returns::Pointer(t) = &ret
            && !t.contains('.')
        {
            types.push(t.trim_start_matches("[]").to_owned());
        }
        rets.push(ret);
    }
    if !types.is_empty() {
        out.push_str("Response Types:\n\n");
        for t in &types {
            let _ = writeln!(out, "- {}", type_link(g, t));
        }
        out.push('\n');
    }
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
        if let Some(a) = params_arg(g.client, m) {
            args.push(format!("{a} {}", type_link(g, &format!("{}{name}Params", s.prefix))));
        }
        let results = match ret {
            Returns::Pointer(t) => format!("(\\*{}, {})", type_link(g, t), builtin("error")),
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
    let shared: Vec<&str> = g
        .client
        .models
        .iter()
        .filter(|d| model_pkg(&d.r#ref) == Pkg::Shared)
        .map(|d| d.r#ref.name.written.as_str())
        .collect();
    if !shared.is_empty() {
        out.push_str("# Shared Response Types\n\n");
        for name in shared {
            let _ = writeln!(out, "- {}", link(&g.target.module_path, "shared", Some("shared"), name));
        }
        out.push('\n');
    }
    for s in &g.services {
        section(&mut out, g, s);
    }
    let text = out.trim_end_matches('\n').to_owned() + "\n";
    ("api.md".to_owned(), text)
}
