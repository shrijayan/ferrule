//! A small synthetic API (`tests/synthetic/{openapi,stainless}.yml`) against its committed golden
//! tree (`tests/synthetic/golden`, as `ferrule-gen generate --target php` writes it before
//! php-cs-fixer runs, the runtime options included), byte for byte. It covers what imagekit does
//! not: a bearer token, integer path and query params, a date-time query param, a map-of-objects
//! model, a list response with a cursor, and a custom casing.
//!
//! Golden changes are reviewed by a human: never rewrite the tree to make this pass.

mod common;

use std::path::Path;

use ferrule_php_plugin::{PhpMeta, generate};

fn generated() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic");
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).expect("synthetic input");
    let converted = ferrule_converter::convert_text(&read("openapi.yml"), &read("stainless.yml")).expect("converts");
    assert!(!converted.output.is_empty(), "{:?}", converted.diagnostics);
    let out = generate(&common::model(&converted.output), &PhpMeta::default()).expect("generates");
    let mut files = out.files;
    files.push((".ferrule/php-runtime.json".to_owned(), out.runtime.to_json()));
    files.sort();
    common::dump("FERRULE_PHP_DUMP_RAW", &files);
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

/// The output is the same on every run, and php-cs-fixer accepts it and is settled after one pass
/// (`scripts/format` on the formatted package changes nothing).
#[test]
fn synthetic_is_deterministic_and_formats() {
    let files = generated();
    assert_eq!(files, generated());
    let php: Vec<(String, String)> = files.into_iter().filter(|(p, _)| !p.starts_with(".ferrule/")).collect();
    let Some(formatted) = common::format(&php) else {
        eprintln!("skipped formatting: {}", common::NOT_INSTALLED);
        return;
    };
    assert_eq!(common::format(&formatted), Some(formatted));
}

/// Every assertion of a service test is narrowed already for PHPStan, so each one is preceded by
/// the ignore comment imagekit-php's tests carry (a union response's `assertNotNull` too).
#[test]
fn every_test_assertion_ignores_the_narrowed_type() {
    for (path, text) in generated().iter().filter(|(p, _)| p.starts_with("tests/Services/")) {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("$this->assert") {
                assert_eq!(
                    lines[i - 1].trim(),
                    "// @phpstan-ignore-next-line method.alreadyNarrowedType",
                    "{path}:{}",
                    i + 1
                );
            }
        }
    }
}
