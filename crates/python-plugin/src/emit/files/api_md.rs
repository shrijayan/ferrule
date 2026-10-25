//! api.md: every resource's types and methods, as the oracle SDKs list them.

use super::input::{FilesMethod, FilesResource, ParamLocation, ProjectFilesInput};
use crate::js;

/// `from <module> import a, b`, parenthesized one per line past `width` columns.
#[must_use]
pub fn import_line<S: AsRef<str>>(module: &str, names: &[S], width: usize) -> String {
    let names: Vec<&str> = names.iter().map(AsRef::as_ref).collect();
    let line = format!("from {module} import {}", names.join(", "));
    if js::len(&line) <= width {
        return line;
    }
    let mut out = format!("from {module} import (\n");
    for n in &names {
        out.push_str("    ");
        out.push_str(n);
        out.push_str(",\n");
    }
    out.push(')');
    out
}

fn file_path(pkg: &str, module: &str) -> String {
    format!("src/{pkg}/{}.py", module.replace('.', "/"))
}

fn signature(pkg: &str, m: &FilesMethod) -> String {
    let path: Vec<&str> = m
        .params
        .iter()
        .filter(|p| p.location == ParamLocation::Path)
        .map(|p| p.py_name.as_str())
        .collect();
    let mut args: Vec<String> = Vec::new();
    if let Some((last, rest)) = path.split_last() {
        args.push((*last).to_owned());
        if !rest.is_empty() {
            args.push("\\*".to_owned());
            args.extend(rest.iter().map(|p| (*p).to_owned()));
        }
    }
    if let Some(params) = &m.params_module {
        args.push(format!("\\*\\*<a href=\"{}\">params</a>", file_path(pkg, params)));
    }
    args.join(", ")
}

fn method_line(pkg: &str, accessor: &str, r: &FilesResource, m: &FilesMethod) -> String {
    let ret = m.response.as_ref().map_or_else(
        || "None".to_owned(),
        |res| format!("<a href=\"./{}\">{}</a>", file_path(pkg, &res.module), res.name),
    );
    format!(
        "- <code title=\"{} {}\">client.{accessor}.<a href=\"./{}\">{}</a>({}) -> {ret}</code>",
        m.http_method.as_str(),
        m.path,
        file_path(pkg, &r.module),
        m.name,
        signature(pkg, m),
    )
}

fn section(pkg: &str, r: &FilesResource, parent: &str, depth: usize, blocks: &mut Vec<String>) {
    let accessor = if parent.is_empty() {
        r.name.clone()
    } else {
        format!("{parent}.{}", r.name)
    };
    let mut out = vec![format!("{} {}", "#".repeat(depth), r.title)];
    if !r.types.is_empty() {
        out.push("Types:".to_owned());
        out.push(format!(
            "```python\n{}\n```",
            import_line(&format!("{pkg}.{}", r.types_module), &r.types, 120)
        ));
    }
    if !r.methods.is_empty() {
        out.push("Methods:".to_owned());
        let lines: Vec<String> = r.methods.iter().map(|m| method_line(pkg, &accessor, r, m)).collect();
        out.push(lines.join("\n"));
    }
    blocks.push(out.join("\n\n"));
    for sub in &r.subresources {
        section(pkg, sub, &accessor, depth + 1, blocks);
    }
}

/// The text of api.md.
#[must_use]
pub fn emit_api_md(input: &ProjectFilesInput) -> String {
    let pkg = &input.package_name;
    let mut blocks: Vec<String> = Vec::new();
    if !input.shared_types.is_empty() {
        blocks.push(format!(
            "# Shared Types\n\n```python\n{}\n```",
            import_line(&format!("{pkg}.types"), &input.shared_types, 120)
        ));
    }
    for r in &input.resources {
        section(pkg, r, "", 1, &mut blocks);
    }
    format!("{}\n", blocks.join("\n\n"))
}
