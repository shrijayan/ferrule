//! The gem's index files: `lib/<gem>.rb` (every `require` in load order), `models.rb` (the
//! top-level aliases of params and named models, and the Sorbet constants) with its views, and
//! `version.rb` with its views.

use std::fmt::Write as _;

use super::{Gen, View};
use crate::doc::{assign, print, text};
use crate::printer::{RB_WIDTH, len, quote};
use crate::tree::Decl;
use crate::types::ClassRef;

/// The standard libraries and the `tapioca` guard at the top of `lib/<gem>.rb`.
const LIB_HEADER: &str = include_str!("lib_header.rb.txt");

/// The Sorbet constants `models.rb` defines before its aliases (`__M__` is the module).
const MODELS_HEADER: &str = include_str!("models_header.rb.txt");

/// The runtime's files in the order `lib/<gem>.rb` requires them, before the client.
const RUNTIME_REQUIRES: [&str; 19] = [
    "version",
    "internal/util",
    "internal/type/converter",
    "internal/type/unknown",
    "internal/type/boolean",
    "internal/type/file_input",
    "internal/type/enum",
    "internal/type/union",
    "internal/type/array_of",
    "internal/type/hash_of",
    "internal/type/base_model",
    "internal/type/base_page",
    "internal/type/request_parameters",
    "internal",
    "request_options",
    "file_part",
    "errors",
    "internal/transport/base_client",
    "internal/transport/pooled_net_requester",
];

/// The model files in load order: a model before the ones that extend it, else by path.
fn model_order<'g>(g: &'g Gen<'_>) -> Vec<&'g str> {
    let bases: Vec<&ClassRef> = g
        .models
        .iter()
        .filter_map(|f| match &f.decl {
            Decl::Class(c) => c.base.as_ref(),
            _ => None,
        })
        .collect();
    let (first, rest): (Vec<_>, Vec<_>) = g.models.iter().partition(|f| bases.contains(&f.decl.r()));
    first.iter().chain(&rest).map(|f| f.path.as_str()).collect()
}

/// `lib/<gem>.rb`.
#[must_use]
pub fn lib_file(g: &Gen<'_>) -> (String, String) {
    let lib = &g.target.lib;
    let mut out = String::from(crate::printer::template(LIB_HEADER));
    out.push_str("\n# Gems.\nrequire \"connection_pool\"\n\n# Package files.\n");
    let mut require = |p: &str| {
        let _ = writeln!(out, "require_relative \"{lib}/{p}\"");
    };
    for p in RUNTIME_REQUIRES {
        require(p);
    }
    require("client");
    for p in model_order(g) {
        require(&format!("models/{p}"));
    }
    require("models");
    let mut resources: Vec<&str> = g.resources.iter().map(|r| r.file.as_str()).collect();
    resources.sort_by(|a, b| ferrule_sdk_model::js::cmp(a, b));
    for r in resources {
        require(&format!("resources/{r}"));
    }
    (format!("lib/{lib}.rb"), out)
}

/// What a top-level alias names.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Class,
    /// An enum, union or namespace.
    Module,
    /// A converter constant (`Labels = HashOf[...]`), typed as a converter in each view.
    Converter,
}

/// The top-level aliases: params, named models and the namespaces of nested models, sorted by
/// name ignoring case.
fn aliases(g: &Gen<'_>) -> Vec<(String, Kind)> {
    let mut out: Vec<(String, Kind)> = Vec::new();
    // A converter constant's item models (`LabelsItem` before `Labels`) are top-level too.
    let before = |d: &'_ Decl| match d {
        Decl::Alias(a) => a.before.clone(),
        _ => Vec::new(),
    };
    for decl in g
        .models
        .iter()
        .flat_map(|f| before(&f.decl).into_iter().chain([f.decl.clone()]))
    {
        let r = decl.r();
        if !r.aliased {
            continue;
        }
        let top = r.segs[0].clone();
        let kind = match decl {
            _ if r.segs.len() > 1 => Kind::Module,
            Decl::Class(_) => Kind::Class,
            Decl::Alias(_) => Kind::Converter,
            Decl::Enum(_) | Decl::Union(_) => Kind::Module,
        };
        if !out.iter().any(|(n, _)| *n == top) {
            out.push((top, kind));
        }
    }
    out.sort_by(|a, b| {
        ferrule_sdk_model::js::cmp(&a.0.to_lowercase(), &b.0.to_lowercase())
            .then_with(|| ferrule_sdk_model::js::cmp(&a.0, &b.0))
    });
    out
}

fn models_header(g: &Gen<'_>) -> String {
    let m = &g.m;
    let one = format!("  [{m}::Internal::Type::BaseModel, *{m}::Internal::Type::BaseModel.subclasses].each do |cls|");
    let base_models = if len(&one) <= RB_WIDTH {
        format!("{one}\n")
    } else {
        format!(
            "  [\n    {m}::Internal::Type::BaseModel,\n    *{m}::Internal::Type::BaseModel.subclasses\n  ].each do |cls|\n"
        )
    };
    let align = " ".repeat(2 + len(&format!("{m}::Internal::Util")));
    crate::printer::template(MODELS_HEADER)
        .replace("__BASE_MODELS__", &base_models)
        .replace("__ALIGN__", &align)
        .replace("__M__", m)
}

/// `models.rb` and its views.
#[must_use]
pub fn models_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let m = &g.m;
    let aliases = aliases(g);
    let mut rb = models_header(g);
    let mut rbi = format!("# typed: strong\n\nmodule {m}\n");
    let mut rbs = format!("module {m}\n");
    let converter = format!("{m}::Internal::Type::Converter");
    for (i, (name, kind)) in aliases.iter().enumerate() {
        let value = format!("{m}::Models::{name}");
        let doc = assign(name, text(value.clone()));
        rb.push('\n');
        if *kind == Kind::Converter {
            let _ = writeln!(rb, "  # @type [{converter}]");
        }
        let _ = writeln!(rb, "  {}", print(&doc, RB_WIDTH, 2));
        if i > 0 {
            rbi.push('\n');
            rbs.push('\n');
        }
        match kind {
            Kind::Converter => {
                let _ = writeln!(rbi, "  {name} = T.let({value}, {converter})");
                let _ = writeln!(rbs, "  {name}: {converter}");
            }
            Kind::Class | Kind::Module => {
                let _ = writeln!(rbi, "  {name} = {value}");
                let word = if *kind == Kind::Module { "module" } else { "class" };
                let _ = writeln!(rbs, "  {word} {name} = {value}");
            }
        }
    }
    rb.push_str("end\n");
    rbi.push_str("end\n");
    rbs.push_str("end\n");
    vec![
        (format!("{}/models.rb", g.lib(View::Rb)), rb),
        (format!("{}/models.rbi", g.lib(View::Rbi)), rbi),
        (format!("{}/models.rbs", g.lib(View::Rbs)), rbs),
    ]
}

/// `version.rb` and its views.
#[must_use]
pub fn version_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let m = &g.m;
    vec![
        (
            format!("{}/version.rb", g.lib(View::Rb)),
            format!(
                "# frozen_string_literal: true\n\nmodule {m}\n  VERSION = {}\nend\n",
                quote(&g.target.version)
            ),
        ),
        (
            format!("{}/version.rbi", g.lib(View::Rbi)),
            format!("# typed: strong\n\nmodule {m}\n  VERSION = T.let(T.unsafe(nil), String)\nend\n"),
        ),
        (
            format!("{}/version.rbs", g.lib(View::Rbs)),
            format!("module {m}\n  VERSION: String\nend\n"),
        ),
    ]
}
