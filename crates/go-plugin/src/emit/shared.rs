//! `shared/shared.go` (the `$shared` models) and `aliases.go` (the root package's aliases of
//! them, and of `apierror.Error`).

use std::fmt::Write as _;

use super::Gen;
use super::decls::print_decls;
use super::resource::all_model_decls;
use crate::printer::go_file;
use crate::types::{Ctx, Decl, Pkg, model_pkg};

/// v2's private aliases of `param.APIUnion` and `param.APIObject`, which params structs embed;
/// imagekit-go and knock-mgmt-go declare them after the imports of `aliases.go` and
/// `shared/shared.go`.
fn param_aliases(g: &Gen<'_>, ctx: &mut Ctx<'_>) -> &'static str {
    if !g.target.v2() {
        return "";
    }
    ctx.import_module("packages/param");
    "// aliased to make [param.APIUnion] private when embedding\ntype paramUnion = param.APIUnion\n\n\
     // aliased to make [param.APIObject] private when embedding\ntype paramObj = param.APIObject\n\n"
}

/// The declarations of every `$shared` model, in model order.
#[must_use]
pub fn shared_decls(g: &Gen<'_>) -> Vec<Decl> {
    let mut ctx = g.ctx(Pkg::Shared);
    g.client
        .models
        .iter()
        .filter(|d| model_pkg(&d.r#ref) == Pkg::Shared)
        .flat_map(|d| all_model_decls(g, d, &mut ctx))
        .collect()
}

/// `shared/shared.go`, if there are `$shared` models.
#[must_use]
pub fn shared_file(g: &Gen<'_>) -> Option<(String, String)> {
    let mut ctx = g.ctx(Pkg::Shared);
    let decls: Vec<Decl> = g
        .client
        .models
        .iter()
        .filter(|d| model_pkg(&d.r#ref) == Pkg::Shared)
        .flat_map(|d| all_model_decls(g, d, &mut ctx))
        .collect();
    if decls.is_empty() {
        return None;
    }
    let mut body = param_aliases(g, &mut ctx).to_owned();
    print_decls(&mut body, &decls, &mut ctx);
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    Some(("shared/shared.go".to_owned(), go_file("shared", &imports, &body)))
}

/// `aliases.go`.
#[must_use]
pub fn aliases_file(g: &Gen<'_>) -> (String, String) {
    let mut ctx = g.ctx(Pkg::Root);
    ctx.import_module("internal/apierror");
    let mut body = format!("{}type Error = apierror.Error\n\n", param_aliases(g, &mut ctx));
    let decls = shared_decls(g);
    if !decls.is_empty() {
        ctx.import_module("shared");
    }
    for d in &decls {
        let name = d.name();
        let _ = write!(
            body,
            "// This is an alias to an internal type.\ntype {name} = shared.{name}\n\n"
        );
        if let Decl::Enum { members, .. } = d {
            for (constant, _) in members {
                let _ = write!(
                    body,
                    "// This is an alias to an internal value.\nconst {constant} = shared.{constant}\n\n"
                );
            }
        }
    }
    let imports: Vec<String> = ctx.imports.into_iter().collect();
    (
        "aliases.go".to_owned(),
        go_file(&g.target.package_name, &imports, &body),
    )
}
