//! Edge cases no oracle SDK covers (client settings, auth schemes, names that meet generated
//! members), each from a reduced inline spec and stainless.yml.

mod common;

use ferrule_go_plugin::{GoMeta, GoOutput, generate};

/// One `GET /v1/thing` returning `response` (a YAML schema, indented for its place).
fn spec(response: &str, security_schemes: &str) -> String {
    format!(
        "openapi: 3.1.0
info:
  title: Acme
  version: 1.0.0
servers:
  - url: https://api.acme.test
paths:
  /v1/thing:
    get:
      responses:
        '200':
          description: The thing.
          content:
            application/json:
              schema:
{response}
components:
  securitySchemes:
{security_schemes}"
    )
}

const BEARER: &str = "    BearerAuth:
      type: http
      scheme: bearer";

/// A stainless.yml with `client_settings` (YAML, indented for its place).
fn config(client_settings: &str) -> String {
    format!(
        "organization:
  name: acme
resources:
  thing:
    methods:
      retrieve: get /v1/thing
targets:
  python:
    package_name: acme
    publish:
      pypi: false
  go:
    package_name: acme
    production_repo: acme-test/acme-go
client_settings:
{client_settings}
environments:
  production: https://api.acme.test
readme:
  example_requests:
    default:
      type: request
      endpoint: get /v1/thing
      params: {{}}
    headline:
      type: request
      endpoint: get /v1/thing
      params: {{}}
"
    )
}

const API_KEY: &str = "  opts:
    api_key:
      type: string
      auth:
        security_scheme: BearerAuth
      read_env: ACME_API_KEY";

fn go(openapi: &str, stainless: &str) -> GoOutput {
    let converted = ferrule_converter::convert_text(openapi, stainless).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    let annotated = converted.output;
    generate(&common::model(&annotated), &GoMeta::default()).expect("generates")
}

fn file<'a>(out: &'a GoOutput, path: &str) -> &'a str {
    out.files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is generated"), |(_, t)| t.as_str())
}

#[test]
fn a_property_named_json_does_not_collide_with_the_json_metadata_field() {
    let response = "                type: object
                properties:
                  json:
                    type: string";
    let out = go(&spec(response, BEARER), &config(API_KEY));
    let text = file(&out, "thing.go");
    // ogx-go and knock-mgmt-go spell `json` as `Json` (`JsonSchema`, `JsonBody`), which leaves
    // `JSON` to the metadata field.
    assert!(text.contains("\tJson string               `json:\"json\"`\n"), "{text}");
    assert!(text.contains("\tJson        apijson.Field\n"), "{text}");
    assert!(text.contains("\tJSON thingGetResponseJSON `json:\"-\"`\n"), "{text}");
}

const OBJECT: &str = "                type: object
                properties:
                  name:
                    type: string";

#[test]
fn non_string_client_settings_are_sent_as_strings() {
    let settings = format!(
        "{API_KEY}
    org_id:
      type: integer
      send_in_header: X-Org
    ratio:
      type: number
      send_as_query_param: ratio
    beta:
      type: boolean
      send_in_header: X-Beta
    region:
      type: string
      send_in_header: X-Region"
    );
    let out = go(&spec(OBJECT, BEARER), &config(&settings));
    let options = &out.runtime.client_options;
    assert!(
        options.contains("r.Apply(WithHeader(\"X-Org\", fmt.Sprint(*r.OrgID)))"),
        "{options}"
    );
    assert!(
        options.contains("r.Apply(WithQuery(\"ratio\", fmt.Sprint(*r.Ratio)))"),
        "{options}"
    );
    assert!(
        options.contains("r.Apply(WithHeader(\"X-Beta\", fmt.Sprint(*r.Beta)))"),
        "{options}"
    );
    // A string option is sent as it is.
    assert!(
        options.contains("r.Apply(WithHeader(\"X-Region\", *r.Region))"),
        "{options}"
    );
}

#[test]
fn a_bearer_scheme_keyword_replaces_bearer() {
    let schemes = "    BearerAuth:
      type: http
      scheme: bearer
      x-stainless-auth:
        scheme_keyword: Token";
    let out = go(&spec(OBJECT, schemes), &config(API_KEY));
    let options = &out.runtime.client_options;
    assert!(
        options.contains("WithHeader(\"authorization\", fmt.Sprintf(\"Token %s\", *r.APIKey))"),
        "{options}"
    );
}

fn api_key_scheme(location: &str) -> String {
    format!(
        "    BearerAuth:
      type: apiKey
      in: {location}
      name: key"
    )
}

#[test]
fn an_api_key_scheme_is_sent_where_it_says() {
    let out = go(&spec(OBJECT, &api_key_scheme("query")), &config(API_KEY));
    let options = &out.runtime.client_options;
    assert!(options.contains("r.Apply(WithQuery(\"key\", *r.APIKey))"), "{options}");
    let out = go(&spec(OBJECT, &api_key_scheme("header")), &config(API_KEY));
    let options = &out.runtime.client_options;
    assert!(options.contains("r.Apply(WithHeader(\"key\", *r.APIKey))"), "{options}");
}

#[test]
fn an_api_key_in_a_cookie_is_reported() {
    let converted =
        ferrule_converter::convert_text(&spec(OBJECT, &api_key_scheme("cookie")), &config(API_KEY)).expect("converts");
    let Err(err) = generate(&common::model(&converted.output), &GoMeta::default()) else {
        panic!("an API key in a cookie generates");
    };
    assert_eq!(err.code, ferrule_sdk_model::FerCode::Fer201, "{err}");
    assert_eq!(err.pointer, "#/components/securitySchemes/BearerAuth", "{err}");
}

#[test]
fn default_client_options_carry_configured_defaults() {
    let settings = format!(
        "{API_KEY}
    region:
      type: string
      default: us
      read_env: ACME_REGION
      send_in_header: X-Region
    page_size:
      type: integer
      default: 5
      send_as_query_param: page_size
  default_headers:
    X-Api-Version: '2024-01-01'"
    );
    let out = go(&spec(OBJECT, BEARER), &config(&settings));
    let client = file(&out, "client.go");
    // As imagekit-go writes its password default: after the base URL, before the environment.
    let expected = "\tif o, ok := os.LookupEnv(\"ACME_BASE_URL\"); ok {
\t\tdefaults = append(defaults, option.WithBaseURL(o))
\t}
\tdefaults = append(defaults, option.WithRegion(\"us\"))
\tdefaults = append(defaults, option.WithPageSize(5))
\tdefaults = append(defaults, option.WithHeader(\"X-Api-Version\", \"2024-01-01\"))
\tif o, ok := os.LookupEnv(\"ACME_API_KEY\"); ok {
\t\tdefaults = append(defaults, option.WithAPIKey(o))
\t}
\tif o, ok := os.LookupEnv(\"ACME_REGION\"); ok {
\t\tdefaults = append(defaults, option.WithRegion(o))
\t}
";
    assert!(client.contains(expected), "{client}");
}
