//! Go v2 cases no oracle SDK pins byte for byte, each on a committed fixture
//! (`tests/<fixture>`) or a reduced inline spec and stainless.yml, generated with
//! `Layout::V2`.

mod common;

use std::path::Path;

use ferrule_go_plugin::{GoMeta, GoOutput, Layout, generate};

fn convert(openapi: &str, stainless: &str) -> String {
    let converted = ferrule_converter::convert_text(openapi, stainless).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    converted.output
}

fn v2(annotated: &str, meta: GoMeta) -> GoOutput {
    let meta = GoMeta {
        layout: Some(Layout::V2),
        ..meta
    };
    generate(&common::model(annotated), &meta).expect("generates")
}

fn fixture(name: &str, meta: GoMeta) -> GoOutput {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(name);
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("fixture input");
    v2(&convert(&read("openapi.yml"), &read("stainless.yml")), meta)
}

fn file<'a>(out: &'a GoOutput, path: &str) -> &'a str {
    out.files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is generated"), |(_, t)| t.as_str())
}

/// What imagekit-go's and knock-mgmt-go's `aliases.go` and `shared/shared.go` declare after
/// their imports.
const PARAM_ALIASES: &str = "// aliased to make [param.APIUnion] private when embedding
type paramUnion = param.APIUnion

// aliased to make [param.APIObject] private when embedding
type paramObj = param.APIObject

";

#[test]
fn packages_with_params_declare_the_param_obj_alias() {
    let out = fixture("synthetic", GoMeta::default());
    let aliases = file(&out, "aliases.go");
    assert!(
        aliases.contains(&format!(
            "\t\"github.com/acme-test/acme-go/packages/param\"\n\t\"github.com/acme-test/acme-go/shared\"\n)\n\n{PARAM_ALIASES}type Error = apierror.Error\n"
        )),
        "{aliases}"
    );
    let shared = file(&out, "shared/shared.go");
    assert!(
        shared.contains(&format!(
            "\t\"github.com/acme-test/acme-go/packages/param\"\n\t\"github.com/acme-test/acme-go/packages/respjson\"\n)\n\n{PARAM_ALIASES}type ErrorInfo struct"
        )),
        "{shared}"
    );
}

/// `POST /v1/things` with a body; `components` (YAML, indented for its place) holds `Thing`
/// (the response) and whatever else the case needs.
fn spec(body: &str, components: &str) -> String {
    format!(
        "openapi: 3.1.0
info:
  title: Acme
  version: 1.0.0
servers:
  - url: https://api.acme.test
security:
  - BearerAuth: []
paths:
  /v1/things:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
{body}
      responses:
        '200':
          description: The thing.
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Thing'
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
  schemas:
    Thing:
      type: object
      properties:
        id:
          type: string
{components}"
    )
}

/// A stainless.yml whose `things` resource declares `models` (YAML, indented for its place).
fn config(models: &str) -> String {
    format!(
        "organization:
  name: acme
resources:
  things:
    models:
{models}
    methods:
      create: post /v1/things
targets:
  python:
    package_name: acme
    publish:
      pypi: false
  go:
    package_name: acme
    production_repo: acme-test/acme-go
client_settings:
  opts:
    api_key:
      type: string
      auth:
        security_scheme: BearerAuth
      read_env: ACME_API_KEY
environments:
  production: https://api.acme.test
readme:
  example_requests:
    default:
      type: request
      endpoint: post /v1/things
      params: {{}}
    headline:
      type: request
      endpoint: post /v1/things
      params: {{}}
"
    )
}

#[test]
fn a_named_enum_only_requests_use_is_declared() {
    let body = "              type: object
              properties:
                color:
                  $ref: '#/components/schemas/Color'";
    let components = "    Color:
      type: string
      enum: [red, green]";
    let out = v2(
        &convert(
            &spec(body, components),
            &config("      thing: Thing\n      color: Color"),
        ),
        GoMeta::default(),
    );
    let text = file(&out, "thing.go");
    assert!(text.contains("\ntype Color string\n"), "{text}");
    assert!(text.contains("\tColorRed   Color = \"red\"\n"), "{text}");
}

#[test]
fn a_root_enum_named_file_renames_the_runtime_file_helper() {
    let body = "              type: object
              properties:
                name:
                  type: string";
    let components = "        kind:
          $ref: '#/components/schemas/File'
    File:
      type: string
      enum: [image, video]";
    let out = v2(
        &convert(&spec(body, components), &config("      thing: Thing\n      file: File")),
        GoMeta::default(),
    );
    assert!(file(&out, "thing.go").contains("\ntype File string\n"));
    assert!(
        out.runtime.new_file,
        "field.go's File helper would collide with the File enum"
    );
}
