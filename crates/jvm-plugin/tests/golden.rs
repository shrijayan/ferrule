//! A small synthetic API (`tests/synthetic/{openapi,stainless}.yml`) against its committed golden
//! tree (`tests/synthetic/golden`), byte for byte: the Java flavor's Kotlin files after ktfmt, as
//! `ferrule generate --target java` writes them, and the runtime options. The project files are
//! the same templates the oracle test proves, so they are not repeated here. It covers what
//! OneBusAway does not: a JSON body with a nested object, an enum and an array, a method without a
//! response, an integer path param, a subresource with two path params, a resource model, a
//! nullable field, a query array, bearer auth and a custom casing.
//!
//! Golden changes are reviewed by a human: never rewrite the tree to make this pass.

mod common;

use std::path::Path;

use ferrule_jvm_plugin::{Flavor, JvmMeta, generate};

fn generated(flavor: Flavor) -> (Vec<(String, String)>, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("synthetic input");
    let annotated = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml"))
        .expect("converts")
        .output;
    let out = generate(&common::model(&annotated), flavor, &JvmMeta::default()).expect("generates");
    (out.files, out.runtime.to_json())
}

fn is_kotlin(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e == "kt")
}

#[test]
fn synthetic_matches_the_golden_tree() {
    let (files, runtime) = generated(Flavor::Java);
    let kotlin: Vec<(String, String)> = files.into_iter().filter(|(p, _)| is_kotlin(p)).collect();
    let Some(mut formatted) = common::ktfmt(&kotlin, "synthetic-golden") else {
        return;
    };
    formatted.push((".ferrule/jvm-runtime.json".to_owned(), runtime));
    formatted.sort();
    common::dump("FERRULE_JVM_GOLDEN_DUMP", &formatted);
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic/golden");
    let paths: Vec<&str> = formatted.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(common::walk(&golden), paths, "the golden tree has other files");
    for (path, text) in &formatted {
        let expected = std::fs::read_to_string(golden.join(path)).expect("golden file");
        assert!(*text == expected, "{path} differs from the golden tree");
    }
}

#[test]
fn synthetic_is_deterministic_in_both_flavors() {
    for flavor in [Flavor::Java, Flavor::Kotlin] {
        assert_eq!(generated(flavor), generated(flavor));
    }
}

#[test]
fn synthetic_kotlin_flavor_parses() {
    let (files, _) = generated(Flavor::Kotlin);
    let kotlin: Vec<(String, String)> = files.into_iter().filter(|(p, _)| is_kotlin(p)).collect();
    // ktfmt fails on a file that does not parse.
    let _ = common::ktfmt(&kotlin, "synthetic-kotlin");
}
