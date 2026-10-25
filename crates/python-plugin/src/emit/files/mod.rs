//! The non-code files of a generated SDK: README.md, api.md, .stats.yml, project and CI files,
//! and the test suite. Pure: input in, path -> text out. Port of
//! `python-plugin/src/emit/files/index.ts`.

pub mod api_md;
pub mod api_tests;
pub mod input;
mod js;
pub mod render;
pub mod templates;

use std::collections::BTreeMap;

pub use input::ProjectFilesInput;
use input::{ExampleRequest, FilesResource};
use render::{TemplateVars, render};

use crate::model::{ArrayFormat, NestedFormat};

fn count_methods(resources: &[FilesResource]) -> usize {
    resources
        .iter()
        .map(|r| r.methods.len() + count_methods(&r.subresources))
        .sum()
}

/// The text of `.stats.yml`.
#[must_use]
pub fn emit_stats(input: &ProjectFilesInput) -> String {
    format!(
        "configured_endpoints: {}\nopenapi_spec_url: {}\nopenapi_spec_hash: {}\nconfig_hash: {}\n",
        count_methods(&input.resources),
        input.stats.openapi_spec_url,
        input.stats.openapi_spec_hash,
        input.stats.config_hash,
    )
}

fn result_var(e: &ExampleRequest) -> &str {
    e.accessor.rsplit('.').next().unwrap_or(&e.accessor)
}

fn array_format(f: ArrayFormat) -> &'static str {
    match f {
        ArrayFormat::Comma => "comma",
        ArrayFormat::Repeat => "repeat",
        ArrayFormat::Indices => "indices",
        ArrayFormat::Brackets => "brackets",
    }
}

fn object_format(f: NestedFormat) -> &'static str {
    match f {
        NestedFormat::Brackets => "brackets",
        NestedFormat::Dots => "dots",
    }
}

/// `/^https?:\/\//` and then `/\/$/` removed, once each.
fn docs_label(url: &str) -> &str {
    let url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    url.strip_suffix('/').unwrap_or(url)
}

/// The value of every `%%TOKEN%%` the templates use.
#[must_use]
pub fn template_vars(input: &ProjectFilesInput) -> TemplateVars {
    let ex = &input.readme_example;
    let probe = &input.retry_probe;
    let org = &input.organization;
    let probe_args_comma = if probe.args.is_empty() {
        String::new()
    } else {
        format!("{}, ", probe.args)
    };
    TemplateVars::from([
        ("PACKAGE", input.package_name.clone()),
        ("DIST", input.distribution_name.clone()),
        ("CLIENT", input.client_class.clone()),
        ("ENV_PREFIX", input.env_prefix.clone()),
        ("REPO", input.repo.clone()),
        ("VERSION", input.version.clone()),
        ("YEAR", input.copyright_year.to_string()),
        ("ORG_NAME", org.name.clone()),
        ("ORG_TITLE", org.title.clone()),
        ("CONTACT", org.contact.clone()),
        ("DOCS_URL", org.docs_url.clone()),
        ("DOCS_LABEL", docs_label(&org.docs_url).to_owned()),
        ("README_TITLE", input.readme_title.clone()),
        ("API_KEY_ENV", input.auth.env.clone()),
        ("AUTH_QUERY", input.auth.param.clone()),
        ("ARRAY_FORMAT", array_format(input.query.array_format).to_owned()),
        ("OBJECT_FORMAT", object_format(input.query.object_format).to_owned()),
        ("EX_ACCESSOR", ex.accessor.clone()),
        ("EX_METHOD", ex.method.clone()),
        ("EX_ARGS", ex.args.clone()),
        ("EX_RESULT", result_var(ex).to_owned()),
        ("PROBE_VERB", probe.http_method.as_str().to_owned()),
        ("PROBE_PATH", probe.path.clone()),
        ("PROBE_ACCESSOR", probe.accessor.clone()),
        ("PROBE_METHOD", probe.method.clone()),
        ("PROBE_ARGS", probe.args.clone()),
        ("PROBE_ARGS_COMMA", probe_args_comma),
    ])
}

/// Every project file, keyed by its path in the SDK repository, sorted by path (JavaScript string
/// order: UTF-16 code units).
///
/// # Panics
///
/// If a bundled template uses a token [`template_vars`] has no value for, or marks a line that is
/// not a one-line import for sorting: a bug in the templates, which the tests rule out.
#[must_use]
pub fn emit_project_files(input: &ProjectFilesInput) -> Vec<(String, String)> {
    let vars = template_vars(input);
    // A later file with the same path replaces an earlier one, as `Map.set` does.
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for (path, template) in templates::TEMPLATES {
        let text = render(template, &vars).unwrap_or_else(|e| panic!("bundled template {path} does not render: {e}"));
        files.insert((*path).to_owned(), text);
    }
    files.insert("api.md".to_owned(), api_md::emit_api_md(input));
    files.insert(".stats.yml".to_owned(), emit_stats(input));
    files.extend(api_tests::emit_api_tests(input));
    let mut files: Vec<(String, String)> = files.into_iter().collect();
    files.sort_by(|(a, _), (b, _)| js::cmp(a, b));
    files
}
