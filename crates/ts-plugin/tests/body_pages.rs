//! Page schemes no oracle pins yet (`tests/fixtures/body-pages`): `cursor_id` and `offset` with
//! `param_location: body`, and `cursor_url`, in the current layout.

mod support;

use std::path::Path;

use ferrule_ts_plugin::{Layout, generate_layout};

fn files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/body-pages");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let (client, meta, extras) =
        support::load(&read("openapi.yml"), &read("stainless.yml")).expect("the fixture maps to the model");
    generate_layout(&client, &meta, &extras, Layout::Typescript)
}

fn file<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is not generated"), |(_, t)| t.as_str())
}

#[track_caller]
fn has(text: &str, snippet: &str) {
    assert!(text.contains(snippet), "missing:\n{snippet}\n--- in:\n{text}");
}

#[test]
fn body_schemes_read_the_previous_request_from_the_body() {
    let f = files();
    let pagination = file(&f, "src/core/pagination.ts");
    has(
        pagination,
        "    if ((this.options.body as Record<string, unknown>)?.['before']) {\n",
    );
    has(
        pagination,
        "    const offset = Number((this.options.body as Record<string, unknown>)?.['offset'] ?? 0);\n",
    );
    assert!(!pagination.contains("this.options.query"), "{pagination}");
}

/// `buildURL` takes an absolute path as is, so the next URL keeps its origin, and a base URL
/// ending in `/v1` is not prefixed to a path that already starts with `/v1`.
#[test]
fn cursor_url_requests_the_whole_next_url() {
    let f = files();
    has(
        file(&f, "src/core/pagination.ts"),
        "    const next = new URL(url);
    return {
      ...this.options,
      path: next.toString(),
      query: Object.fromEntries(next.searchParams),
    };
",
    );
}
