//! A small synthetic API (`tests/synthetic/{openapi,stainless}.yml`): what OneBusAway does not
//! cover, such as a JSON body with a nested object, an enum and an array, a method without a
//! response, an integer path param, a subresource, shared models (one a map of objects) and a
//! custom casing.

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
    files
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
