//! Prints [`Decl`]s: response structs with their JSON metadata, params structs with their
//! serializers, and enums with their constants.

use std::fmt::Write as _;

use crate::naming::unexported;
use crate::printer::{ALIGN, comment};
use crate::types::{Ctx, Decl, GoField, Mode};

/// The struct tag of a field.
fn tag(f: &GoField, mode: Mode) -> String {
    let key = if mode == Mode::Query { "query" } else { "json" };
    let mut out = format!("`{key}:\"{}\"", f.wire);
    let api: Vec<&str> = [(f.required, "required"), (f.nullable, "nullable")]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, s)| *s)
        .collect();
    if !api.is_empty() {
        let _ = write!(out, " api:\"{}\"", api.join(","));
    }
    if let Some(format) = f.format {
        let _ = write!(out, " format:\"{format}\"");
    }
    out.push('`');
    out
}

fn struct_decl(
    out: &mut String,
    name: &str,
    doc: &str,
    fields: &[GoField],
    bases: &[String],
    mode: Mode,
    ctx: &mut Ctx<'_>,
) {
    comment(out, 0, doc);
    let _ = writeln!(out, "type {name} struct {{");
    let json = format!("{}JSON", unexported(name));
    for f in fields {
        comment(out, 1, &f.doc);
        let ty = if mode == Mode::Response {
            f.ty.clone()
        } else {
            format!("param.Field[{}]", f.ty)
        };
        let _ = writeln!(out, "\t{}{ALIGN}{ty}{ALIGN}{}", f.name, tag(f, mode));
    }
    if mode == Mode::Response {
        let _ = writeln!(out, "\tJSON{ALIGN}{json}{ALIGN}`json:\"-\"`");
    }
    for b in bases {
        let _ = writeln!(out, "\t{b}");
    }
    out.push_str("}\n\n");
    match mode {
        Mode::Response => {
            ctx.import_module("internal/apijson");
            comment(
                out,
                0,
                &format!("{json} contains the JSON metadata for the struct [{name}]"),
            );
            let _ = writeln!(out, "type {json} struct {{");
            for f in fields {
                let _ = writeln!(out, "\t{}{ALIGN}apijson.Field", f.name);
            }
            let _ = writeln!(out, "\traw{ALIGN}string");
            let _ = writeln!(out, "\tExtraFields{ALIGN}map[string]apijson.Field");
            out.push_str("}\n\n");
            let _ = write!(
                out,
                "func (r *{name}) UnmarshalJSON(data []byte) (err error) {{\n\treturn apijson.UnmarshalRoot(data, r)\n}}\n\n\
                 func (r {json}) RawJSON() string {{\n\treturn r.raw\n}}\n\n"
            );
        }
        Mode::Query => {
            ctx.import("net/url");
            ctx.import_module("internal/apiquery");
            ctx.import_module("internal/param");
            let settings = query_settings(ctx);
            comment(
                out,
                0,
                &format!("URLQuery serializes [{name}]'s query parameters as `url.Values`."),
            );
            let _ = write!(
                out,
                "func (r {name}) URLQuery() (v url.Values) {{\n\treturn apiquery.MarshalWithSettings(r, apiquery.QuerySettings{{\n{settings}\t}})\n}}\n\n"
            );
        }
        Mode::Body => {
            ctx.import_module("internal/apijson");
            ctx.import_module("internal/param");
            let _ = write!(
                out,
                "func (r {name}) MarshalJSON() (data []byte, err error) {{\n\treturn apijson.MarshalRoot(r)\n}}\n\n"
            );
        }
    }
}

/// The `apiquery.QuerySettings` fields from `query_settings`.
fn query_settings(ctx: &Ctx<'_>) -> String {
    use ferrule_sdk_model::model::{ArrayFormat, NestedFormat};
    let (array, nested) = ctx
        .client
        .settings
        .as_ref()
        .map_or((ArrayFormat::Comma, NestedFormat::Brackets), |s| {
            (s.array_format, s.nested_format)
        });
    let array = match array {
        ArrayFormat::Comma => "Comma",
        ArrayFormat::Repeat => "Repeat",
        ArrayFormat::Indices => "Indices",
        ArrayFormat::Brackets => "Brackets",
    };
    let nested = match nested {
        NestedFormat::Brackets => "Brackets",
        NestedFormat::Dots => "Dots",
    };
    format!(
        "\t\tArrayFormat:{ALIGN}apiquery.ArrayQueryFormat{array},\n\t\tNestedFormat:{ALIGN}apiquery.NestedQueryFormat{nested},\n"
    )
}

fn enum_decl(out: &mut String, name: &str, doc: &str, underlying: &str, members: &[(String, String)]) {
    comment(out, 0, doc);
    let _ = writeln!(out, "type {name} {underlying}\n\nconst (");
    for (constant, value) in members {
        let _ = writeln!(out, "\t{constant}{ALIGN}{name}{ALIGN}= {value}");
    }
    out.push_str(")\n\n");
    let names: Vec<&str> = members.iter().map(|(c, _)| c.as_str()).collect();
    let _ = write!(
        out,
        "func (r {name}) IsKnown() bool {{\n\tswitch r {{\n\tcase {}:\n\t\treturn true\n\t}}\n\treturn false\n}}\n\n",
        names.join(", ")
    );
}

/// Appends the declarations in order.
pub fn print_decls(out: &mut String, decls: &[Decl], ctx: &mut Ctx<'_>) {
    for d in decls {
        match d {
            Decl::Struct {
                name,
                doc,
                fields,
                bases,
                mode,
            } => struct_decl(out, name, doc, fields, bases, *mode, ctx),
            Decl::Enum {
                name,
                doc,
                underlying,
                members,
            } => enum_decl(out, name, doc, underlying, members),
        }
    }
}
