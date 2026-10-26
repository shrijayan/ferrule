//! A small synthetic API (`tests/synthetic/{openapi,stainless}.yml`) against its committed golden
//! tree (`tests/synthetic/golden`, as `ferrule-gen generate --target ruby` writes it before the
//! formatters run, the runtime options included), byte for byte. It covers what OneBusAway does
//! not: a JSON body with a nested object, an enum and an array, a method without a response, an
//! integer path param, a subresource, shared models (one a map of objects) and a custom casing.
//!
//! Golden changes are reviewed by a human: never rewrite the tree to make this pass.

mod common;

use std::path::Path;

use ferrule_ruby_plugin::{RubyMeta, generate};

fn generated() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("synthetic input");
    let converted = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml")).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    let annotated = converted.output;
    let out = generate(&common::model(&annotated), &RubyMeta::default()).expect("generates");
    let mut files = out.files;
    files.push((".ferrule/ruby-runtime.json".to_owned(), out.runtime.to_json()));
    files.sort();
    common::dump("FERRULE_RUBY_DUMP_RAW", &files);
    files
}

#[test]
fn synthetic_matches_the_golden_tree() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic/golden");
    let files = generated();
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(common::walk(&golden), paths, "the golden tree has other files");
    for (path, text) in &files {
        let expected = std::fs::read_to_string(golden.join(path)).expect("golden file");
        assert!(*text == expected, "{path} differs from the golden tree");
    }
}

/// The output is the same on every run, and the gem's formatters accept it and are settled after
/// one pass (`rake format` on the formatted gem changes nothing).
#[test]
fn synthetic_is_deterministic_and_formats() {
    let files = generated();
    assert_eq!(files, generated());
    let Some(formatted) = common::format(&files) else {
        eprintln!("skipped formatting: {}", common::NOT_INSTALLED);
        return;
    };
    assert_eq!(common::format(&formatted), Some(formatted));
}

fn file<'f>(files: &'f [(String, String)], path: &str) -> &'f str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is not generated"), |(_, t)| t.as_str())
}

/// A converter constant's item model (`Labels = HashOf[-> { LabelsItem }]`) is referred to
/// through the top-level alias, as every named model is, so `models.rb` declares it.
#[test]
fn aliases_the_item_model_of_a_converter() {
    let files = generated();
    let labels = file(&files, "lib/acme/models/labels.rb");
    assert!(labels.contains("Acme::LabelsItem"), "{labels}");
    let models = file(&files, "lib/acme/models.rb");
    assert!(models.contains("  LabelsItem = Acme::Models::LabelsItem\n"), "{models}");
    assert!(file(&files, "rbi/acme/models.rbi").contains("  LabelsItem = Acme::Models::LabelsItem\n"));
    assert!(file(&files, "sig/acme/models.rbs").contains("  class LabelsItem = Acme::Models::LabelsItem\n"));
}

/// The top-level alias of a converter constant is typed as a converter in each view, as
/// imagekit-ruby's `CustomMetadata` is.
#[test]
fn types_the_alias_of_a_converter_constant() {
    let files = generated();
    let models = file(&files, "lib/acme/models.rb");
    assert!(
        models.contains("  # @type [Acme::Internal::Type::Converter]\n  Labels = Acme::Models::Labels\n"),
        "{models}"
    );
    let rbi = file(&files, "rbi/acme/models.rbi");
    assert!(
        rbi.contains("  Labels = T.let(Acme::Models::Labels, Acme::Internal::Type::Converter)\n"),
        "{rbi}"
    );
    let rbs = file(&files, "sig/acme/models.rbs");
    assert!(rbs.contains("  Labels: Acme::Internal::Type::Converter\n"), "{rbs}");
}
