//! The resource files, one pass with a [`View`] flag: `lib/**/resources/*.rb` (the methods and
//! their YARD tags), `rbi/**/resources/*.rbi` and `sig/**/resources/*.rbs`. A subresource's class
//! nests in its parent's (`class Files` > `class Versions`), and a parent holds an attribute per
//! subresource.

use std::fmt::Write as _;

use super::model::{def_new, param_tags, short, truncated};
use super::{Gen, Meth, Res, View};
use crate::doc::{Doc, call, concat, print_rb, text};
use crate::printer::{COMMENT_WIDTH, comment, quote, wrap};
use crate::tree::RField;
use crate::types::RType;

/// A method's arguments as Ruby takes them: the last path parameter positional, the other path
/// parameters and every other parameter as keywords.
struct Args<'c> {
    /// The last path parameter and its description.
    positional: Option<(&'c RField, &'c str)>,
    /// The other path parameters, in path order, with their descriptions.
    path: Vec<(&'c RField, &'c str)>,
    /// The query, header and body parameters.
    rest: &'c [RField],
}

impl<'c> Args<'c> {
    fn of(m: &'c Meth<'_>) -> Self {
        let mut path: Vec<(&RField, &str)> = m.args.path.iter().map(|f| (f, f.description.as_str())).collect();
        let positional = path.pop();
        Self {
            positional,
            path,
            rest: &m.args.rest,
        }
    }

    /// Every keyword argument with its description: the other path parameters, then the rest.
    fn keywords(&self) -> Vec<(&'c RField, &'c str)> {
        let mut out = self.path.clone();
        out.extend(self.rest.iter().map(|f| (f, f.description.as_str())));
        out
    }

    /// Whether the method needs a params hash with required keys (no `= {}` default).
    fn required(&self) -> bool {
        !self.path.is_empty() || self.rest.iter().any(RField::must_pass)
    }

    /// Whether the method parses its params (`dump_request`).
    fn parsed(&self) -> bool {
        !self.path.is_empty() || !self.rest.is_empty()
    }
}

/// `name:` for a required keyword, `name: nil` for an optional one.
fn keyword(f: &RField) -> String {
    f.init_arg()
}

/// The method's doc text: its description, else its summary.
fn doc_text(m: &Meth<'_>) -> String {
    if m.method.description.trim().is_empty() {
        m.method.summary.clone()
    } else {
        m.method.description.clone()
    }
}

/// `"api/where/stop/%1$s.json"` with its path parameters, or the bare string.
fn path_arg(m: &Meth<'_>) -> String {
    let mut template = m.method.path.trim_start_matches('/').to_owned();
    for (i, p) in m.method.path_params.iter().enumerate() {
        template = template.replace(&format!("{{{}}}", p.field.wire_name), &format!("%{}$s", i + 1));
    }
    if m.method.path_params.is_empty() {
        quote(&template)
    } else {
        let names: Vec<&str> = m.args.path.iter().map(|f| f.name.as_str()).collect();
        format!("[{}, {}]", quote(&template), names.join(", "))
    }
}

/// The YARD block of a method: its description, `@overload`, `@param`, `@return` and `@see`.
fn rb_method_doc(g: &Gen<'_>, out: &mut String, indent: usize, m: &Meth<'_>, args: &Args<'_>) {
    let pad = " ".repeat(indent);
    let keywords = args.keywords();
    let any_truncated = args.positional.iter().chain(&keywords).any(|(_, d)| truncated(d));
    let mut lines = Vec::new();
    if any_truncated {
        lines.extend(wrap(
            &format!(
                "Some parameter documentations has been truncated, see {{{}}} for more details.",
                m.params.doc(&g.m)
            ),
            COMMENT_WIDTH,
        ));
        lines.push(String::new());
    }
    lines.extend(wrap(&doc_text(m), COMMENT_WIDTH));
    if !lines.is_empty() {
        lines.push(String::new());
    }
    let mut overload: Vec<String> = args.positional.iter().map(|(f, _)| f.name.clone()).collect();
    overload.extend(keywords.iter().map(|(f, _)| keyword(f)));
    overload.push("request_options: {}".to_owned());
    lines.push(format!("@overload {}({})", m.name, overload.join(", ")));
    lines.push(String::new());
    comment(out, indent, &lines);
    let mut tags: Vec<(String, String, String)> = args
        .positional
        .iter()
        .chain(&keywords)
        .map(|(f, d)| {
            let t = f.ty.yard(&g.m);
            let t = if f.nullable { format!("{t}, nil") } else { t };
            (f.name.clone(), t, short(d))
        })
        .collect();
    tags.push((
        "request_options".to_owned(),
        format!("{}::RequestOptions, Hash{{Symbol=>Object}}, nil", g.m),
        String::new(),
    ));
    param_tags(out, &pad, "", &tags);
    let returns = m.response.as_ref().map_or_else(|| "nil".to_owned(), |t| t.yard(&g.m));
    let _ = writeln!(out, "{pad}#\n{pad}# @return [{returns}]\n{pad}#");
    let _ = writeln!(out, "{pad}# @see {}", m.params.doc(&g.m));
}

/// Whether the method sends a multipart body.
fn multipart(m: &Meth<'_>) -> bool {
    m.method
        .body
        .as_ref()
        .is_some_and(|b| b.media_type == "multipart/form-data")
}

fn rb_method(g: &Gen<'_>, out: &mut String, indent: usize, m: &Meth<'_>) {
    let pad = " ".repeat(indent);
    let body = " ".repeat(indent + 2);
    let args = Args::of(m);
    rb_method_doc(g, out, indent, m, &args);
    let mut sig: Vec<String> = args.positional.iter().map(|(f, _)| f.name.clone()).collect();
    sig.push(if args.required() { "params" } else { "params = {}" }.to_owned());
    let _ = writeln!(out, "{pad}def {}({})", m.name, sig.join(", "));
    let mut request: Vec<Doc> = vec![
        text(format!("method: :{}", m.method.http_method.as_str())),
        text(format!("path: {}", path_arg(m))),
    ];
    let options = if args.parsed() {
        let _ = writeln!(
            out,
            "{body}parsed, options = {}.dump_request(params)",
            m.params.code(&g.m)
        );
        for (f, _) in &args.path {
            let _ = writeln!(
                out,
                "{body}{name} =\n{body}  parsed.delete(:{name}) do\n{body}    raise ArgumentError.new(\"missing required path argument #{{_1}}\")\n{body}  end",
                name = f.name
            );
        }
        let query: Vec<&RField> = args
            .rest
            .iter()
            .filter(|f| m.method.query_params.iter().any(|q| q.field.wire_name == f.wire))
            .collect();
        if !query.is_empty() {
            let _ = writeln!(out, "{body}query = {}::Internal::Util.encode_query_params(parsed)", g.m);
            let renamed: Vec<Doc> = query
                .iter()
                .filter_map(|f| f.api_name().map(|w| text(format!("{}: {}", f.name, quote(w)))))
                .collect();
            if renamed.is_empty() {
                request.push(text("query: query"));
            } else {
                request.push(concat(vec![text("query: "), call("query.transform_keys", renamed)]));
            }
        }
        if multipart(m) {
            request.push(text("headers: {\"content-type\" => \"multipart/form-data\"}"));
        }
        match &m.args.body {
            crate::tree::Body::Fields => request.push(text("body: parsed")),
            crate::tree::Body::Param(name) => request.push(text(format!("body: parsed[:{name}]"))),
            crate::tree::Body::None => {}
        }
        "options"
    } else {
        "params[:request_options]"
    };
    let model = m
        .response
        .as_ref()
        .map_or_else(|| "NilClass".to_owned(), |t| t.rb_inner(&g.m));
    request.push(text(format!("model: {model}")));
    request.push(text(format!("options: {options}")));
    let _ = writeln!(out, "{body}{}", print_rb(&call("@client.request", request), indent + 2));
    let _ = writeln!(out, "{pad}end");
}

/// What the method returns, as Sorbet names it.
fn rbi_returns(g: &Gen<'_>, m: &Meth<'_>) -> String {
    match &m.response {
        Some(RType::Model(r)) if !r.aliased => r.doc(&g.m),
        Some(t) => t.rbi_returned(&g.m),
        None => "NilClass".to_owned(),
    }
}

fn rbi_method(g: &Gen<'_>, out: &mut String, indent: usize, m: &Meth<'_>) {
    let pad = " ".repeat(indent);
    let args = Args::of(m);
    comment(out, indent, &wrap(&doc_text(m), COMMENT_WIDTH));
    let keywords = args.keywords();
    let mut types: Vec<String> = args
        .positional
        .iter()
        .chain(&keywords)
        .map(|(f, _)| {
            let t = f.ty.rbi_input(&g.m);
            let t = if f.nullable { format!("T.nilable({t})") } else { t };
            format!("{}: {t}", f.name)
        })
        .collect();
    types.push(format!("request_options: {}::RequestOptions::OrHash", g.m));
    match &m.response {
        Some(_) => {
            let _ = writeln!(
                out,
                "{pad}sig {{ params({}).returns({}) }}",
                types.join(", "),
                rbi_returns(g, m)
            );
        }
        None => {
            let _ = writeln!(out, "{pad}sig {{ params({}).void }}", types.join(", "));
        }
    }
    let mut defs: Vec<(String, &str)> = args.positional.iter().map(|(f, d)| (f.name.clone(), *d)).collect();
    defs.extend(keywords.iter().map(|(f, d)| (keyword(f), *d)));
    defs.push(("request_options: {}".to_owned(), ""));
    def_new(out, indent, &m.name, &defs);
}

fn rbs_method(g: &Gen<'_>, out: &mut String, indent: usize, m: &Meth<'_>) {
    let args = Args::of(m);
    let rbs = |f: &RField| {
        let ty = f.ty.rbs(&g.m);
        if f.nullable { format!("{ty}?") } else { ty }
    };
    let mut list: Vec<String> = args
        .positional
        .iter()
        .map(|(f, _)| format!("{} {}", rbs(f), f.name))
        .collect();
    list.extend(args.keywords().iter().map(|(f, _)| {
        let optional = if f.must_pass() { "" } else { "?" };
        format!("{optional}{}: {}", f.name, rbs(f))
    }));
    list.push(format!("?request_options: {}::request_opts", g.m));
    let returns = match &m.response {
        Some(RType::Model(r)) if !r.aliased => r.doc(&g.m),
        Some(t) => t.rbs(&g.m),
        None => "nil".to_owned(),
    };
    let _ = writeln!(
        out,
        "{}def {}: ({}) -> {returns}",
        " ".repeat(indent),
        m.name,
        list.join(", ")
    );
}

/// The resource's subresources with a class.
fn children<'g>(g: &'g Gen<'_>, res: &Res<'_>) -> Vec<&'g Res<'g>> {
    g.resources
        .iter()
        .filter(|r| r.class.len() == res.class.len() + 1 && r.class.starts_with(&res.class))
        .collect()
}

/// One resource file in one view.
#[must_use]
pub fn resource_file(g: &Gen<'_>, res: &Res<'_>, view: View) -> (String, String) {
    let mut out = String::new();
    match view {
        View::Rb => out.push_str("# frozen_string_literal: true\n\n"),
        View::Rbi => out.push_str("# typed: strong\n\n"),
        View::Rbs => {}
    }
    let _ = writeln!(out, "module {}\n  module Resources", g.m);
    for (i, c) in res.class.iter().enumerate() {
        let _ = writeln!(out, "{}class {c}", " ".repeat(4 + 2 * i));
    }
    let indent = 4 + 2 * res.class.len();
    let pad = " ".repeat(indent);
    let mut items: Vec<String> = Vec::new();
    let subs = children(g, res);
    for s in &subs {
        let path = s.path(&g.m);
        items.push(match view {
            View::Rb => format!("{pad}# @return [{path}]\n{pad}attr_reader :{}\n", s.attr),
            View::Rbi => format!("{pad}sig {{ returns({path}) }}\n{pad}attr_reader :{}\n", s.attr),
            View::Rbs => format!("{pad}attr_reader {}: {path}\n", s.attr),
        });
    }
    for m in &res.methods {
        let mut text = String::new();
        match view {
            View::Rb => rb_method(g, &mut text, indent, m),
            View::Rbi => rbi_method(g, &mut text, indent, m),
            View::Rbs => rbs_method(g, &mut text, indent, m),
        }
        items.push(text);
    }
    let m = &g.m;
    items.push(match view {
        View::Rb => {
            let mut t = format!(
                "{pad}# @api private\n{pad}#\n{pad}# @param client [{m}::Client]\n{pad}def initialize(client:)\n{pad}  @client = client\n"
            );
            for s in &subs {
                let _ = writeln!(t, "{pad}  @{} = {}.new(client: client)", s.attr, s.path(m));
            }
            let _ = writeln!(t, "{pad}end");
            t
        }
        View::Rbi => format!(
            "{pad}# @api private\n{pad}sig {{ params(client: {m}::Client).returns(T.attached_class) }}\n{pad}def self.new(client:)\n{pad}end\n"
        ),
        View::Rbs => format!("{pad}def initialize: (client: {m}::Client) -> void\n"),
    });
    out.push_str(&items.join("\n"));
    for i in (0..res.class.len()).rev() {
        let _ = writeln!(out, "{}end", " ".repeat(4 + 2 * i));
    }
    out.push_str("  end\nend\n");
    (format!("{}/resources/{}.{}", g.lib(view), res.file, view.ext()), out)
}
