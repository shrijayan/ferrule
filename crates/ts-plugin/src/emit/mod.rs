//! The emitters: pure functions from the context to `(path from the SDK root, text)`.

pub mod decls;
pub mod resource;

use crate::ctx::Ctx;

/// The first line of every generated source file (Stainless names itself; Ferrule does too).
pub const HEADER: &str = "// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.";

/// A generated file: path from the SDK root and text.
pub type File = (String, String);

/// The files of `src/resources/**`.
#[must_use]
pub fn resource_files(ctx: &Ctx<'_>) -> Vec<File> {
    let mut out = Vec::new();
    for r in ctx.all() {
        out.push((format!("src/{}.ts", r.module()), resource::emit_resource(ctx, r)));
        if r.package {
            out.push((format!("src/{}.ts", r.dir()), resource::emit_shim(r)));
            out.push((format!("src/{}/index.ts", r.dir()), resource::emit_index(ctx, r)));
        }
    }
    out.push(("src/resources/index.ts".to_owned(), resource::emit_resources_index(ctx)));
    out
}
