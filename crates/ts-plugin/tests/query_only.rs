//! A method other than GET whose params are all query params (`tests/fixtures/query-only`):
//! the current layout destructures and lists them, as knock-mgmt-node's `commits.commitAll` and
//! `branches.create` do, and breaks the call as prettier does.

mod support;

use std::path::Path;

use ferrule_ts_plugin::{Layout, generate_layout};

#[test]
fn lists_the_query_of_a_put() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/query-only");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let (client, meta, extras) =
        support::load(&read("openapi.yml"), &read("stainless.yml")).expect("the fixture maps to the model");
    let files = generate_layout(&client, &meta, &extras, Layout::Typescript);
    let (_, commits) = files
        .iter()
        .find(|(p, _)| p == "src/resources/commits.ts")
        .expect("commits.ts");
    let want =
        "  commitAll(params: CommitCommitAllParams, options?: RequestOptions): APIPromise<CommitCommitAllResponse> {
    const { environment, commit_message, resource_id, resource_type } = params;
    return this._client.put('/commits', {
      query: { environment, commit_message, resource_id, resource_type },
      ...options,
    });
  }
";
    assert!(commits.contains(want), "missing:\n{want}\n--- in:\n{commits}");
}

/// With every query param optional, `params` may be `null`: destructured `params ?? {}`, as
/// papr-typescript's `memory.deleteAll` does.
#[test]
fn destructures_optional_params_that_may_be_null() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/query-only");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let spec = read("openapi.yml").replace(
        "          name: environment\n          required: true\n",
        "          name: environment\n",
    );
    assert!(!spec.contains("required: true"), "the fixture moved");
    let (client, meta, extras) = support::load(&spec, &read("stainless.yml")).expect("the fixture maps to the model");
    let files = generate_layout(&client, &meta, &extras, Layout::Typescript);
    let (_, commits) = files
        .iter()
        .find(|(p, _)| p == "src/resources/commits.ts")
        .expect("commits.ts");
    let want = "    params: CommitCommitAllParams | null | undefined = {},
    options?: RequestOptions,
  ): APIPromise<CommitCommitAllResponse> {
    const { commit_message, environment, resource_id, resource_type } = params ?? {};
";
    assert!(commits.contains(want), "missing:\n{want}\n--- in:\n{commits}");
}
