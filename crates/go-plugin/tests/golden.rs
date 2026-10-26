//! Small APIs (`tests/<fixture>/{openapi,stainless}.yml`) against their committed golden trees
//! (`tests/<fixture>/golden`, as `ferrule-gen generate --target go` writes them, the runtime
//! options included), byte for byte. They cover what OneBusAway does not:
//!
//! - `synthetic`: a JSON body with a nested object, an enum and an array, a method without a
//!   response, an integer path param, a subresource, a resource model and a custom casing;
//! - `paginated`: cursor pagination shaped like knock's `items_cursor` (knock-go), with a named
//!   and an inline item type.
//!
//! Golden changes are reviewed by a human: never rewrite a tree to make this pass.

mod common;

use std::path::{Path, PathBuf};

use ferrule_go_plugin::{GoMeta, generate};

const FIXTURES: [&str; 2] = ["synthetic", "paginated"];

fn dir(fixture: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(fixture)
}

fn generated(fixture: &str) -> Vec<(String, String)> {
    let dir = dir(fixture);
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("fixture input");
    let annotated = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml"))
        .expect("converts")
        .output;
    let out = generate(&common::model(&annotated), &GoMeta::default()).expect("generates");
    let mut files = out.files;
    files.push((".ferrule/go-runtime.json".to_owned(), out.runtime.to_json()));
    files.sort();
    files
}

#[test]
fn fixtures_match_their_golden_trees() {
    for fixture in FIXTURES {
        let golden = dir(fixture).join("golden");
        let files = generated(fixture);
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            common::walk(&golden),
            paths,
            "{fixture}: the golden tree has other files"
        );
        for (path, text) in &files {
            let expected = std::fs::read_to_string(golden.join(path)).expect("golden file");
            assert!(*text == expected, "{fixture}: {path} differs from the golden tree");
        }
    }
}

#[test]
fn fixtures_are_gofmt_clean_and_deterministic() {
    for fixture in FIXTURES {
        let files = generated(fixture);
        assert_eq!(files, generated(fixture));
        if common::gofmt_path().is_none() {
            eprintln!("skipped gofmt: not installed");
            return;
        }
        for (path, text) in files.iter().filter(|(p, _)| common::is_go(p)) {
            assert!(
                common::gofmt(text).as_deref() == Some(text.as_str()),
                "{fixture}: {path} is not gofmt-clean"
            );
        }
    }
}
