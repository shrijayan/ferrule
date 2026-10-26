//! What the generated SDK sends and its tests pass, on small inline specs the synthetic golden and
//! imagekit do not cover. No oracle generation has these shapes, so each test pins what the files
//! must carry rather than a file byte for byte.

mod common;

use std::collections::HashMap;

use ferrule_csharp_plugin::{CSharpMeta, CSharpOutput, generate};

/// The files generated for `paths` and `components` (OpenAPI YAML fragments, `components` holding
/// at least the security scheme `Key`) with the resource `things` binding `methods`, and the
/// client option `api_key` authenticating with `Key`.
fn files(paths: &str, components: &str, methods: &str) -> HashMap<String, String> {
    generated(paths, components, methods).files.into_iter().collect()
}

/// What generating [`files`]'s inputs returns.
fn generated(paths: &str, components: &str, methods: &str) -> CSharpOutput {
    let openapi = format!(
        "openapi: 3.1.0\ninfo:\n  title: Acme\n  version: 1.0.0\nservers:\n  - url: https://api.acme.test\nsecurity:\n  - Key: []\npaths:\n{paths}components:\n{components}"
    );
    let stainless = format!(
        "organization:\n  name: acme\n  docs: https://docs.acme.test\nresources:\n  things:\n    methods:\n{methods}targets:\n  csharp:\n    package_name: Acme\n    production_repo: acme-test/acme-dotnet\n    publish:\n      nuget: false\nclient_settings:\n  opts:\n    api_key:\n      type: string\n      auth:\n        security_scheme: Key\n      read_env: ACME_API_KEY\nenvironments:\n  production: https://api.acme.test\nreadme:\n  example_requests:\n    default:\n      type: request\n      endpoint: get /things\n      params: {{}}\n    headline:\n      type: request\n      endpoint: get /things\n      params: {{}}\n"
    );
    let converted = ferrule_converter::convert_text(&openapi, &stainless).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    generate(&common::model(&converted.output), &CSharpMeta::default()).expect("generates")
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
    assert!(
        params.contains("public required IReadOnlyDictionary<string, string> Metadata"),
        "{params}"
    );
    assert!(params.contains("public required JsonElement Payload"), "{params}");
    let test = &f["src/Acme.Tests/Services/ThingServiceTest.cs"];
    assert!(
        test.contains("Metadata = new Dictionary<string, string>() { { \"foo\", \"foo\" } }"),
        "{test}"
    );
    assert!(
        test.contains("Payload = JsonSerializer.SerializeToElement(\"bar\")"),
        "{test}"
    );
    assert!(test.contains("using System.Collections.Generic;"), "{test}");
    assert!(test.contains("using System.Text.Json;"), "{test}");
}

/// An API key in the query (OneBusAway's `key`) is added to every request's query by the runtime's
/// `auth_query` slot, not dropped; a header key stays in `auth_headers`.
#[test]
fn a_query_api_key_is_sent_in_the_query() {
    let get = "  /things:\n    get:\n      responses:\n        '204':\n          description: Done.\n";
    let out = generated(
        get,
        "  securitySchemes:\n    Key:\n      type: apiKey\n      in: query\n      name: key\n",
        "      list: get /things\n",
    );
    assert_eq!(
        out.runtime.auth_query,
        "        if (options.ApiKey != null)\n        {\n            collection.Add(\"key\", options.ApiKey);\n        }\n"
    );
    assert_eq!(out.runtime.auth_headers, "");
    assert!(out.runtime.to_json().contains("\"auth_query\""));
    let out = generated(
        get,
        "  securitySchemes:\n    Key:\n      type: apiKey\n      in: header\n      name: X-Api-Key\n",
        "      list: get /things\n",
    );
    assert_eq!(out.runtime.auth_query, "");
    assert!(
        out.runtime
            .auth_headers
            .contains("request.Headers.Add(\"X-Api-Key\", options.ApiKey);")
    );
    // The options of an SDK without a query key are as before.
    assert!(!out.runtime.to_json().contains("auth_query"));
}

/// A model no C# method reaches (only a method left out for C# returns it) is still declared, as
/// Stainless declares every model the config names, and its reference to a model left out for C#
/// is inlined as `for_target` inlines it, not a `JsonElement`.
#[test]
fn an_unreached_model_keeps_the_inlined_shape_of_a_model_left_out() {
    let f = files(
        "  /things:\n    get:\n      responses:\n        '204':\n          description: Done.\n  /holders:\n    get:\n      responses:\n        '200':\n          description: The holder.\n          content:\n            application/json:\n              schema:\n                $ref: '#/components/schemas/Holder'\n",
        &format!(
            "{BEARER}  schemas:\n    Holder:\n      type: object\n      required: [inner]\n      properties:\n        inner:\n          $ref: '#/components/schemas/Inner'\n    Inner:\n      type: object\n      properties:\n        note:\n          type: string\n"
        ),
        "      list: get /things\n      holder:\n        endpoint: get /holders\n        skip: [csharp]\n    models:\n      holder: Holder\n      inner:\n        openapi_uri: '#/components/schemas/Inner'\n        skip: [csharp]\n",
    );
    let (path, holder) = f
        .iter()
        .find(|(p, _)| p.ends_with("/Holder.cs"))
        .unwrap_or_else(|| panic!("no Holder.cs in {:?}", f.keys().collect::<Vec<_>>()));
    assert!(!holder.contains("JsonElement Inner"), "{path}:\n{holder}");
    assert!(holder.contains("Note"), "{path}:\n{holder}");
}

/// A method whose required argument the test cannot build (here a dictionary of enum values) has
/// no service test, rather than one that does not compile; the others keep theirs.
#[test]
fn a_method_whose_required_argument_cannot_be_built_has_no_test() {
    let f = files(
        "  /things:\n    get:\n      responses:\n        '204':\n          description: Done.\n    post:\n      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n              type: object\n              required: [levels]\n              properties:\n                levels:\n                  type: object\n                  additionalProperties:\n                    type: string\n                    enum: [low, high]\n      responses:\n        '204':\n          description: Done.\n",
        BEARER,
        "      create: post /things\n      list: get /things\n",
    );
    let test = &f["src/Acme.Tests/Services/ThingServiceTest.cs"];
    assert!(!test.contains("Create_Works"), "{test}");
    assert!(test.contains("List_Works"), "{test}");
}
