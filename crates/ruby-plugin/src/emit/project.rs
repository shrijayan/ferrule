//! The gem's project files that code generation owns: `.rubocop.yml` (which `rake format:rb`
//! formats `lib/**.rb` with, so it is part of the output's layout) and `examples/.keep`.

use super::Gen;

/// OneBusAway ruby-sdk's `.rubocop.yml` (`__M__` is the module).
const RUBOCOP: &str = include_str!("rubocop.yml.txt");

fn keep(purpose: &str) -> String {
    format!(
        "File generated from our OpenAPI spec by Ferrule.\n\nThis directory can be used to store {purpose}.\n\
         It is ignored by Ferrule code generation and its content (other than this keep file) won't be touched."
    )
}

/// `.rubocop.yml` and `examples/.keep`.
#[must_use]
pub fn project_files(g: &Gen<'_>) -> Vec<(String, String)> {
    vec![
        (
            ".rubocop.yml".to_owned(),
            crate::printer::template(RUBOCOP).replace("__M__", &g.m),
        ),
        (
            "examples/.keep".to_owned(),
            keep("example files demonstrating usage of this SDK"),
        ),
    ]
}
