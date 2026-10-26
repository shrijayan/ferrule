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
