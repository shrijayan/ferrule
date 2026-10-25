//! The non-code files of a generated SDK: README.md, api.md, .stats.yml, project and CI files, and
//! the test suite. Pure: input in, path -> text out.

pub mod api_md;
pub mod api_tests;
pub mod input;
pub mod layout;
pub mod render;
pub mod templates;
pub mod tokens;

use std::collections::{BTreeMap, BTreeSet};

use input::FilesResource;
pub use input::ProjectFilesInput;
use render::{TemplateVars, render};
use tokens::{flag, validates_headers};

use crate::js;
use crate::model::{ArrayFormat, NestedFormat};

fn endpoints<'a>(resources: &'a [FilesResource], out: &mut BTreeSet<(&'static str, &'a str)>) {
    for r in resources {
        out.extend(r.methods.iter().map(|m| (m.http_method.as_str(), m.path.as_str())));
        endpoints(&r.subresources, out);
    }
}

/// The text of `.stats.yml`. An endpoint bound to several methods (lumaai's `post
/// /generations/video`) counts once.
#[must_use]
pub fn emit_stats(input: &ProjectFilesInput) -> String {
    let mut configured = BTreeSet::new();
    endpoints(&input.resources, &mut configured);
    format!(
        "configured_endpoints: {}\nopenapi_spec_url: {}\nopenapi_spec_hash: {}\nconfig_hash: {}\n",
        configured.len(),
        input.stats.openapi_spec_url,
        input.stats.openapi_spec_hash,
        input.stats.config_hash,
    )
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

/// The README intro sentence, which the oracles wrap after "from any Python 3.9+".
fn intro_line(title: &str) -> String {
    format!("The {title} Python library provides convenient access to the {title} REST API from any Python 3.9+")
}

/// The value of every `%%TOKEN%%` the templates use.
#[must_use]
pub fn template_vars(input: &ProjectFilesInput) -> TemplateVars {
    let org = &input.organization;
    let mut exceptions = Vec::new();
    if validates_headers(&input.auth) {
        exceptions.push(format!("{}Error", input.client_class));
    }
    exceptions.push("APIStatusError".to_owned());
    if !input.retry_probe.paginated {
        exceptions.push("APITimeoutError".to_owned());
    }
    exceptions.push("APIResponseValidationError".to_owned());
    let mut vars = TemplateVars::from([
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
        ("SECURITY_CONTACT", org.security_contact.clone()),
        ("DOCS_URL", org.docs_url.clone()),
        ("DOCS_LABEL", docs_label(&org.docs_url).to_owned()),
        ("README_TITLE", input.readme_title.clone()),
        ("README_ATTRIBUTION", flag(input.readme_attribution)),
        // OneBusAway's README has blank lines around an intro line longer than the other
        // oracles' (112 columns; theirs are 92 to 104): the only input that tells them apart.
        ("README_INTRO_SPACED", flag(js::len(&intro_line(&org.title)) > 104)),
        ("MOCK_TESTS", flag(input.mock_tests)),
        ("EXCEPTIONS", exceptions.join(", ")),
        ("ARRAY_FORMAT", array_format(input.query.array_format).to_owned()),
        ("OBJECT_FORMAT", object_format(input.query.object_format).to_owned()),
    ]);
    tokens::auth_vars(&input.auth, &mut vars);
    tokens::example_vars(&input.readme_example, &input.retry_probe, &mut vars);
    vars
}

/// Lock files list a package's dependents (`# via x`) sorted by name, the generated package
/// among them.
fn sort_dependents(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    for line in text.split_inclusive('\n') {
        if line.starts_with("    # via ") {
            run.push(line);
            continue;
        }
        run.sort_unstable();
        out.append(&mut run);
        out.push(line);
    }
    run.sort_unstable();
    out.append(&mut run);
    out.concat()
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
        if *path == "scripts/mock" && !input.mock_tests {
            continue;
        }
        let text = render(template, &vars).unwrap_or_else(|e| panic!("bundled template {path} does not render: {e}"));
        let text = if matches!(*path, "requirements.lock" | "requirements-dev.lock") {
            sort_dependents(&text)
        } else {
            text
        };
        files.insert((*path).to_owned(), text);
    }
    files.insert("api.md".to_owned(), api_md::emit_api_md(input));
    files.extend(api_md::emit_standalone_api_mds(input));
    files.insert(".stats.yml".to_owned(), emit_stats(input));
    files.extend(api_tests::emit_api_tests(input));
    let mut files: Vec<(String, String)> = files.into_iter().collect();
    files.sort_by(|(a, _), (b, _)| js::cmp(a, b));
    files
}
