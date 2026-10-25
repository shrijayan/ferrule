//! The resource files, one pass with a [`View`] flag: `lib/**/resources/*.rb` (the methods and
//! their YARD tags), `rbi/**/resources/*.rbi` and `sig/**/resources/*.rbs`.

use std::fmt::Write as _;

use super::model::{collapse, def_new, param_tags, short};
use super::{Gen, Meth, Res, View};
use crate::doc::{Doc, call, concat, print_rb, text};
use crate::printer::{COMMENT_WIDTH, comment, quote, wrap};
use crate::tree::{Class, Decl, RField};
use crate::types::RType;

/// The params class of `m` (always declared, see [`Gen::model_files`]).
fn params<'g>(g: &'g Gen<'_>, m: &Meth<'_>) -> &'g Class {
    g.models
        .iter()
        .find_map(|f| match &f.decl {
            Decl::Class(c) if c.r == m.params => Some(c),
            _ => None,
        })
        .expect("every method has a params class")
}

/// The path parameters (positional) and the rest of the params class.
fn split<'c>(m: &Meth<'_>, c: &'c Class) -> (&'c [RField], &'c [RField]) {
    c.fields.split_at(m.method.path_params.len())
}

/// The method's doc text: its description, else its summary.
fn doc_text(m: &Meth<'_>) -> String {
    if m.method.description.trim().is_empty() {
        m.method.summary.clone()
    } else {
        m.method.description.clone()
    }
}

fn response_yard(g: &Gen<'_>, m: &Meth<'_>) -> String {
    m.response.as_ref().map_or_else(|| "nil".to_owned(), |t| t.yard(&g.m))
}

/// `"api/where/stop/%1$s.json"` with its path parameters, or the bare string.
fn path_arg(m: &Meth<'_>, path_params: &[RField]) -> String {
    let mut template = m.method.path.trim_start_matches('/').to_owned();
    for (i, p) in m.method.path_params.iter().enumerate() {
        template = template.replace(&format!("{{{}}}", p.field.wire_name), &format!("%{}$s", i + 1));
    }
    if path_params.is_empty() {
        quote(&template)
    } else {
        let names: Vec<&str> = path_params.iter().map(|f| f.name.as_str()).collect();
        format!("[{}, {}]", quote(&template), names.join(", "))
    }
}

/// The YARD block of a method: its description, `@overload`, `@param`, `@return` and `@see`.
fn rb_method_doc(g: &Gen<'_>, out: &mut String, m: &Meth<'_>, path: &[RField], rest: &[RField]) {
    let pad = "      ";
    let path_docs: Vec<&str> = m
        .method
        .path_params
        .iter()
        .map(|p| p.field.description.as_str())
        .collect();
    let truncated = path_docs
        .iter()
        .copied()
        .chain(rest.iter().map(|f| f.description.as_str()))
        .any(|d| short(d) != collapse(d));
    let mut lines = Vec::new();
    if truncated {
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
    let mut overload: Vec<String> = path.iter().map(|f| f.name.clone()).collect();
    overload.extend(rest.iter().map(|f| {
        if f.required {
            format!("{}:", f.name)
        } else {
            format!("{}: nil", f.name)
        }
    }));
    overload.push("request_options: {}".to_owned());
    lines.push(format!("@overload {}({})", m.name, overload.join(", ")));
    lines.push(String::new());
    comment(out, 6, &lines);
    let path_docs = m.method.path_params.iter().map(|p| p.field.description.as_str());
    let mut tags: Vec<(String, String, String)> = path
        .iter()
        .zip(path_docs)
        .map(|(f, d)| (f.name.clone(), f.ty.yard(&g.m), short(d)))
        .collect();
    tags.extend(rest.iter().map(|f| {
        let t = f.ty.yard(&g.m);
        let t = if f.nullable { format!("{t}, nil") } else { t };
        (f.name.clone(), t, short(&f.description))
    }));
    tags.push((
        "request_options".to_owned(),
        format!("{}::RequestOptions, Hash{{Symbol=>Object}}, nil", g.m),
        String::new(),
    ));
    param_tags(out, pad, "", &tags);
    let _ = writeln!(out, "{pad}#");
    let _ = writeln!(out, "{pad}# @return [{}]", response_yard(g, m));
    let _ = writeln!(out, "{pad}#");
    let _ = writeln!(out, "{pad}# @see {}", m.params.doc(&g.m));
}

fn rb_method(g: &Gen<'_>, out: &mut String, m: &Meth<'_>) {
    let pad = "      ";
    let c = params(g, m);
    let (path, rest) = split(m, c);
    rb_method_doc(g, out, m, path, rest);
    let required = rest.iter().any(|f| f.required);
    let mut sig: Vec<String> = path.iter().map(|f| f.name.clone()).collect();
    sig.push(if required {
        "params".to_owned()
    } else {
        "params = {}".to_owned()
    });
    let _ = writeln!(out, "{pad}def {}({})", m.name, sig.join(", "));
    let body = "        ";
    let mut args: Vec<Doc> = vec![
        text(format!("method: :{}", m.method.http_method.as_str())),
        text(format!("path: {}", path_arg(m, path))),
    ];
    let options = if rest.is_empty() {
        "params[:request_options]"
    } else {
        let _ = writeln!(
            out,
            "{body}parsed, options = {}.dump_request(params)",
            m.params.code(&g.m)
        );
        let query: Vec<&RField> = rest
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
                args.push(text("query: query"));
            } else {
                args.push(concat(vec![text("query: "), call("query.transform_keys", renamed)]));
            }
        }
        "options"
    };
    if let Some(t) = &m.response {
        args.push(text(format!("model: {}", t.rb_inner(&g.m))));
    }
    args.push(text(format!("options: {options}")));
    let _ = writeln!(out, "{body}{}", print_rb(&call("@client.request", args), 8));
    let _ = writeln!(out, "{pad}end");
}

fn rbi_method(g: &Gen<'_>, out: &mut String, m: &Meth<'_>) {
    let pad = "      ";
    let c = params(g, m);
    let (path, rest) = split(m, c);
    comment(out, 6, &wrap(&doc_text(m), COMMENT_WIDTH));
    let mut types: Vec<String> = path
        .iter()
        .map(|f| format!("{}: {}", f.name, f.ty.rbi_input(&g.m)))
        .collect();
    types.extend(rest.iter().map(|f| {
        let t = f.ty.rbi_input(&g.m);
        let t = if f.nullable { format!("T.nilable({t})") } else { t };
        format!("{}: {t}", f.name)
    }));
    types.push(format!("request_options: {}::RequestOptions::OrHash", g.m));
    let returns = match &m.response {
        Some(RType::Model(r)) if !r.aliased => r.doc(&g.m),
        Some(t) => t.rbi(&g.m),
        None => "NilClass".to_owned(),
    };
    let _ = writeln!(out, "{pad}sig {{ params({}).returns({returns}) }}", types.join(", "));
    let path_docs = m.method.path_params.iter().map(|p| p.field.description.as_str());
    let mut args: Vec<(String, &str)> = path.iter().zip(path_docs).map(|(f, d)| (f.name.clone(), d)).collect();
    args.extend(rest.iter().map(|f| {
        let a = if f.required {
            format!("{}:", f.name)
        } else {
            format!("{}: nil", f.name)
        };
        (a, f.description.as_str())
    }));
    args.push(("request_options: {}".to_owned(), ""));
    def_new(out, 6, &m.name, &args);
}

fn rbs_method(g: &Gen<'_>, out: &mut String, m: &Meth<'_>) {
    let c = params(g, m);
    let (path, rest) = split(m, c);
    let mut args: Vec<String> = path.iter().map(|f| format!("{} {}", f.ty.rbs(&g.m), f.name)).collect();
    args.extend(rest.iter().map(|f| {
        let ty = f.ty.rbs(&g.m);
        let ty = if f.nullable { format!("{ty}?") } else { ty };
        let optional = if f.required { "" } else { "?" };
        format!("{optional}{}: {ty}", f.name)
    }));
    args.push(format!("?request_options: {}::request_opts", g.m));
    let returns = match &m.response {
        Some(RType::Model(r)) if !r.aliased => r.doc(&g.m),
        Some(t) => t.rbs(&g.m),
        None => "nil".to_owned(),
    };
    let _ = writeln!(out, "      def {}: ({}) -> {returns}", m.name, args.join(", "));
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
    let _ = writeln!(out, "module {}", g.m);
    let _ = writeln!(out, "  module Resources");
    let _ = writeln!(out, "    class {}", res.class.join("::"));
    let mut first = true;
    for m in &res.methods {
        if !first {
            out.push('\n');
        }
        first = false;
        match view {
            View::Rb => rb_method(g, &mut out, m),
            View::Rbi => rbi_method(g, &mut out, m),
            View::Rbs => rbs_method(g, &mut out, m),
        }
    }
    if !first {
        out.push('\n');
    }
    match view {
        View::Rb => {
            let _ = writeln!(out, "      # @api private");
            let _ = writeln!(out, "      #");
            let _ = writeln!(out, "      # @param client [{}::Client]", g.m);
            let _ = writeln!(out, "      def initialize(client:)");
            let _ = writeln!(out, "        @client = client");
            let _ = writeln!(out, "      end");
        }
        View::Rbi => {
            let _ = writeln!(out, "      # @api private");
            let _ = writeln!(
                out,
                "      sig {{ params(client: {}::Client).returns(T.attached_class) }}",
                g.m
            );
            let _ = writeln!(out, "      def self.new(client:)");
            let _ = writeln!(out, "      end");
        }
        View::Rbs => {
            let _ = writeln!(out, "      def initialize: (client: {}::Client) -> void", g.m);
        }
    }
    out.push_str("    end\n  end\nend\n");
    (format!("{}/resources/{}.{}", g.lib(view), res.file, view.ext()), out)
}
