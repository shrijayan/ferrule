//! The model files, in one pass over the declarations with a [`View`] flag: `lib/**/models/*.rb`
//! (converter declarations with YARD tags), `rbi/**/models/*.rbi` (Sorbet) and
//! `sig/**/models/*.rbs` (RBS).

use std::fmt::Write as _;

use super::{Gen, View};

use crate::printer::{COMMENT_WIDTH, RB_WIDTH, comment, len, literal, wrap};
use crate::tree::{Class, Decl, EnumDecl, ModelFile, RField, UnionDecl};
use crate::types::RType;

/// The comment block of a declaration: its description, then `@see` (`.rb` only).
fn header(g: &Gen<'_>, out: &mut String, indent: usize, description: &str, see: Option<&crate::tree::See>, view: View) {
    let mut lines = wrap(description, COMMENT_WIDTH);
    if view == View::Rb
        && let Some(see) = see
    {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("@see {}", see.render(&g.m)));
    }
    comment(out, indent, &lines);
}

/// The opening `module` lines of a file's namespace: `module M`, `module Models` and any
/// enclosing modules of the declaration; returns the indent of the declaration.
fn open(out: &mut String, g: &Gen<'_>, f: &ModelFile) -> usize {
    let _ = writeln!(out, "module {}", g.m);
    let _ = writeln!(out, "  module Models");
    let segs = &f.decl.r().segs;
    let mut indent = 4;
    for s in &segs[..segs.len() - 1] {
        let _ = writeln!(out, "{}module {s}", " ".repeat(indent));
        indent += 2;
    }
    indent
}

fn close(out: &mut String, from: usize) {
    let mut indent = from;
    while indent >= 2 {
        indent -= 2;
        let _ = writeln!(out, "{}end", " ".repeat(indent));
    }
}

/// One model file in one view: its path from the gem root and its text.
#[must_use]
pub fn model_file(g: &Gen<'_>, f: &ModelFile, view: View) -> (String, String) {
    let mut out = String::new();
    match view {
        View::Rb => out.push_str("# frozen_string_literal: true\n\n"),
        View::Rbi => out.push_str("# typed: strong\n\n"),
        View::Rbs => {}
    }
    let indent = open(&mut out, g, f);
    decl(g, &mut out, indent, &f.decl, view);
    close(&mut out, indent);
    (format!("{}/models/{}.{}", g.lib(view), f.path, view.ext()), out)
}

fn decl(g: &Gen<'_>, out: &mut String, indent: usize, d: &Decl, view: View) {
    match (d, view) {
        (Decl::Class(c), View::Rb) => rb_class(g, out, indent, c),
        (Decl::Class(c), View::Rbi) => rbi_class(g, out, indent, c),
        (Decl::Class(c), View::Rbs) => rbs_class(g, out, indent, c),
        (Decl::Enum(e), _) => enum_decl(g, out, indent, e, view),
        (Decl::Union(u), _) => union_decl(g, out, indent, u, view),
    }
}

/// `extend ...Converter` and `include ...RequestParameters` of a params class.
fn request_parameters(g: &Gen<'_>, out: &mut String, pad: &str) {
    let _ = writeln!(out, "{pad}extend {}::Internal::Type::RequestParameters::Converter", g.m);
    let _ = writeln!(out, "{pad}include {}::Internal::Type::RequestParameters", g.m);
}

fn base(g: &Gen<'_>, c: &Class) -> String {
    c.base
        .as_ref()
        .map_or_else(|| format!("{}::Internal::Type::BaseModel", g.m), |b| b.doc(&g.m))
}

// ---- lib/**.rb ----

/// A `required`/`optional` declaration as Stainless's generator writes it, before rubocop: on
/// one line, or one argument per line two columns in when the statement is longer than 106
/// columns (Stainless measures it from a four-column indent, whatever its depth). `rubocop -a`
/// then aligns the arguments and breaks the lambdas that run past 110 columns
/// (`ferrule generate`, as the gem's `rake format:rb`).
#[must_use]
pub fn rb_statement(indent: usize, keyword: &str, args: &[String]) -> String {
    let pad = " ".repeat(indent);
    let one = format!("{keyword} {}", args.join(", "));
    if 4 + len(&one) <= RB_WIDTH {
        return format!("{pad}{one}\n");
    }
    let cont = format!(",\n{pad}  ");
    let rest: Vec<String> = args[1..].iter().map(|a| long_lambda(a, indent + 2)).collect();
    format!("{pad}{keyword} {}{cont}{}\n", args[0], rest.join(&cont))
}

/// `-> { M::Internal::Type::ArrayOf[X] }` as Stainless's generator breaks it when `X` alone
/// does not fit in 106 columns: a `do` block with the brackets open.
fn long_lambda(arg: &str, indent: usize) -> String {
    let Some(start) = arg.find("-> { ") else {
        return arg.to_owned();
    };
    let (prefix, lambda) = arg.split_at(start);
    let Some(body) = lambda.strip_prefix("-> { ").and_then(|b| b.strip_suffix(" }")) else {
        return arg.to_owned();
    };
    let Some(open) = body.find('[') else {
        return arg.to_owned();
    };
    let (head, inner) = body.split_at(open + 1);
    let Some(inner) = inner.strip_suffix(']') else {
        return arg.to_owned();
    };
    if 4 + len(inner) <= RB_WIDTH || !(head.ends_with("ArrayOf[") || head.ends_with("HashOf[")) {
        return arg.to_owned();
    }
    let pad = " ".repeat(indent);
    format!("{prefix}-> do\n{pad}  {head}\n{pad}    {inner}\n{pad}  ]\n{pad}end")
}

fn yard_return(g: &Gen<'_>, f: &RField) -> String {
    let t = f.ty.yard(&g.m);
    if !f.required || f.nullable {
        format!("{t}, nil")
    } else {
        t
    }
}

fn yard_param(g: &Gen<'_>, f: &RField) -> String {
    let t = f.ty.yard(&g.m);
    if f.nullable { format!("{t}, nil") } else { t }
}

/// A description on one line.
#[must_use]
pub fn collapse(description: &str) -> String {
    description
        .split(ferrule_sdk_model::js::is_space)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A description as one line, cut at 80 columns, for `@param` tags.
#[must_use]
pub fn short(description: &str) -> String {
    let joined = collapse(description);
    let units: Vec<u16> = joined.encode_utf16().collect();
    if units.len() <= COMMENT_WIDTH {
        joined
    } else {
        String::from_utf16_lossy(&units[..COMMENT_WIDTH])
    }
}

/// `@param` lines: blank comment lines between them when any has a description.
pub fn param_tags(out: &mut String, pad: &str, prefix: &str, params: &[(String, String, String)]) {
    let spaced = params.iter().any(|(_, _, d)| !d.is_empty());
    for (i, (name, ty, d)) in params.iter().enumerate() {
        if i > 0 && spaced {
            let _ = writeln!(out, "{pad}#");
        }
        if d.is_empty() {
            let _ = writeln!(out, "{pad}# {prefix}@param {name} [{ty}]");
        } else {
            let _ = writeln!(out, "{pad}# {prefix}@param {name} [{ty}] {d}");
        }
    }
}

fn rb_class(g: &Gen<'_>, out: &mut String, indent: usize, c: &Class) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    header(g, out, indent, "", c.see.as_ref(), View::Rb);
    let _ = writeln!(out, "{pad}class {} < {}", c.r.name(), base(g, c));
    let mut first = true;
    let mut sep = |out: &mut String| {
        if !first {
            out.push('\n');
        }
        first = false;
    };
    if c.params {
        sep(out);
        request_parameters(g, out, &inner);
    }
    for f in &c.fields {
        sep(out);
        let _ = writeln!(out, "{inner}# @!attribute {}", f.name);
        for l in wrap(&f.description, COMMENT_WIDTH) {
            if l.is_empty() {
                let _ = writeln!(out, "{inner}#");
            } else {
                let _ = writeln!(out, "{inner}#   {l}");
            }
        }
        let _ = writeln!(out, "{inner}#");
        let _ = writeln!(out, "{inner}#   @return [{}]", yard_return(g, f));
        let mut args = vec![format!(":{}", f.name), f.ty.rb_field(&g.m)];
        if let Some(api) = f.api_name().filter(|_| !c.params) {
            args.push(format!("api_name: {}", crate::printer::symbol(api)));
        }
        if f.nullable {
            args.push("nil?: true".to_owned());
        }
        let keyword = if f.required { "required" } else { "optional" };
        out.push_str(&rb_statement(indent + 2, keyword, &args));
    }
    sep(out);
    let mut sig: Vec<String> = c
        .fields
        .iter()
        .map(|f| {
            if f.required {
                format!("{}:", f.name)
            } else {
                format!("{}: nil", f.name)
            }
        })
        .collect();
    let mut tags: Vec<(String, String, String)> = c
        .fields
        .iter()
        .map(|f| (f.name.clone(), yard_param(g, f), short(&f.description)))
        .collect();
    if c.params {
        sig.push("request_options: {}".to_owned());
        tags.push((
            "request_options".to_owned(),
            format!("{}::RequestOptions, Hash{{Symbol=>Object}}", g.m),
            String::new(),
        ));
    }
    let _ = writeln!(out, "{inner}# @!method initialize({})", sig.join(", "));
    let mut intro = Vec::new();
    if c.fields
        .iter()
        .any(|f| short(&f.description) != collapse(&f.description))
    {
        intro.extend(wrap(
            &format!(
                "Some parameter documentations has been truncated, see {{{}}} for more details.",
                c.r.doc(&g.m)
            ),
            COMMENT_WIDTH,
        ));
        intro.push(String::new());
    }
    let described = wrap(&c.description, COMMENT_WIDTH);
    if !described.is_empty() {
        intro.extend(described);
        intro.push(String::new());
    }
    for l in intro {
        if l.is_empty() {
            let _ = writeln!(out, "{inner}#");
        } else {
            let _ = writeln!(out, "{inner}#   {l}");
        }
    }
    param_tags(out, &inner, "  ", &tags);
    for n in &c.nested {
        out.push('\n');
        decl(g, out, indent + 2, n, View::Rb);
    }
    let _ = writeln!(out, "{pad}end");
}

// ---- rbi/**.rbi ----

/// Sorbet's reader type of a field, as `attr_reader` returns it.
fn rbi_reader(g: &Gen<'_>, c: &Class, f: &RField) -> String {
    let t = rbi_read(g, c, &f.ty);
    if !f.required || f.nullable {
        format!("T.nilable({t})")
    } else {
        t
    }
}

/// The type a field holds: enums of params as `OrSymbol`, else as returned.
fn rbi_read(g: &Gen<'_>, c: &Class, t: &RType) -> String {
    if c.params { t.rbi_params(&g.m) } else { t.rbi(&g.m) }
}

/// Sorbet's writer type: a model also as a hash; for an optional field, every model inside it.
fn rbi_writer(g: &Gen<'_>, c: &Class, f: &RField) -> String {
    let t = if matches!(f.ty, RType::Model(_)) {
        f.ty.rbi_input(&g.m)
    } else if !f.required {
        f.ty.rbi_with(&g.m, true, c.params)
    } else {
        rbi_read(g, c, &f.ty)
    };
    if f.nullable { format!("T.nilable({t})") } else { t }
}

fn nilable(t: String, nullable: bool) -> String {
    if nullable { format!("T.nilable({t})") } else { t }
}

fn rbi_class(g: &Gen<'_>, out: &mut String, indent: usize, c: &Class) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    let _ = writeln!(out, "{pad}class {} < {}", c.r.name(), base(g, c));
    if c.params {
        request_parameters(g, out, &inner);
        out.push('\n');
    }
    let _ = writeln!(
        out,
        "{inner}OrHash = T.type_alias {{ T.any({}, {}::Internal::AnyHash) }}",
        c.r.code(&g.m),
        g.m
    );
    for f in &c.fields {
        out.push('\n');
        comment(out, indent + 2, &wrap(&f.description, COMMENT_WIDTH));
        let reader = rbi_reader(g, c, f);
        let writer = rbi_writer(g, c, f);
        if reader == writer {
            let _ = writeln!(out, "{inner}sig {{ returns({reader}) }}");
            let _ = writeln!(out, "{inner}attr_accessor :{}", f.name);
        } else {
            let _ = writeln!(out, "{inner}sig {{ returns({reader}) }}");
            let _ = writeln!(out, "{inner}attr_reader :{}", f.name);
            out.push('\n');
            let _ = writeln!(out, "{inner}sig {{ params({}: {writer}).void }}", f.name);
            let _ = writeln!(out, "{inner}attr_writer :{}", f.name);
        }
    }
    out.push('\n');
    let mut params: Vec<String> = c
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.name, nilable(f.ty.rbi_input(&g.m), f.nullable)))
        .collect();
    let mut hash: Vec<String> = c
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.name, nilable(rbi_read(g, c, &f.ty), f.nullable)))
        .collect();
    let mut args: Vec<(String, &str)> = c
        .fields
        .iter()
        .map(|f| {
            let a = if f.required {
                format!("{}:", f.name)
            } else {
                format!("{}: nil", f.name)
            };
            (a, f.description.as_str())
        })
        .collect();
    if c.params {
        params.push(format!("request_options: {}::RequestOptions::OrHash", g.m));
        hash.push(format!("request_options: {}::RequestOptions", g.m));
        args.push(("request_options: {}".to_owned(), ""));
    }
    comment(out, indent + 2, &wrap(&c.description, COMMENT_WIDTH));
    let _ = writeln!(
        out,
        "{inner}sig {{ params({}).returns(T.attached_class) }}",
        params.join(", ")
    );
    def_new(out, indent + 2, "self.new", &args);
    out.push('\n');
    let _ = writeln!(out, "{inner}sig {{ override.returns({{ {} }}) }}", hash.join(", "));
    let _ = writeln!(out, "{inner}def to_hash");
    let _ = writeln!(out, "{inner}end");
    for n in &c.nested {
        out.push('\n');
        decl(g, out, indent + 2, n, View::Rbi);
    }
    let _ = writeln!(out, "{pad}end");
}

/// `def name(args)` with an empty body; arguments with a description get it as a comment.
pub fn def_new(out: &mut String, indent: usize, name: &str, args: &[(String, &str)]) {
    let pad = " ".repeat(indent);
    if args.iter().all(|(_, d)| d.is_empty()) {
        let list: Vec<&str> = args.iter().map(|(a, _)| a.as_str()).collect();
        if list.is_empty() {
            let _ = writeln!(out, "{pad}def {name}");
        } else {
            let _ = writeln!(out, "{pad}def {name}({})", list.join(", "));
        }
    } else {
        let _ = writeln!(out, "{pad}def {name}(");
        for (i, (a, d)) in args.iter().enumerate() {
            comment(out, indent + 2, &wrap(d, COMMENT_WIDTH));
            let comma = if i + 1 < args.len() { "," } else { "" };
            let _ = writeln!(out, "{pad}  {a}{comma}");
        }
        let _ = writeln!(out, "{pad})");
    }
    let _ = writeln!(out, "{pad}end");
}

// ---- sig/**.rbs ----

fn rbs_type(g: &Gen<'_>, f: &RField) -> String {
    let t = f.ty.rbs(&g.m);
    if f.nullable { format!("{t}?") } else { t }
}

/// The `type <name> = { ... }` alias of a class's fields.
fn rbs_record(g: &Gen<'_>, c: &Class) -> String {
    let fields: Vec<String> = c
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.name, rbs_type(g, f)))
        .collect();
    if fields.is_empty() {
        "{ }".to_owned()
    } else {
        format!("{{ {} }}", fields.join(", "))
    }
}

fn rbs_class(g: &Gen<'_>, out: &mut String, indent: usize, c: &Class) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    let alias = crate::naming::rbs_alias(c.r.name());
    if c.params {
        let _ = writeln!(
            out,
            "{pad}type {alias} = {} & {}::Internal::Type::request_parameters",
            rbs_record(g, c),
            g.m
        );
    } else {
        let _ = writeln!(out, "{pad}type {alias} = {}", rbs_record(g, c));
    }
    out.push('\n');
    let _ = writeln!(out, "{pad}class {} < {}", c.r.name(), base(g, c));
    let mut first = true;
    let mut sep = |out: &mut String| {
        if !first {
            out.push('\n');
        }
        first = false;
    };
    if c.params {
        sep(out);
        request_parameters(g, out, &inner);
    }
    for f in &c.fields {
        sep(out);
        let t = f.ty.rbs(&g.m);
        let full = rbs_type(g, f);
        if c.base.is_some() {
            let read = if f.required { full.clone() } else { format!("{t}?") };
            let _ = writeln!(out, "{inner}def {}: -> {read}", f.name);
            out.push('\n');
            let _ = writeln!(out, "{inner}def {}=: ({full} _) -> {full}", f.name);
        } else if f.required || f.nullable {
            let _ = writeln!(out, "{inner}attr_accessor {}: {full}", f.name);
        } else {
            let _ = writeln!(out, "{inner}attr_reader {}: {t}?", f.name);
            out.push('\n');
            let _ = writeln!(out, "{inner}def {}=: ({t}) -> {t}", f.name);
        }
    }
    sep(out);
    let mut args: Vec<String> = c
        .fields
        .iter()
        .map(|f| {
            let q = if f.required { "" } else { "?" };
            format!("{q}{}: {}", f.name, rbs_type(g, f))
        })
        .collect();
    let mut hash: Vec<String> = c
        .fields
        .iter()
        .map(|f| format!("{}: {}", f.name, rbs_type(g, f)))
        .collect();
    if c.params {
        args.push(format!("?request_options: {}::request_opts", g.m));
        hash.push(format!("request_options: {}::RequestOptions", g.m));
    }
    let _ = writeln!(out, "{inner}def initialize: ({}) -> void", args.join(", "));
    out.push('\n');
    let _ = writeln!(out, "{inner}def to_hash: -> {{ {} }}", hash.join(", "));
    for n in &c.nested {
        out.push('\n');
        decl(g, out, indent + 2, n, View::Rbs);
    }
    let _ = writeln!(out, "{pad}end");
}

// ---- enums and unions, every view ----

fn enum_decl(g: &Gen<'_>, out: &mut String, indent: usize, e: &EnumDecl, view: View) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    let code = e.r.code(&g.m);
    if view == View::Rbs {
        let values: Vec<String> = e.members.iter().map(|m| literal(&m.value)).collect();
        let _ = writeln!(
            out,
            "{pad}type {} = {}",
            crate::naming::rbs_alias(e.r.name()),
            values.join(" | ")
        );
        out.push('\n');
    } else {
        header(g, out, indent, &e.description, e.see.as_ref(), view);
    }
    let _ = writeln!(out, "{pad}module {}", e.r.name());
    let _ = writeln!(out, "{inner}extend {}::Internal::Type::Enum", g.m);
    out.push('\n');
    match view {
        View::Rb => {
            for m in &e.members {
                let _ = writeln!(out, "{inner}{} = {}", m.constant, literal(&m.value));
            }
            out.push('\n');
            let _ = writeln!(out, "{inner}# @!method self.values");
            let _ = writeln!(out, "{inner}#   @return [Array<Symbol>]");
        }
        View::Rbi => {
            let _ = writeln!(out, "{inner}TaggedSymbol = T.type_alias {{ T.all(Symbol, {code}) }}");
            let _ = writeln!(out, "{inner}OrSymbol = T.type_alias {{ T.any(Symbol, String) }}");
            out.push('\n');
            for m in &e.members {
                let _ = writeln!(
                    out,
                    "{inner}{} = T.let({}, {code}::TaggedSymbol)",
                    m.constant,
                    literal(&m.value)
                );
            }
            out.push('\n');
            let _ = writeln!(out, "{inner}sig {{ override.returns(T::Array[{code}::TaggedSymbol]) }}");
            let _ = writeln!(out, "{inner}def self.values");
            let _ = writeln!(out, "{inner}end");
        }
        View::Rbs => {
            for m in &e.members {
                let _ = writeln!(out, "{inner}{}: {}", m.constant, literal(&m.value));
            }
            out.push('\n');
            let _ = writeln!(out, "{inner}def self?.values: -> ::Array[{}]", e.r.rbs_alias(&g.m));
        }
    }
    let _ = writeln!(out, "{pad}end");
}

fn union_decl(g: &Gen<'_>, out: &mut String, indent: usize, u: &UnionDecl, view: View) {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    if view != View::Rbs {
        header(g, out, indent, &u.description, u.see.as_ref(), view);
    }
    let _ = writeln!(out, "{pad}module {}", u.r.name());
    let _ = writeln!(out, "{inner}extend {}::Internal::Type::Union", g.m);
    if view == View::Rb {
        for v in &u.variants {
            out.push('\n');
            let _ = writeln!(out, "{inner}variant {}", v.ty.rb_field(&g.m));
        }
    }
    for n in &u.nested {
        out.push('\n');
        decl(g, out, indent + 2, n, view);
    }
    let _ = writeln!(out, "{pad}end");
}
