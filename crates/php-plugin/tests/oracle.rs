//! The generated package against a real Stainless PHP SDK, byte for byte after php-cs-fixer lays
//! it out (as the package's `scripts/format` does). Skipped when `oracles/cache` is not fetched,
//! and the comparisons when php-cs-fixer is not installed (`mise run php:tools`, see
//! `tests/common`).
//!
//! imagekit-php (Apache-2.0, `9f9309d5`) is the generation of the imagekit config oracle: the
//! `.stats.yml` hashes are those of the `openapi/v1.0.0.yaml` and `stainless-config/main.yaml`
//! the other imagekit oracles pair with.

mod common;

use std::path::PathBuf;

use ferrule_php_plugin::{PhpMeta, PhpOutput, generate};

/// Oracle files that are not generated output: release-please's, or the runtime template's
/// (`runtime/php`, proven by `runtime/tests/test_render_oracles_php.py`).
fn not_generated(path: &str) -> bool {
    path == "src/Core.php"
        || path == "src/RequestOptions.php"
        || path.starts_with("src/Core/")
        || path.starts_with("tests/Core/")
        || path == "CHANGELOG.md"
}

/// Project files the plugin does not emit yet: the README and the docs, CI, release-please's
/// files, `bin/`, and what composer and the generator record (`composer.lock`, `.stats.yml`).
fn project_file(path: &str) -> bool {
    const NOT_YET: [&str; 8] = [
        "README.md",
        "SECURITY.md",
        "LICENSE",
        "composer.lock",
        ".stats.yml",
        ".release-please-manifest.json",
        "release-please-config.json",
        "bin/check-release-environment",
    ];
    NOT_YET.contains(&path) || path.starts_with(".github/")
}

fn imagekit() -> Option<(PathBuf, PhpOutput)> {
    let oracle = common::oracle("imagekit-php")?;
    let annotated = common::annotated("imagekit-config", "openapi/v1.0.0.yaml", "stainless-config/main.yaml")?;
    let meta = PhpMeta {
        example: Some((vec!["files".into()], "upload".into())),
        disable_mock_tests: true,
        ..PhpMeta::default()
    };
    let out = generate(&common::model(&annotated), &meta).expect("generates");
    Some((oracle, out))
}

/// Compares the formatted `files` with `oracle`; returns (identical, differing).
fn compare(oracle: &std::path::Path, files: &[(String, String)]) -> (usize, Vec<String>) {
    common::dump("FERRULE_PHP_DUMP", files);
    let mut identical = 0;
    let mut differing = Vec::new();
    for (path, text) in files {
        let Ok(expected) = std::fs::read_to_string(oracle.join(path)) else {
            if renamed(path).is_none() {
                differing.push(format!("{path} (not in the oracle)"));
            }
            continue;
        };
        let is_known = IMAGEKIT_KNOWN.iter().find(|(p, _)| p == path);
        match (text == &expected, is_known) {
            (true, None) => identical += 1,
            (true, Some((p, _))) => panic!("{p} is a known difference but now matches; drop it"),
            (false, Some(_)) => {}
            (false, None) => differing.push(path.clone()),
        }
    }
    (identical, differing)
}

/// imagekit-php files that differ, with the main reason (the test fails when one of them starts
/// matching).
const IMAGEKIT_KNOWN: &[(&str, &str)] = &[
    ("composer.json", WEBHOOKS),
    ("src/Accounts/Origins/OriginRequest.php", TITLE),
    ("src/Accounts/Origins/OriginResponse.php", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointCreateParams.php", TITLE),
    (
        "src/Accounts/URLEndpoints/URLEndpointCreateParams/URLRewriter.php",
        TITLE,
    ),
    ("src/Accounts/URLEndpoints/URLEndpointRequest.php", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointRequest/URLRewriter.php", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointResponse.php", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointResponse/URLRewriter.php", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointUpdateParams.php", TITLE),
    (
        "src/Accounts/URLEndpoints/URLEndpointUpdateParams/URLRewriter.php",
        TITLE,
    ),
    ("src/Assets/AssetListResponseItem.php", PLACE),
    ("src/Beta/V2/Files/FileUploadParams.php", TOKEN),
    ("src/Beta/V2/Files/FileUploadParams/Transformation/Post.php", TITLE),
    (
        "src/Beta/V2/Files/FileUploadResponse/SelectedFieldsSchema/DefaultValue.php",
        TITLE,
    ),
    ("src/Client.php", WEBHOOKS),
    (
        "src/CustomMetadataFields/CustomMetadataField/Schema/DefaultValue.php",
        TITLE,
    ),
    (
        "src/CustomMetadataFields/CustomMetadataFieldCreateParams/Schema/DefaultValue.php",
        TITLE,
    ),
    (
        "src/CustomMetadataFields/CustomMetadataFieldUpdateParams/Schema/DefaultValue.php",
        TITLE,
    ),
    ("src/ExtensionConfig.php", TITLE),
    ("src/ExtensionConfig/AutoTaggingExtension.php", TITLE_DOC),
    ("src/ExtensionItem.php", TITLE),
    ("src/ExtensionItem/AutoTaggingExtension.php", TITLE_DOC),
    ("src/Files/File/SelectedFieldsSchema/DefaultValue.php", TITLE),
    ("src/Files/FileUpdateParams.php", VARIANT_ORDER),
    ("src/Files/FileUpdateParams/RemoveAITags.php", VARIANT_ORDER),
    ("src/Files/FileUploadParams.php", TOKEN),
    ("src/Files/FileUploadParams/Transformation/Post.php", TITLE),
    (
        "src/Files/FileUploadResponse/SelectedFieldsSchema/DefaultValue.php",
        TITLE,
    ),
    ("src/Files/UpdateFileRequest.php", TITLE),
    ("src/SavedExtension.php", TITLE),
    ("src/SavedExtensions/SavedExtensionCreateParams.php", TITLE),
    ("src/SavedExtensions/SavedExtensionUpdateParams.php", TITLE),
    ("src/ServiceContracts/Accounts/OriginsContract.php", TITLE),
    ("src/ServiceContracts/Accounts/OriginsRawContract.php", TITLE),
    ("src/ServiceContracts/Accounts/URLEndpointsContract.php", TITLE),
    ("src/ServiceContracts/AssetsContract.php", PLACE),
    ("src/ServiceContracts/AssetsRawContract.php", PLACE),
    ("src/ServiceContracts/Beta/V2/FilesContract.php", TOKEN),
    ("src/ServiceContracts/FilesContract.php", TOKEN),
    ("src/ServiceContracts/SavedExtensionsContract.php", TITLE),
    ("src/Services/Accounts/OriginsRawService.php", TITLE),
    ("src/Services/Accounts/OriginsService.php", TITLE),
    ("src/Services/Accounts/URLEndpointsService.php", TITLE),
    ("src/Services/AssetsRawService.php", PLACE),
    ("src/Services/AssetsService.php", PLACE),
    ("src/Services/Beta/V2/FilesRawService.php", UPLOAD),
    ("src/Services/Beta/V2/FilesService.php", TOKEN),
    ("src/Services/FilesRawService.php", UPLOAD),
    ("src/Services/FilesService.php", TOKEN),
    ("src/Services/SavedExtensionsService.php", TITLE),
    ("src/SolidColorOverlayTransformation/Gradient.php", VARIANT_ORDER),
    ("src/Transformation/AIDropShadow.php", VARIANT_ORDER),
    ("src/Transformation/Gradient.php", VARIANT_ORDER),
    ("src/Transformation/Shadow.php", VARIANT_ORDER),
    ("src/Transformation/Sharpen.php", VARIANT_ORDER),
    ("src/Transformation/Trim.php", VARIANT_ORDER),
    ("src/Transformation/UnsharpMask.php", VARIANT_ORDER),
    ("tests/Services/Accounts/OriginsTest.php", EXAMPLES),
    ("tests/Services/Accounts/URLEndpointsTest.php", EXAMPLES),
    ("tests/Services/Beta/V2/FilesTest.php", EXAMPLES),
    ("tests/Services/Cache/InvalidationTest.php", EXAMPLES),
    ("tests/Services/CustomMetadataFieldsTest.php", EXAMPLES),
    ("tests/Services/Files/BulkTest.php", EXAMPLES),
    ("tests/Services/Files/MetadataTest.php", EXAMPLES),
    ("tests/Services/FilesTest.php", EXAMPLES),
    ("tests/Services/FoldersTest.php", EXAMPLES),
    ("tests/Services/SavedExtensionsTest.php", EXAMPLES),
];

/// Where the plugin and the oracle name files differently, by path prefix, with the reason: the
/// files under these that only one of them has.
const RENAMED: &[(&str, &str)] = &[
    ("src/Accounts/Origins/OriginRequest/", TITLE),
    ("src/Accounts/Origins/OriginResponse/", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointCreateParams/URLRewriter/", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointRequest/URLRewriter/", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointResponse/URLRewriter/", TITLE),
    ("src/Accounts/URLEndpoints/URLEndpointUpdateParams/URLRewriter/", TITLE),
    ("src/Beta/V2/Files/FileUploadParams/Transformation/Post/", TITLE),
    ("src/Files/FileUploadParams/Transformation/Post/", TITLE),
    ("src/Files/UpdateFileRequest/", TITLE),
    ("src/ExtensionConfig/", TITLE),
    ("src/ExtensionItem/", TITLE),
    (
        "src/Beta/V2/Files/FileUploadResponse/SelectedFieldsSchema/DefaultValue/",
        TITLE,
    ),
    (
        "src/CustomMetadataFields/CustomMetadataField/Schema/DefaultValue/",
        TITLE,
    ),
    (
        "src/CustomMetadataFields/CustomMetadataFieldCreateParams/Schema/DefaultValue/",
        TITLE,
    ),
    (
        "src/CustomMetadataFields/CustomMetadataFieldUpdateParams/Schema/DefaultValue/",
        TITLE,
    ),
    ("src/Files/File/SelectedFieldsSchema/DefaultValue/", TITLE),
    ("src/Files/FileUploadResponse/SelectedFieldsSchema/DefaultValue/", TITLE),
    ("src/Assets/Folder.php", PLACE),
    ("src/Files/Folder", PLACE),
    ("src/Webhooks/", WEBHOOKS),
    ("src/Services/Webhooks", WEBHOOKS),
    ("src/ServiceContracts/Webhooks", WEBHOOKS),
    ("tests/Services/WebhooksTest.php", WEBHOOKS),
];

fn renamed(path: &str) -> Option<&'static str> {
    RENAMED.iter().find(|(p, _)| path.starts_with(p)).map(|(_, r)| *r)
}

const TITLE: &str = "model gap: Stainless names a union variant after its schema title (`Google Cloud Storage (GCS)` -> GoogleCloudStorageGcs, `Mixed` -> Mixed_), which the model does not carry; the plugin names it after its discriminator value or position";
const TITLE_DOC: &str = "model gap: a schema title is read as the variant's description";
const VARIANT_ORDER: &str = "model gap: the adapter lists a union's const variants last, Stainless in spec order";
const PLACE: &str = "model gap: a model's module is the converter's (Python's) and does not say which resource declares it; `Folder` (files) is used only by assets, so it lands in Assets";
const TOKEN: &str = "unexplained: Stainless keeps `token` third among the upload fields, out of alphabetical order (as in imagekit-ruby); the services also list one shape import later";
const WEBHOOKS: &str = "Forge gap: webhook_unwrap methods and their event models are not in the model (the client's webhooks service, composer's standard-webhooks); Client.php also keeps imagekit's hand-edited baseUrlOverridden";
const EXAMPLES: &str = "Forge gap G18: parameter examples are not in the model";
const UPLOAD: &str =
    "hand edit: imagekit sends uploads to upload.imagekit.io unless the base URL is overridden; and TOKEN";

#[test]
fn imagekit_matches_the_oracle_but_for_the_known_differences() {
    let Some((oracle, out)) = imagekit() else { return };
    common::dump("FERRULE_PHP_DUMP_RAW", &out.files);
    let Some(files) = common::format(&out.files) else {
        eprintln!("skipped: {}", common::NOT_INSTALLED);
        return;
    };
    let (identical, differing) = compare(&oracle, &files);
    let renamed_ours = files.iter().filter(|(p, _)| !oracle.join(p).is_file()).count();
    eprintln!("imagekit-php: {identical}/{} generated files identical", files.len());
    assert!(differing.is_empty(), "differ from the oracle: {differing:#?}");
    assert_eq!(identical + IMAGEKIT_KNOWN.len() + renamed_ours, files.len());
}

#[test]
fn imagekit_emits_every_generated_file() {
    let Some((oracle, out)) = imagekit() else { return };
    let missing: Vec<String> = common::walk(&oracle)
        .into_iter()
        .filter(|p| !not_generated(p) && !project_file(p) && renamed(p).is_none())
        .filter(|p| !out.files.iter().any(|(f, _)| f == p))
        .collect();
    assert!(missing.is_empty(), "not emitted: {missing:#?}");
}

#[test]
fn imagekit_fills_the_runtime_options() {
    let Some((_, out)) = imagekit() else { return };
    let r = &out.runtime;
    assert_eq!(r.namespace, "Imagekit");
    assert_eq!((r.timeout, r.max_retries), (60.0, 2));
    assert_eq!((r.initial_retry_delay, r.max_retry_delay), (0.5, 8.0));
}
