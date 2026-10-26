//! The generated SDK against a real Stainless .NET SDK, byte for byte after csharpier lays it out
//! (as the SDK's `scripts/format` does). Skipped when `oracles/cache` is not fetched, and the
//! comparison when csharpier is not installed (`mise run csharp:tools`, see `tests/common`).
//!
//! imagekit-csharp (imagekit-dotnet 6.3.0, Apache-2.0) is proven against imagekit-csharp-config:
//! the md5 of its `stainless-config/main.yaml` and `openapi/v1.0.0.yaml` equal the SDK's
//! `.stats.yml` hashes.

mod common;

use std::path::{Path, PathBuf};

use ferrule_csharp_plugin::{CSharpMeta, CSharpOutput, generate};

fn imagekit() -> Option<(PathBuf, CSharpOutput)> {
    let oracle = common::oracle("imagekit-csharp")?;
    let annotated = common::annotated(
        "imagekit-csharp-config",
        "openapi/v1.0.0.yaml",
        "stainless-config/main.yaml",
    )?;
    let meta = CSharpMeta {
        version: Some("6.3.0".into()),
        docs: Some("https://imagekit.io/docs/api-reference".into()),
        license: Some("Apache-2.0".into()),
        disable_mock_tests: true,
        ..CSharpMeta::default()
    };
    let out = generate(&common::model(&annotated), &meta).expect("generates");
    Some((oracle, out))
}

/// Compares the formatted `files` with `oracle`; returns (identical, differing).
fn compare(oracle: &Path, files: &[(String, String)], known: &[(&str, &str)]) -> (usize, Vec<String>) {
    let mut identical = 0;
    let mut differing = Vec::new();
    for (path, text) in files {
        let Ok(expected) = std::fs::read(oracle.join(path)) else {
            if !known.iter().any(|(p, _)| p == path) {
                differing.push(format!("{path} (not in the oracle)"));
            }
            continue;
        };
        let is_known = known.iter().find(|(p, _)| p == path);
        match (text.as_bytes() == expected.as_slice(), is_known) {
            (true, None) => identical += 1,
            (true, Some((p, _))) => panic!("{p} is a known difference but now matches; drop it"),
            (false, Some(_)) => {}
            (false, None) => differing.push(path.clone()),
        }
    }
    (identical, differing)
}

// Why imagekit-csharp files differ. Each file lists its main reason; most have only that one.
const DOC_WRAP: &str = "doc wrap: Stainless moves some words of a doc comment between lines in a \
     way no fixed width explains; the text is the same";
const EXAMPLES: &str = "test values: the model carries no schema examples (Forge gap G18), so \
     the tests pass property names, 0 and true, and build bodies from required properties only";
const WEBHOOKS: &str = "webhooks: the model has no webhooks (Forge gap), so their service and \
     unwrap enums are missing; the Helper service is an imagekit hand edit";
const HAND_EDIT: &str = "hand edit: the SYSLIB1045 line is imagekit's own, not generated";
const INLINE_NAMES: &str = "inline names: Stainless names some inline enums and union variants \
     after the referenced model or its title (FileType, RemoveBg, UpdateFileDetails) where the \
     model gives only the property path";
const MIXED_UNION: &str = "mixed unions: a union of a string, a bool enum or a list orders and \
     names its variants differently (Default, Mixed) and wraps lists in ImmutableArray";
const SHARED_CONST: &str = "shared const: Stainless does not lift a const property shared by \
     every variant (`type`) onto the union";
const PROP_ORDER: &str = "property order: one variant keeps `type` before `username`, unlike the \
     sorted order every other class follows";
const MULTI_DISCRIMINATOR: &str = "discriminator: a mapping with several values for one variant \
     (file and file-version) is matched as one case in Stainless's converter";
const MULTIPART: &str = "multipart: upload bodies use MultipartJsonDictionary and binary file \
     fields; the plugin sends JSON";
const NS_ALIAS: &str = "namespace alias: a referenced namespace whose last segment equals this \
     file's (Files) is aliased as `Files::`";
const DECLARING_RESOURCE: &str = "declaring resource: the model does not say which resource \
     declares a shared model, so Folder lands under Assets instead of Files";

const ITEM_MODEL: &str = "item model: the config names `#/components/schemas/Extensions/items` \
     ExtensionItem, but the model has no model at a pointer inside another schema, so the item \
     union is named after the alias (Extension)";

/// imagekit-csharp files that differ, with the main reason.
const IMAGEKIT_KNOWN: &[(&str, &str)] = &[
    (".editorconfig", HAND_EDIT),
    ("src/Imagekit.Tests/Services/Accounts/OriginServiceTest.cs", EXAMPLES),
    (
        "src/Imagekit.Tests/Services/Accounts/UrlEndpointServiceTest.cs",
        EXAMPLES,
    ),
    ("src/Imagekit.Tests/Services/Beta/V2/FileServiceTest.cs", EXAMPLES),
    ("src/Imagekit.Tests/Services/Cache/InvalidationServiceTest.cs", EXAMPLES),
    (
        "src/Imagekit.Tests/Services/CustomMetadataFieldServiceTest.cs",
        EXAMPLES,
    ),
    ("src/Imagekit.Tests/Services/FileServiceTest.cs", EXAMPLES),
    ("src/Imagekit.Tests/Services/Files/BulkServiceTest.cs", EXAMPLES),
    ("src/Imagekit.Tests/Services/Files/MetadataServiceTest.cs", EXAMPLES),
    ("src/Imagekit.Tests/Services/FolderServiceTest.cs", EXAMPLES),
    ("src/Imagekit.Tests/Services/SavedExtensionServiceTest.cs", EXAMPLES),
    ("src/Imagekit/Core/ModelBase.cs", WEBHOOKS),
    ("src/Imagekit/IImageKitClient.cs", WEBHOOKS),
    ("src/Imagekit/ImageKitClient.cs", WEBHOOKS),
    ("src/Imagekit/Models/Accounts/Origins/OriginDeleteParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Accounts/Origins/OriginGetParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Accounts/Origins/OriginRequest.cs", PROP_ORDER),
    (
        "src/Imagekit/Models/Accounts/UrlEndpoints/UrlEndpointCreateParams.cs",
        SHARED_CONST,
    ),
    (
        "src/Imagekit/Models/Accounts/UrlEndpoints/UrlEndpointListParams.cs",
        DOC_WRAP,
    ),
    (
        "src/Imagekit/Models/Accounts/UrlEndpoints/UrlEndpointRequest.cs",
        SHARED_CONST,
    ),
    (
        "src/Imagekit/Models/Accounts/UrlEndpoints/UrlEndpointResponse.cs",
        SHARED_CONST,
    ),
    (
        "src/Imagekit/Models/Accounts/UrlEndpoints/UrlEndpointUpdateParams.cs",
        SHARED_CONST,
    ),
    ("src/Imagekit/Models/Accounts/Usage/UsageGetParams.cs", DOC_WRAP),
    (
        "src/Imagekit/Models/Accounts/UsageAnalytics/UsageAnalyticsGetParams.cs",
        DOC_WRAP,
    ),
    (
        "src/Imagekit/Models/Accounts/UsageAnalytics/UsageAnalyticsResponse.cs",
        INLINE_NAMES,
    ),
    ("src/Imagekit/Models/Assets/AssetListParams.cs", INLINE_NAMES),
    ("src/Imagekit/Models/Assets/AssetListResponse.cs", MULTI_DISCRIMINATOR),
    ("src/Imagekit/Models/Assets/Folder.cs", DECLARING_RESOURCE),
    ("src/Imagekit/Models/BaseOverlay.cs", DOC_WRAP),
    ("src/Imagekit/Models/Beta/V2/Files/FileUploadParams.cs", MULTIPART),
    ("src/Imagekit/Models/Beta/V2/Files/FileUploadResponse.cs", NS_ALIAS),
    (
        "src/Imagekit/Models/Cache/Invalidation/InvalidationCreateParams.cs",
        DOC_WRAP,
    ),
    (
        "src/Imagekit/Models/CustomMetadataFields/CustomMetadataField.cs",
        MIXED_UNION,
    ),
    (
        "src/Imagekit/Models/CustomMetadataFields/CustomMetadataFieldCreateParams.cs",
        MIXED_UNION,
    ),
    (
        "src/Imagekit/Models/CustomMetadataFields/CustomMetadataFieldListParams.cs",
        DOC_WRAP,
    ),
    (
        "src/Imagekit/Models/CustomMetadataFields/CustomMetadataFieldUpdateParams.cs",
        MIXED_UNION,
    ),
    ("src/Imagekit/Models/Dummy/DummyCreateParams.cs", INLINE_NAMES),
    ("src/Imagekit/Models/Extension.cs", ITEM_MODEL),
    ("src/Imagekit/Models/ExtensionConfig.cs", INLINE_NAMES),
    ("src/Imagekit/Models/Files/Bulk/BulkAddTagsParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/File.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/FileMoveParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/FileRenameParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/FileUpdateParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/FileUpdateResponse.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/FileUploadParams.cs", MULTIPART),
    ("src/Imagekit/Models/Files/FileUploadResponse.cs", DOC_WRAP),
    ("src/Imagekit/Models/Files/UpdateFileRequest.cs", INLINE_NAMES),
    ("src/Imagekit/Models/Files/Versions/VersionDeleteParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Folders/FolderCopyParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Folders/FolderCreateParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Folders/FolderRenameParams.cs", DOC_WRAP),
    ("src/Imagekit/Models/Folders/Job/JobGetResponse.cs", DOC_WRAP),
    ("src/Imagekit/Models/GetImageAttributesOptions.cs", DOC_WRAP),
    ("src/Imagekit/Models/ImageOverlay.cs", DOC_WRAP),
    ("src/Imagekit/Models/Overlay.cs", DOC_WRAP),
    ("src/Imagekit/Models/OverlayPosition.cs", DOC_WRAP),
    ("src/Imagekit/Models/OverlayTiming.cs", DOC_WRAP),
    ("src/Imagekit/Models/ResponsiveImageAttributes.cs", DOC_WRAP),
    (
        "src/Imagekit/Models/SavedExtensions/SavedExtensionCreateParams.cs",
        DOC_WRAP,
    ),
    (
        "src/Imagekit/Models/SavedExtensions/SavedExtensionListParams.cs",
        DOC_WRAP,
    ),
    (
        "src/Imagekit/Models/SavedExtensions/SavedExtensionUpdateParams.cs",
        DOC_WRAP,
    ),
    ("src/Imagekit/Models/SelectedFieldsSchemaItem.cs", MIXED_UNION),
    ("src/Imagekit/Models/SharedSavedExtension.cs", DOC_WRAP),
    ("src/Imagekit/Models/SolidColorOverlay.cs", DOC_WRAP),
    ("src/Imagekit/Models/SolidColorOverlayTransformation.cs", MIXED_UNION),
    ("src/Imagekit/Models/SrcOptions.cs", DOC_WRAP),
    ("src/Imagekit/Models/SubtitleOverlay.cs", DOC_WRAP),
    ("src/Imagekit/Models/SubtitleOverlayTransformation.cs", DOC_WRAP),
    ("src/Imagekit/Models/TextOverlay.cs", DOC_WRAP),
    ("src/Imagekit/Models/TextOverlayTransformation.cs", MIXED_UNION),
    ("src/Imagekit/Models/Transformation.cs", MIXED_UNION),
    ("src/Imagekit/Models/VideoOverlay.cs", DOC_WRAP),
    ("src/Imagekit/Services/Cache/IInvalidationService.cs", DOC_WRAP),
    ("src/Imagekit/Services/IFileService.cs", DOC_WRAP),
];

#[test]
fn imagekit_matches_the_oracle_but_for_the_known_differences() {
    let Some((oracle, out)) = imagekit() else { return };
    let mut raw = out.files.clone();
    raw.push((".ferrule/csharp-runtime.json".to_owned(), out.runtime.to_json()));
    common::dump("FERRULE_CSHARP_DUMP_RAW", &raw);
    let Some(files) = common::format(&out.files) else {
        eprintln!("skipped: {}", common::NOT_INSTALLED);
        return;
    };
    common::dump("FERRULE_CSHARP_DUMP", &files);
    let (identical, differing) = compare(&oracle, &files, IMAGEKIT_KNOWN);
    eprintln!("imagekit-csharp: {identical}/{} generated files identical", files.len());
    assert!(differing.is_empty(), "differ from the oracle: {differing:#?}");
    assert_eq!(identical + IMAGEKIT_KNOWN.len(), files.len());
}
