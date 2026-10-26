//! Prints [`Decl`]s: response structs with their JSON metadata, params structs with their
//! serializers, and enums with their constants.

use std::fmt::Write as _;

use crate::naming::unexported;
use crate::printer::{ALIGN, comment};
use crate::types::{Ctx, Decl, GoField, Mode, opt_scalar};

/// The struct tag of a field.
fn tag(f: &GoField) -> String {
    let mut out = format!("`{}:\"{}\"", f.key, f.wire);
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

/// A field's doc: its description, and in v2 the values of an enum-typed field.
fn field_doc(f: &GoField, v2: bool) -> String {
    if !v2 || f.enum_values.is_empty() {
        return f.doc.clone();
    }
    let any = format!("Any of {}.", f.enum_values.join(", "));
    if f.doc.trim().is_empty() {
        any
    } else {
        format!("{}\n\n{any}", f.doc.trim_end())
    }
}

/// v2's response struct: metadata in an inline `JSON` struct of `respjson.Field`s.
fn response_v2(out: &mut String, name: &str, fields: &[GoField], bases: &[String], ctx: &mut Ctx<'_>) {
    ctx.import_module("internal/apijson");
    ctx.import_module("packages/respjson");
    for f in fields {
        comment(out, 1, &field_doc(f, true));
        let _ = writeln!(out, "\t{}{ALIGN}{}{ALIGN}{}", f.name, f.ty, tag(f));
    }
    for b in bases {
        let _ = writeln!(out, "\t{b}");
    }
    out.push_str(
        "\t// JSON contains metadata for fields, check presence with [respjson.Field.Valid].\n\tJSON struct {\n",
    );
    for f in fields {
        let _ = writeln!(out, "\t\t{}{ALIGN}respjson.Field", f.name);
    }
    let _ = write!(
        out,
        "\t\tExtraFields{ALIGN}map[string]respjson.Field\n\t\traw{ALIGN}string\n\t}} `json:\"-\"`\n}}\n\n\
         // Returns the unmodified JSON received from the API\n\
         func (r {name}) RawJSON() string {{ return r.JSON.raw }}\n\
         func (r *{name}) UnmarshalJSON(data []byte) error {{\n\treturn apijson.UnmarshalRoot(data, r)\n}}\n\n"
    );
}

/// v2's params struct (imagekit-go, knock-mgmt-go): an optional scalar is a `param.Opt[T]`,
/// every optional field and every required non-scalar is `omitzero`, query fields are hidden
/// from JSON, and `paramObj` is embedded.
fn params_v2(out: &mut String, name: &str, fields: &[GoField], bases: &[String], ctx: &mut Ctx<'_>) {
    let is_scalar = |f: &GoField| opt_scalar(&f.ty) && f.enum_values.is_empty();
    let json = fields.iter().any(|f| f.key == "json");
    let query = fields.iter().any(|f| f.key == "query");
    if json || fields.iter().any(|f| (!f.required || f.nullable) && is_scalar(f)) {
        ctx.import_module("packages/param");
    }
    for f in fields {
        comment(out, 1, &field_doc(f, true));
        // A path field is a required plain value; a nullable scalar is a `param.Opt` even when
        // required (knock-mgmt-go's `Label`), and params tag no `nullable`.
        let scalar = is_scalar(f) || f.key == "path";
        let opt = scalar && (!f.required || f.nullable);
        let ty = if opt {
            format!("param.Opt[{}]", f.ty)
        } else {
            f.ty.clone()
        };
        let omitzero = if scalar && !opt { "" } else { ",omitzero" };
        let mut tag = format!("`{}:\"{}{omitzero}\"", f.key, f.wire);
        if f.required {
            tag.push_str(" api:\"required\"");
        }
        if let Some(format) = f.format {
            let _ = write!(tag, " format:\"{format}\"");
        }
        if f.key != "json" {
            tag.push_str(" json:\"-\"");
        }
        tag.push('`');
        let _ = writeln!(out, "\t{}{ALIGN}{ty}{ALIGN}{tag}", f.name);
    }
    for b in bases {
        let _ = writeln!(out, "\t{b}");
    }
    out.push_str("\tparamObj\n}\n\n");
    if json {
        ctx.import_module("internal/apijson");
        let _ = write!(
            out,
            "func (r {name}) MarshalJSON() (data []byte, err error) {{\n\ttype shadow {name}\n\treturn param.MarshalObject(r, (*shadow)(&r))\n}}\nfunc (r *{name}) UnmarshalJSON(data []byte) error {{\n\treturn apijson.UnmarshalRoot(data, r)\n}}\n\n"
        );
    }
    if query {
        ctx.import("net/url");
        ctx.import_module("internal/apiquery");
        let settings = query_settings(ctx);
        comment(
            out,
            0,
            &format!("URLQuery serializes [{name}]'s query parameters as `url.Values`."),
        );
        let _ = write!(
            out,
            "func (r {name}) URLQuery() (v url.Values, err error) {{\n\treturn apiquery.MarshalWithSettings(r, apiquery.QuerySettings{{\n{settings}\t}})\n}}\n\n"
        );
    }
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
    if ctx.v2 {
        if mode == Mode::Response {
            response_v2(out, name, fields, bases, ctx);
        } else {
            params_v2(out, name, fields, bases, ctx);
        }
        return;
    }
    let json = format!("{}JSON", unexported(name));
    for f in fields {
        comment(out, 1, &f.doc);
        let ty = if mode == Mode::Response {
            f.ty.clone()
        } else {
            format!("param.Field[{}]", f.ty)
        };
        let _ = writeln!(out, "\t{}{ALIGN}{ty}{ALIGN}{}", f.name, tag(f));
    }
    if mode == Mode::Response {
        let _ = writeln!(out, "\tJSON{ALIGN}{json}{ALIGN}`json:\"-\"`");
    }
    for b in bases {
        let _ = writeln!(out, "\t{b}");
    }
    out.push_str("}\n\n");
    if mode == Mode::Response {
        {
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
        return;
    }
    // Params: a body serializer for the JSON fields, a query one for the query fields (both
    // for a struct that mixes them; an empty struct is a body).
    ctx.import_module("internal/param");
    if fields.is_empty() || fields.iter().any(|f| f.key == "json") {
        ctx.import_module("internal/apijson");
        let _ = write!(
            out,
            "func (r {name}) MarshalJSON() (data []byte, err error) {{\n\treturn apijson.MarshalRoot(r)\n}}\n\n"
        );
    }
    if fields.iter().any(|f| f.key == "query") {
        ctx.import("net/url");
        ctx.import_module("internal/apiquery");
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

fn enum_decl(out: &mut String, name: &str, doc: &str, underlying: &str, members: &[(String, String)], v2: bool) {
    comment(out, 0, doc);
    let _ = writeln!(out, "type {name} {underlying}\n\nconst (");
    for (constant, value) in members {
        let _ = writeln!(out, "\t{constant}{ALIGN}{name}{ALIGN}= {value}");
    }
    out.push_str(")\n\n");
    // v2 enums have no `IsKnown`.
    if v2 {
        return;
    }
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
            } => enum_decl(out, name, doc, underlying, members, ctx.v2),
        }
    }
}
