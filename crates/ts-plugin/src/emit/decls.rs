//! Prints [`Decl`]s: `export interface`, `export type` and the namespaces of their inline types.

use crate::comment::jsdoc;
use crate::pretty::{WIDTH, property_signature, render, type_alias};
use crate::types::{Decl, DeclKind};

fn sections(deprecated: bool) -> Vec<Vec<String>> {
    if deprecated {
        vec![vec!["@deprecated".to_owned()]]
    } else {
        Vec::new()
    }
}

/// The lines of `decl` at `indent` spaces, its namespace after it.
#[must_use]
pub fn print_decl(decl: &Decl, indent: usize) -> Vec<String> {
    let pad = " ".repeat(indent);
    let mut out = jsdoc(&decl.description, &sections(decl.deprecated), indent);
    match &decl.kind {
        DeclKind::Interface { extends, fields } => {
            let ext = if extends.is_empty() {
                String::new()
            } else {
                format!(" extends {}", extends.join(", "))
            };
            if fields.is_empty() {
                out.push(format!("{pad}export interface {}{ext} {{}}", decl.name));
            } else {
                out.push(format!("{pad}export interface {}{ext} {{", decl.name));
                for (i, f) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(String::new());
                    }
                    out.extend(jsdoc(&f.description, &sections(f.deprecated), indent + 2));
                    out.push(render(
                        &property_signature(&f.key, f.optional, &f.ty),
                        indent + 2,
                        WIDTH,
                    ));
                }
                out.push(format!("{pad}}}"));
            }
        }
        DeclKind::Alias(t) => out.push(render(&type_alias(&decl.name, t), indent, WIDTH)),
    }
    if !decl.nested.is_empty() {
        out.push(String::new());
        out.push(format!("{pad}export namespace {} {{", decl.name));
        for (i, d) in decl.nested.iter().enumerate() {
            if i > 0 {
                out.push(String::new());
            }
            out.extend(print_decl(d, indent + 2));
        }
        out.push(format!("{pad}}}"));
    }
    out
}
