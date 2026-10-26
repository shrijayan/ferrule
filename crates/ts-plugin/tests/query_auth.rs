//! A nullable API key sent in the query, on the petstore fixture in the `node` layout: without a
//! key the client sends the default query alone, as `authHeaders` sends no header.

mod support;

use std::path::Path;

use ferrule_ts_plugin::{Layout, generate_layout};

#[test]
fn a_missing_nullable_query_key_is_left_out_of_the_query() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/petstore");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let spec = read("openapi.yml").replace(
        "    BearerAuth:\n      type: http\n      scheme: bearer\n",
        "    BearerAuth:\n      type: apiKey\n      in: query\n      name: key\n",
    );
    let config = read("stainless.yml").replace(
        "      nullable: false\n",
        "      nullable: true\n      send_as_query_param: key\n",
    );
    assert!(
        spec.contains("in: query") && config.contains("nullable: true"),
        "the fixture moved"
    );
    let (client, meta, extras) = support::load(&spec, &config).expect("the fixture maps to the model");
    let files = generate_layout(&client, &meta, &extras, Layout::Node);
    let index = files
        .iter()
        .find(|(p, _)| p == "src/index.ts")
        .map(|(_, t)| t.as_str())
        .expect("src/index.ts");
    let want = "  protected override defaultQuery(): Core.DefaultQuery | undefined {
    if (this.apiKey == null) {
      return this._options.defaultQuery;
    }
    return {
      key: this.apiKey,
      ...this._options.defaultQuery,
    };
  }
";
    assert!(index.contains(want), "{index}");
}
