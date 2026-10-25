//! `api.md`: every resource's types and methods, and a file of its own for each standalone API
//! (`src/resources/<path>/api.md`), linked from its parent's section.

use super::File;
use super::resource::{exports_of, plan_methods};
use crate::ctx::{Ctx, TsResource};
use crate::naming::words_of;
use crate::types::Mapper;

/// The file api.md links a resource's names to: its module, or a standalone API's shim.
fn link(r: &TsResource<'_>) -> String {
    if r.res.standalone {
        format!("./src/{}.ts", r.dir())
    } else {
        format!("./src/{}.ts", r.module())
    }
}

/// The types and methods of one resource, under a heading of `level` `#`s.
fn section(ctx: &Ctx<'_>, r: &TsResource<'_>, level: usize, out: &mut Vec<String>, files: &mut Vec<File>) {
    let hashes = "#".repeat(level);
    out.push(format!("{hashes} {}", r.class));
    let href = link(r);
    let mut mapper = Mapper::new(ctx, &r.module());
    let plans = plan_methods(ctx, r, &mut mapper);
    let params: Vec<String> = plans.iter().filter_map(|p| p.params_type.clone()).collect();
    let types: Vec<String> = exports_of(ctx, r).into_iter().filter(|t| !params.contains(t)).collect();
    if !types.is_empty() {
        out.push(String::new());
        out.push("Types:".to_owned());
        out.push(String::new());
        for t in types {
            out.push(format!("- <code><a href=\"{href}\">{t}</a></code>"));
        }
    }
    if !plans.is_empty() {
        out.push(String::new());
        out.push("Methods:".to_owned());
        out.push(String::new());
        let chain = ctx.chain(r);
        let accessor = r.accessor(&chain);
        for p in &plans {
            let mut args: Vec<String> = super::resource::positional(ctx, p.m)
                .iter()
                .map(|q| ctx.member(&words_of(&q.field.name)))
                .collect();
            if p.params_type.is_some() {
                args.push("{ ...params }".to_owned());
            }
            out.push(format!(
                "- <code title=\"{} {}\">{accessor}.<a href=\"{href}\">{}</a>({}) -> {}</code>",
                p.m.http_method.as_str(),
                p.m.path,
                p.name,
                args.join(", "),
                // A model of another module by its name alone: `Shared.ResponseWrapper` -> `ResponseWrapper`.
                p.response.rsplit('.').next().unwrap_or(&p.response)
            ));
        }
    }
    for c in &r.children {
        out.push(String::new());
        if c.res.standalone {
            let path = format!("src/{}/api.md", c.dir());
            out.push(format!("{} [{}]({path})", "#".repeat(level + 1), c.class));
            let mut own = Vec::new();
            section(ctx, c, 1, &mut own, files);
            files.push((path, own.join("\n") + "\n"));
        } else {
            section(ctx, c, level + 1, out, files);
        }
    }
}

/// `api.md` and the standalone APIs' own `api.md` files.
#[must_use]
pub fn emit_api_md(ctx: &Ctx<'_>) -> Vec<File> {
    let mut files = Vec::new();
    let mut out: Vec<String> = Vec::new();
    let shared = ctx.shared_models();
    if !shared.is_empty() {
        out.push("# Shared".to_owned());
        out.push(String::new());
        out.push("Types:".to_owned());
        out.push(String::new());
        for d in shared {
            out.push(format!(
                "- <code><a href=\"./src/resources/shared.ts\">{}</a></code>",
                ctx.type_of(&d.r#ref)
            ));
        }
    }
    for r in &ctx.resources {
        if !out.is_empty() {
            out.push(String::new());
        }
        section(ctx, r, 1, &mut out, &mut files);
    }
    files.push(("api.md".to_owned(), out.join("\n") + "\n"));
    files
}
