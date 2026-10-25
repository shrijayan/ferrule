//! The gem's project files: the gemspec, Gemfile, Rakefile and Steepfile, `.rubocop.yml` (which
//! `rake format:rb` formats `lib/**.rb` with, so it is part of the output's layout), the
//! scripts, the tool configurations and `examples/.keep`. All but the `.keep` file are OneBusAway
//! ruby-sdk's, with the gem's names (see NOTICE).

use ferrule_sdk_model::model::{ArrayFormat, NestedFormat};

use super::Gen;
use crate::naming::{Casings, class_name, split_words};
use crate::printer::template;

/// Templates by published path; `__gem__`, `__gem_name__`, `__M__` and the gemspec's tokens are
/// replaced.
const TEMPLATES: [(&str, &str); 18] = [
    (".rubocop.yml", include_str!("rubocop.yml.txt")),
    ("Gemfile", include_str!("project/Gemfile.txt")),
    ("Rakefile", include_str!("project/Rakefile.txt")),
    ("Steepfile", include_str!("project/Steepfile.txt")),
    (".ruby-version", include_str!("project/ruby-version.txt")),
    (".yardopts", include_str!("project/yardopts.txt")),
    (".solargraph.yml", include_str!("project/solargraph.yml.txt")),
    (".gitignore", include_str!("project/gitignore.txt")),
    ("sorbet/config", include_str!("project/sorbet-config.txt")),
    (
        "sorbet/rbi/.gitignore",
        include_str!("project/sorbet-rbi-gitignore.txt"),
    ),
    ("manifest.yaml", include_str!("project/manifest.yaml.txt")),
    ("scripts/bootstrap", include_str!("project/scripts-bootstrap.txt")),
    ("scripts/format", include_str!("project/scripts-format.txt")),
    ("scripts/lint", include_str!("project/scripts-lint.txt")),
    ("scripts/test", include_str!("project/scripts-test.txt")),
    ("scripts/mock", include_str!("project/scripts-mock.txt")),
    ("scripts/fast-format", include_str!("project/scripts-fast-format.txt")),
    ("__gem__.gemspec", include_str!("project/gemspec.txt")),
];

fn keep(purpose: &str) -> String {
    format!(
        "File generated from our OpenAPI spec by Ferrule.\n\nThis directory can be used to store {purpose}.\n\
         It is ignored by Ferrule code generation and its content (other than this keep file) won't be touched."
    )
}

/// The API's name as the gemspec writes it: `onebusaway-sdk` -> `Onebusaway SDK`; a name
/// without separators stays as written (`ImageKit`).
#[must_use]
pub fn title(g: &Gen<'_>) -> String {
    let name = g
        .client
        .settings
        .as_ref()
        .map_or_else(|| g.target.gem_name.clone(), |s| s.name.written.clone());
    if name.contains(['-', '_', ' ']) {
        split_words(&name)
            .iter()
            .map(|w| class_name(&[w], Casings(&g.client.casings)))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        let mut chars = name.chars();
        chars
            .next()
            .map_or_else(String::new, |c| c.to_ascii_uppercase().to_string() + chars.as_str())
    }
}

/// The project files.
#[must_use]
pub fn project_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let settings = g.client.settings.as_ref();
    let array = match settings.map(|s| s.array_format) {
        Some(ArrayFormat::Repeat) => "repeat",
        Some(ArrayFormat::Indices) => "indices",
        Some(ArrayFormat::Brackets) => "brackets",
        _ => "comma",
    };
    let nested = match settings.map(|s| s.nested_format) {
        Some(NestedFormat::Dots) => "dots",
        _ => "brackets",
    };
    let title = title(g);
    let fill = |text: &str| {
        template(text)
            .replace("__gem_name__", &g.target.gem_name)
            .replace("__gem__", &g.target.lib)
            .replace("__M__", &g.m)
            .replace("__ARRAY__", array)
            .replace("__NESTED__", nested)
            .replace("__TITLE__", &title)
            .replace("__EMAIL__", &g.target.email)
            .replace("__REPO__", &g.target.repo)
    };
    let mut out: Vec<(String, String)> = TEMPLATES
        .iter()
        .map(|(path, text)| (path.replace("__gem__", &g.target.lib), fill(text)))
        .collect();
    out.push((
        "examples/.keep".to_owned(),
        keep("example files demonstrating usage of this SDK"),
    ));
    out
}
