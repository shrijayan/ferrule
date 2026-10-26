//! What the generated SDK sends and its tests pass, on small inline specs the synthetic golden and
//! imagekit do not cover. No oracle generation has these shapes, so each test pins what the files
//! must carry rather than a file byte for byte.

mod common;

use std::collections::HashMap;

use ferrule_csharp_plugin::{CSharpMeta, generate};

/// The files generated for `paths` and `components` (OpenAPI YAML fragments, `components` holding
/// at least the security scheme `Key`) with the resource `things` binding `methods`, and the
/// client option `api_key` authenticating with `Key`.
fn files(paths: &str, components: &str, methods: &str) -> HashMap<String, String> {
    let openapi = format!(
        "openapi: 3.1.0\ninfo:\n  title: Acme\n  version: 1.0.0\nservers:\n  - url: https://api.acme.test\nsecurity:\n  - Key: []\npaths:\n{paths}components:\n{components}"
    );
    let stainless = format!(
        "organization:\n  name: acme\n  docs: https://docs.acme.test\nresources:\n  things:\n    methods:\n{methods}targets:\n  csharp:\n    package_name: Acme\n    production_repo: acme-test/acme-dotnet\n    publish:\n      nuget: false\nclient_settings:\n  opts:\n    api_key:\n      type: string\n      auth:\n        security_scheme: Key\n      read_env: ACME_API_KEY\nenvironments:\n  production: https://api.acme.test\nreadme:\n  example_requests:\n    default:\n      type: request\n      endpoint: get /things\n      params: {{}}\n    headline:\n      type: request\n      endpoint: get /things\n      params: {{}}\n"
    );
    let converted = ferrule_converter::convert_text(&openapi, &stainless).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    generate(&common::model(&converted.output), &CSharpMeta::default())
        .expect("generates")
        .files
        .into_iter()
        .collect()
}

const BEARER: &str = "  securitySchemes:\n    Key:\n      type: http\n      scheme: bearer\n";

/// A required dictionary or JSON property gets a value in the service test, which would not
/// compile without it (CS9035: a required member is not set).
#[test]
fn required_dictionary_and_json_properties_get_test_values() {
    let f = files(
        "  /things:\n    post:\n      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n              type: object\n              required: [metadata, payload]\n              properties:\n                metadata:\n                  type: object\n                  additionalProperties:\n                    type: string\n                payload: {}\n      responses:\n        '204':\n          description: Done.\n",
        BEARER,
        "      create: post /things\n",
    );
    let params = &f["src/Acme/Models/Things/ThingCreateParams.cs"];
    assert!(params.contains("public required IReadOnlyDictionary<string, string> Metadata"), "{params}");
    assert!(params.contains("public required JsonElement Payload"), "{params}");
    let test = &f["src/Acme.Tests/Services/ThingServiceTest.cs"];
    assert!(
        test.contains("Metadata = new Dictionary<string, string>() { { \"foo\", \"foo\" } }"),
        "{test}"
    );
    assert!(test.contains("Payload = JsonSerializer.SerializeToElement(\"bar\")"), "{test}");
    assert!(test.contains("using System.Collections.Generic;"), "{test}");
    assert!(test.contains("using System.Text.Json;"), "{test}");
}
