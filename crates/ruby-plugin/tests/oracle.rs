//! The generated gem against real Stainless Ruby SDKs, byte for byte after `syntax_tree` and rubocop lay out
//! the `.rbi` and `.rbs` views (as the gem's `rake format` does) and the branding of the `.keep`
//! files. Skipped when `oracles/cache` is not fetched; the `.rbi` and `.rbs` files are compared
//! only when `stree` is available (`$FERRULE_STREE`, see `tests/common`).
//!
//! onebusaway-ruby (Apache-2.0) is the same generation as the proven Python pin: the md5 of its
//! `openapi.yml` and `stainless-config.yml` equal the `.stats.yml` hashes of `ruby-sdk` at
//! `46a574dd`.

mod common;

use std::path::PathBuf;

use ferrule_ruby_plugin::{RubyMeta, RubyOutput, generate};

/// Oracle files that differ on purpose or for a recorded gap, with the reason. The test fails
/// when one of them starts matching, so the list stays true.
const ONEBUSAWAY_KNOWN: &[(&str, &str)] = &[
    (
        "test/onebusaway_sdk/resources/arrival_and_departure_test.rb",
        "Forge gap G18: the path param's `example` (1_75403) is not in the model",
    ),
    (
        "test/onebusaway_sdk/resources/routes_for_agency_test.rb",
        "Forge gap G18: the path param's `example` (40) is not in the model",
    ),
    (
        "test/onebusaway_sdk/resources/schedule_for_route_test.rb",
        "Forge gap G18: the path param's `example` (1_100223) is not in the model",
    ),
];

/// Oracle files that are not generated output: release-please's, hand-written, or the runtime
/// template's (`runtime/ruby`, proven by `runtime/tests/test_render_oracles_ruby.py`).
fn not_generated(path: &str, lib: &str) -> bool {
    const RUNTIME: [&str; 4] = ["errors", "file_part", "internal", "request_options"];
    let runtime = ["lib", "rbi", "sig"].iter().any(|v| {
        let dir = format!("{v}/{lib}/");
        path.strip_prefix(&dir).is_some_and(|rest| {
            rest.starts_with("internal/") || RUNTIME.iter().any(|r| rest.split('.').next() == Some(r))
        })
    });
    let runtime_tests =
        path.starts_with(&format!("test/{lib}/internal/")) || path == format!("test/{lib}/file_part_test.rb");
    runtime || runtime_tests || path == "CHANGELOG.md"
}

/// Project files the plugin does not emit yet: the README and the docs, CI, release-please's
/// files, `bin/`, and what bundler and the generator record (`Gemfile.lock`, `.stats.yml`).
fn project_file(path: &str) -> bool {
    const NOT_YET: [&str; 11] = [
        "README.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "LICENSE",
        "Gemfile.lock",
        ".stats.yml",
        ".release-please-manifest.json",
        "release-please-config.json",
        "bin/check-release-environment",
        "bin/publish-gem",
        "scripts/utils/upload-artifact.sh",
    ];
    NOT_YET.contains(&path) || path.starts_with(".github/")
}

fn onebusaway() -> Option<(PathBuf, RubyOutput)> {
    let oracle = common::oracle("onebusaway-ruby")?;
    let annotated = common::annotated("onebusaway-config", "openapi.yml", "stainless-config.yml")?;
    let meta = RubyMeta {
        version: Some("1.12.0".into()),
        example: Some((vec!["current_time".into()], "retrieve".into())),
        email: Some("info@onebusaway.org".into()),
        // The gem name comes from `targets.ruby.gem_name`.
        ..RubyMeta::default()
    };
    let out = generate(&common::model(&annotated), &meta).expect("generates");
    Some((oracle, out))
}

/// Compares the formatted `files` with `oracle`; returns (identical, differing).
fn compare(oracle: &std::path::Path, files: &[(String, String)], known: &[(&str, &str)]) -> (usize, Vec<String>) {
    common::dump("FERRULE_RUBY_DUMP", files);
    let mut identical = 0;
    let mut differing = Vec::new();
    for (path, text) in files {
        let Ok(raw) = std::fs::read_to_string(oracle.join(path)) else {
            if !known.iter().any(|(p, _)| p == path) {
                differing.push(format!("{path} (not in the oracle)"));
            }
            continue;
        };
        let expected = common::brand(&raw);
        let is_known = known.iter().find(|(p, _)| p == path);
        match (text == &expected, is_known) {
            (true, None) => identical += 1,
            (true, Some((p, _))) => panic!("{p} is a known difference but now matches; drop it"),
            (false, Some(_)) => {}
            (false, None) => differing.push(path.clone()),
        }
    }
    (identical, differing)
}

#[test]
fn onebusaway_matches_the_oracle() {
    let Some((oracle, out)) = onebusaway() else { return };
    let Some(files) = common::format(&out.files) else {
        eprintln!("skipped: the gem's formatters are not installed (FERRULE_STREE, FERRULE_RUBOCOP)");
        return;
    };
    let (identical, differing) = compare(&oracle, &files, ONEBUSAWAY_KNOWN);
    eprintln!("onebusaway-ruby: {identical}/{} generated files identical", files.len());
    assert!(differing.is_empty(), "differ from the oracle: {differing:?}");
    assert_eq!(identical + ONEBUSAWAY_KNOWN.len(), files.len());
}

#[test]
fn onebusaway_emits_every_generated_file() {
    let Some((oracle, out)) = onebusaway() else { return };
    let lib = &out.runtime.gem_name;
    let missing: Vec<String> = common::walk(&oracle)
        .into_iter()
        .filter(|p| !not_generated(p, lib) && !project_file(p))
        .filter(|p| !out.files.iter().any(|(f, _)| f == p))
        .collect();
    assert!(missing.is_empty(), "not emitted: {missing:?}");
}

#[test]
fn onebusaway_fills_the_runtime_options() {
    let Some((_, out)) = onebusaway() else { return };
    let r = &out.runtime;
    assert_eq!(
        (
            r.gem_name.as_str(),
            r.module_name.as_str(),
            r.array_format.as_str(),
            r.auth_in.as_str()
        ),
        ("onebusaway_sdk", "OnebusawaySDK", "repeat", "query")
    );
    assert!(r.enum_example.is_empty() && r.union_example.is_empty());
    assert!(
        r.base_model_example
            .contains("`references` is a `OnebusawaySDK::References`")
    );
}

/// imagekit-ruby files that differ, with the main reason (most are gaps of the model, listed in the
/// reasons below; the test fails when one of them starts matching).
const IMAGEKIT_KNOWN: &[(&str, &str)] = &[
    (".gitignore", OLDER_PROJECT),
    ("Rakefile", OLDER_PROJECT),
    ("scripts/test", OLDER_PROJECT),
    ("scripts/mock", OLDER_PROJECT),
    ("imagekitio.gemspec", WEBHOOKS),
    ("lib/imagekitio/client.rb", CUSTOM),
    ("lib/imagekitio/models/accounts/origin_delete_params.rb", PATHDOC),
    ("lib/imagekitio/models/accounts/origin_get_params.rb", PATHDOC),
    ("lib/imagekitio/models/accounts/origin_update_params.rb", PATHDOC),
    ("lib/imagekitio/models/accounts/url_endpoint_delete_params.rb", PATHDOC),
    ("lib/imagekitio/models/accounts/url_endpoint_get_params.rb", PATHDOC),
    ("lib/imagekitio/models/accounts/url_endpoint_update_params.rb", PATHDOC),
    ("lib/imagekitio/models/asset_list_response.rb", MAPPING),
    ("lib/imagekitio/models/beta/v2/file_upload_params.rb", CUSTOM),
    ("lib/imagekitio/models/custom_metadata_field.rb", VARIANT_NAME),
    (
        "lib/imagekitio/models/custom_metadata_field_create_params.rb",
        VARIANT_NAME,
    ),
    (
        "lib/imagekitio/models/custom_metadata_field_update_params.rb",
        VARIANT_NAME,
    ),
    ("lib/imagekitio/models/extension_config.rb", VARIANT_NAME),
    ("lib/imagekitio/models/extensions.rb", EXT_ITEM),
    ("lib/imagekitio/models/file_upload_params.rb", CUSTOM),
    ("lib/imagekitio/models/folder.rb", TITLE),
    ("lib/imagekitio/models/selected_fields_schema.rb", VARIANT_NAME),
    (
        "lib/imagekitio/models/solid_color_overlay_transformation.rb",
        VARIANT_ORDER,
    ),
    ("lib/imagekitio/models/text_overlay_transformation.rb", VARIANT_ORDER),
    ("lib/imagekitio/models/transformation.rb", VARIANT_ORDER),
    ("lib/imagekitio/models/update_file_request.rb", VARIANT_NAME),
    ("lib/imagekitio/models.rb", WEBHOOKS),
    ("lib/imagekitio/resources/assets.rb", MAPPING),
    ("lib/imagekitio/resources/beta/v2/files.rb", HARDBREAK),
    ("lib/imagekitio/resources/files.rb", VARIANT_NAME),
    ("lib/imagekitio.rb", WEBHOOKS),
    ("rbi/imagekitio/client.rbi", CUSTOM),
    ("rbi/imagekitio/models/accounts/origin_delete_params.rbi", PATHDOC),
    ("rbi/imagekitio/models/accounts/origin_get_params.rbi", PATHDOC),
    ("rbi/imagekitio/models/accounts/origin_update_params.rbi", PATHDOC),
    ("rbi/imagekitio/models/accounts/url_endpoint_delete_params.rbi", PATHDOC),
    ("rbi/imagekitio/models/accounts/url_endpoint_get_params.rbi", PATHDOC),
    ("rbi/imagekitio/models/accounts/url_endpoint_update_params.rbi", PATHDOC),
    ("rbi/imagekitio/models/asset_list_response.rbi", MAPPING),
    ("rbi/imagekitio/models/beta/v2/file_upload_params.rbi", TOKEN),
    ("rbi/imagekitio/models/custom_metadata_field.rbi", VARIANT_NAME),
    (
        "rbi/imagekitio/models/custom_metadata_field_create_params.rbi",
        VARIANT_NAME,
    ),
    (
        "rbi/imagekitio/models/custom_metadata_field_update_params.rbi",
        VARIANT_NAME,
    ),
    ("rbi/imagekitio/models/extension_config.rbi", VARIANT_NAME),
    ("rbi/imagekitio/models/extensions.rbi", EXT_ITEM),
    ("rbi/imagekitio/models/file_upload_params.rbi", TOKEN),
    ("rbi/imagekitio/models/folder.rbi", TITLE),
    ("rbi/imagekitio/models/image_overlay.rbi", RECURSIVE),
    ("rbi/imagekitio/models/selected_fields_schema.rbi", VARIANT_NAME),
    (
        "rbi/imagekitio/models/solid_color_overlay_transformation.rbi",
        VARIANT_ORDER,
    ),
    ("rbi/imagekitio/models/text_overlay_transformation.rbi", VARIANT_ORDER),
    ("rbi/imagekitio/models/transformation.rbi", VARIANT_ORDER),
    ("rbi/imagekitio/models/update_file_request.rbi", VARIANT_NAME),
    ("rbi/imagekitio/models/video_overlay.rbi", RECURSIVE),
    ("rbi/imagekitio/models.rbi", WEBHOOKS),
    ("rbi/imagekitio/resources/beta/v2/files.rbi", HARDBREAK),
    ("rbi/imagekitio/resources/files.rbi", VARIANT_NAME),
    ("sig/imagekitio/client.rbs", CUSTOM),
    ("sig/imagekitio/models/asset_list_response.rbs", MAPPING),
    ("sig/imagekitio/models/beta/v2/file_upload_params.rbs", TOKEN),
    ("sig/imagekitio/models/custom_metadata_field.rbs", VARIANT_NAME),
    (
        "sig/imagekitio/models/custom_metadata_field_create_params.rbs",
        VARIANT_NAME,
    ),
    (
        "sig/imagekitio/models/custom_metadata_field_update_params.rbs",
        VARIANT_NAME,
    ),
    ("sig/imagekitio/models/extension_config.rbs", VARIANT_NAME),
    ("sig/imagekitio/models/extensions.rbs", EXT_ITEM),
    ("sig/imagekitio/models/file_upload_params.rbs", TOKEN),
    ("sig/imagekitio/models/selected_fields_schema.rbs", VARIANT_NAME),
    (
        "sig/imagekitio/models/solid_color_overlay_transformation.rbs",
        VARIANT_ORDER,
    ),
    ("sig/imagekitio/models/text_overlay_transformation.rbs", VARIANT_ORDER),
    ("sig/imagekitio/models/transformation.rbs", VARIANT_ORDER),
    ("sig/imagekitio/models/update_file_request.rbs", VARIANT_NAME),
    ("sig/imagekitio/models.rbs", WEBHOOKS),
    ("sig/imagekitio/resources/beta/v2/files.rbs", TOKEN),
    ("sig/imagekitio/resources/files.rbs", TOKEN),
    ("test/imagekitio/client_test.rb", MULTIPART),
    ("test/imagekitio/resources/accounts/origins_test.rb", EXAMPLES),
    ("test/imagekitio/resources/accounts/url_endpoints_test.rb", EXAMPLES),
    ("test/imagekitio/resources/cache/invalidation_test.rb", EXAMPLES),
    ("test/imagekitio/resources/custom_metadata_fields_test.rb", EXAMPLES),
    ("test/imagekitio/resources/files/bulk_test.rb", EXAMPLES),
    ("test/imagekitio/resources/files/metadata_test.rb", EXAMPLES),
    ("test/imagekitio/resources/files_test.rb", EXAMPLES),
    ("test/imagekitio/resources/folders_test.rb", EXAMPLES),
    ("test/imagekitio/resources/saved_extensions_test.rb", EXAMPLES),
];
const OLDER_PROJECT: &str =
    "older generation: imagekit's project files predate OneBusAway's (no scripts/mock, no CPU cap in the Rakefile)";
const CUSTOM: &str =
    "hand edits imagekit keeps in the generated tree (helpers, upload serialization, base_url_overridden?)";
const WEBHOOKS: &str = "Forge gap: webhook_unwrap methods and their event models are not in the model";
const PATHDOC: &str = "model gap: a path parameter's description is kept only when declared on the operation, which the model does not record";
const VARIANT_NAME: &str =
    "model gap: x-stainless-variantName and schema titles (Mixed, UpdateFileDetails) are not in the model";
const VARIANT_ORDER: &str = "model gap: the adapter lists a union's const variants last, Stainless in spec order";
const MAPPING: &str =
    "model gap: a discriminator mapping with several values for one schema (file, file-version) is not in the model";
const EXT_ITEM: &str = "converter gap FER1004: the ExtensionItem model's object form is not supported";
const TITLE: &str = "model gap: a schema title is read as its description";
const TOKEN: &str = "unexplained: Stainless keeps `token` third among the upload fields, out of alphabetical order";
const RECURSIVE: &str = "unexplained: Sorbet input types drop OrHash for the recursive Transformation model";
const EXAMPLES: &str = "Forge gap G18: parameter examples are not in the model";
const MULTIPART: &str = "hand edit: the multipart redirect test skips the body comparison";
const HARDBREAK: &str =
    "unexplained: text after a markdown hard break (a trailing backslash) keeps its source line breaks";

fn imagekit() -> Option<(PathBuf, RubyOutput)> {
    let oracle = common::oracle("imagekit-ruby")?;
    let annotated = common::annotated("imagekit-config", "openapi/v1.0.0.yaml", "stainless-config/main.yaml")?;
    let meta = RubyMeta {
        version: Some("4.5.0".into()),
        example: Some((vec!["files".into()], "upload".into())),
        email: Some("developer@imagekit.io".into()),
        disable_mock_tests: true,
        ..RubyMeta::default()
    };
    let out = generate(&common::model(&annotated), &meta).expect("generates");
    Some((oracle, out))
}

#[test]
fn imagekit_matches_the_oracle_but_for_the_known_differences() {
    let Some((oracle, out)) = imagekit() else { return };
    common::dump("FERRULE_RUBY_DUMP_RAW", &out.files);
    let Some(files) = common::format(&out.files) else {
        eprintln!("skipped: the gem's formatters are not installed (FERRULE_STREE, FERRULE_RUBOCOP)");
        return;
    };
    let (identical, differing) = compare(&oracle, &files, IMAGEKIT_KNOWN);
    eprintln!("imagekit-ruby: {identical}/{} generated files identical", files.len());
    assert!(differing.is_empty(), "differ from the oracle: {differing:#?}");
    assert_eq!(identical + IMAGEKIT_KNOWN.len(), files.len());
}
