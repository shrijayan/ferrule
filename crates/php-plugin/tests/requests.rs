//! How a method's parameters and the client's credentials reach the request, on small inline
//! specs the synthetic golden and imagekit do not cover. No oracle generation has these shapes, so
//! each test pins what the request must carry rather than a file byte for byte.

mod common;

use std::collections::HashMap;

use ferrule_php_plugin::{PhpMeta, PhpOutput, generate};
use ferrule_sdk_model::{FerCode, FerruleError};

/// The files generated for `paths` and `schemes` (OpenAPI YAML fragments) with the resource
/// `things` binding `methods` and the client option `api_key` authenticating with `Key`.
fn files(paths: &str, schemes: &str, methods: &str) -> HashMap<String, String> {
    generated(paths, schemes, methods)
        .expect("generates")
        .files
        .into_iter()
        .collect()
}

/// What generating [`files`]'s inputs returns.
fn generated(paths: &str, schemes: &str, methods: &str) -> Result<PhpOutput, FerruleError> {
    let openapi = format!(
        "openapi: 3.1.0\ninfo:\n  title: Acme\n  version: 1.0.0\nservers:\n  - url: https://api.acme.test\nsecurity:\n  - Key: []\npaths:\n{paths}components:\n  securitySchemes:\n    Key:\n{schemes}"
    );
    let stainless = format!(
        "organization:\n  name: acme\n  docs: https://docs.acme.test\nresources:\n  things:\n    methods:\n{methods}targets:\n  python:\n    package_name: acme\n    publish:\n      pypi: false\n  php:\n    package_name: acme\n    composer_package_name: acme/acme\nclient_settings:\n  opts:\n    api_key:\n      type: string\n      auth:\n        security_scheme: Key\n      read_env: ACME_API_KEY\nenvironments:\n  production: https://api.acme.test\nreadme:\n  example_requests:\n    default:\n      type: request\n      endpoint: get /things\n      params: {{}}\n    headline:\n      type: request\n      endpoint: get /things\n      params: {{}}\n"
    );
    let converted = ferrule_converter::convert_text(&openapi, &stainless).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    generate(&common::model(&converted.output), &PhpMeta::default())
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

/// A date-time inside a list, a map or a union is converted by `DateTimeInterface`'s class name,
/// which `Conversion::tryConvert` matches without a leading backslash.
#[test]
fn a_nested_date_time_is_converted_by_its_class_name() {
    let f = files(
        "  /things:\n    get:\n      responses:\n        '200':\n          description: Done.\n          content:\n            application/json:\n              schema:\n                type: object\n                properties:\n                  seen_at:\n                    type: array\n                    items:\n                      type: string\n                      format: date-time\n                  by_day:\n                    type: object\n                    additionalProperties:\n                      type: string\n                      format: date-time\n",
        BEARER,
        "      list: get /things\n",
    );
    let response = &f["src/Things/ThingListResponse.php"];
    assert!(response.contains("list: \\DateTimeInterface::class"), "{response}");
    assert!(response.contains("map: \\DateTimeInterface::class"), "{response}");
    assert!(!response.contains("'\\DateTimeInterface'"), "{response}");
}

/// An integer enum is `int`-backed, and its properties and parameters take its value as an
/// `int`.
#[test]
fn an_integer_enum_takes_and_stores_an_int() {
    let f = files(
        "  /things:\n    get:\n      parameters:\n        - name: level\n          in: query\n          required: true\n          schema:\n            type: integer\n            enum: [1, 2]\n      responses:\n        '200':\n          description: Done.\n          content:\n            application/json:\n              schema:\n                type: object\n                required: [priority]\n                properties:\n                  priority:\n                    type: integer\n                    enum: [1, 2, 3]\n",
        BEARER,
        "      list: get /things\n",
    );
    let service = &f["src/Services/ThingsService.php"];
    assert!(service.contains("Level|int $level"), "{service}");
    let response = &f["src/Things/ThingListResponse.php"];
    assert!(response.contains("public int $priority;"), "{response}");
    assert!(response.contains("Priority|int $priority"), "{response}");
    assert!(f["src/Things/ThingListResponse/Priority.php"].contains("enum Priority: int"));
}

/// A model whose required field lists the model itself ends its test example with an empty list
/// instead of nesting it forever.
#[test]
fn a_required_recursive_field_ends_the_test_example() {
    let f = files(
        "  /things:\n    post:\n      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n              type: object\n              required: [root]\n              properties:\n                root:\n                  $ref: '#/components/schemas/Node'\n      responses:\n        '204':\n          description: Done.\n",
        &format!(
            "{BEARER}  schemas:\n    Node:\n      type: object\n      required: [children]\n      properties:\n        children:\n          type: array\n          items:\n            $ref: '#/components/schemas/Node'\n"
        ),
        "      create: post /things\n    models:\n      node: Node\n",
    );
    let test = &f["tests/Services/ThingsTest.php"];
    assert!(test.contains("'children' => []"), "{test}");
}

/// A method with query parameters and a body sends each parameter where it belongs: the query
/// ones in the URL, the others in the body.
#[test]
fn query_and_body_parameters_are_split() {
    let f = files(
        "  /things:\n    post:\n      parameters:\n        - name: dry_run\n          in: query\n          schema:\n            type: boolean\n      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n              type: object\n              required: [password]\n              properties:\n                password:\n                  type: string\n      responses:\n        '204':\n          description: Done.\n",
        BEARER,
        "      create: post /things\n",
    );
    let raw = &f["src/Services/ThingsRawService.php"];
    assert!(raw.contains("$query_params = array_flip(['dry_run']);"), "{raw}");
    assert!(raw.contains("query: array_intersect_key($parsed, $query_params),"), "{raw}");
    assert!(raw.contains("body: (object) array_diff_key($parsed, $query_params),"), "{raw}");
}

/// A header parameter is a parameter of the method, and the raw service sends it as a header,
/// apart from the query and the body.
#[test]
fn header_parameters_are_sent_as_headers() {
    let header = "        - name: X-Trace-Id\n          in: header\n          required: true\n          schema:\n            type: string\n";
    let f = files(
        &format!(
            "  /things:\n    get:\n      parameters:\n{header}        - name: limit\n          in: query\n          schema:\n            type: integer\n      responses:\n        '204':\n          description: Done.\n    post:\n      parameters:\n{header}      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n              type: object\n              properties:\n                name:\n                  type: string\n      responses:\n        '204':\n          description: Done.\n"
        ),
        BEARER,
        "      list: get /things\n      create: post /things\n",
    );
    for params in ["ThingListParams", "ThingCreateParams"] {
        let class = &f[&format!("src/Things/{params}.php")];
        assert!(class.contains("#[Required('X-Trace-Id')]"), "{class}");
    }
    let service = &f["src/Services/ThingsService.php"];
    assert!(service.contains("string $xTraceID"), "{service}");
    let raw = &f["src/Services/ThingsRawService.php"];
    assert_eq!(raw.matches("$header_params = array_flip(['X-Trace-Id']);").count(), 2, "{raw}");
    assert_eq!(raw.matches("headers: array_intersect_key($parsed, $header_params),").count(), 2, "{raw}");
    assert!(raw.contains("query: array_intersect_key($parsed, $query_params),"), "{raw}");
    assert!(raw.contains("body: (object) array_diff_key($parsed, $header_params),"), "{raw}");
}

/// A request body that is not an object (or a union of objects) has no parameters to spread, so
/// it is reported, as the Python plugin reports it, rather than dropped from the request.
#[test]
fn a_body_that_is_not_an_object_is_reported() {
    let body = |schema: &str| {
        format!(
            "  /things:\n    post:\n      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n{schema}      responses:\n        '204':\n          description: Done.\n"
        )
    };
    for schema in [
        "              type: array\n              items:\n                type: string\n",
        "              type: string\n",
    ] {
        let Err(err) = generated(&body(schema), BEARER, "      create: post /things\n") else {
            panic!("generated a create() that drops the body:\n{schema}");
        };
        assert_eq!(err.code, FerCode::Fer201, "{err:?}");
        assert_eq!(err.pointer, "#/paths/~1things/post", "{err:?}");
    }
}
