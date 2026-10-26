//! How a method's parameters and the client's credentials reach the request, on small inline
//! specs the synthetic golden and imagekit do not cover. No oracle generation has these shapes, so
//! each test pins what the request must carry rather than a file byte for byte.

mod common;

use std::collections::HashMap;

use ferrule_php_plugin::{PhpMeta, generate};

/// The files generated for `paths` and `schemes` (OpenAPI YAML fragments) with the resource
/// `things` binding `methods` and the client option `api_key` authenticating with `Key`.
fn files(paths: &str, schemes: &str, methods: &str) -> HashMap<String, String> {
    let openapi = format!(
        "openapi: 3.1.0\ninfo:\n  title: Acme\n  version: 1.0.0\nservers:\n  - url: https://api.acme.test\nsecurity:\n  - Key: []\npaths:\n{paths}components:\n  securitySchemes:\n    Key:\n{schemes}"
    );
    let stainless = format!(
        "organization:\n  name: acme\n  docs: https://docs.acme.test\nresources:\n  things:\n    methods:\n{methods}targets:\n  python:\n    package_name: acme\n    publish:\n      pypi: false\n  php:\n    package_name: acme\n    composer_package_name: acme/acme\nclient_settings:\n  opts:\n    api_key:\n      type: string\n      auth:\n        security_scheme: Key\n      read_env: ACME_API_KEY\nenvironments:\n  production: https://api.acme.test\nreadme:\n  example_requests:\n    default:\n      type: request\n      endpoint: get /things\n      params: {{}}\n    headline:\n      type: request\n      endpoint: get /things\n      params: {{}}\n"
    );
    let converted = ferrule_converter::convert_text(&openapi, &stainless).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    generate(&common::model(&converted.output), &PhpMeta::default())
        .expect("generates")
        .files
        .into_iter()
        .collect()
}

const BEARER: &str = "      type: http\n      scheme: bearer\n";

const GET_THING: &str = "  /things:\n    get:\n      responses:\n        '204':\n          description: Done.\n";

/// An API key in the query (OneBusAway's `key`) is sent as `?key=...`, not as a header.
#[test]
fn a_query_api_key_is_sent_in_the_query() {
    let f = files(
        GET_THING,
        "      type: apiKey\n      in: query\n      name: key\n",
        "      list: get /things\n",
    );
    let client = &f["src/Client.php"];
    assert!(
        client.contains("protected function authQuery(): array\n    {\n        return $this->apiKey ? ['key' => $this->apiKey] : [];\n    }\n"),
        "{client}"
    );
    assert!(client.contains("query: [...$this->authQuery(), ...$query],"), "{client}");
    assert!(!client.contains("authHeaders"), "{client}");
}

/// An API key in a header stays a header, and a bearer token too.
#[test]
fn header_credentials_are_sent_as_headers() {
    let f = files(
        GET_THING,
        "      type: apiKey\n      in: header\n      name: X-Api-Key\n",
        "      list: get /things\n",
    );
    assert!(f["src/Client.php"].contains("return ['X-Api-Key' => $this->apiKey];"));
    assert!(!f["src/Client.php"].contains("authQuery"));
    let f = files(GET_THING, BEARER, "      list: get /things\n");
    assert!(f["src/Client.php"].contains("return ['Authorization' => \"Bearer {$this->apiKey}\"];"));
}
