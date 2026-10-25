//! The bin smoke test has no Rust equivalent here: the binary is `ferrule-gen convert` in
//! crates/ferrule-gen.

use ferrule_converter::cli::main;
use std::path::{Path, PathBuf};

/// Keys the public schema requires, before `resources`.
const HEAD: &str = "organization: { name: acme }
environments: { production: https://api.acme.test }
readme:
  example_requests:
    default: { type: request, endpoint: get /a, params: {} }
    headline: { type: request, endpoint: get /a, params: {} }
";

struct Run {
    code: i32,
    out: Vec<String>,
    err: Vec<String>,
}

fn run(words: &[&str]) -> Run {
    let args: Vec<String> = words.iter().map(|s| (*s).to_owned()).collect();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = main(&args, &mut out, &mut err);
    let lines = |b: &[u8]| String::from_utf8_lossy(b).lines().map(str::to_owned).collect();
    Run {
        code,
        out: lines(&out),
        err: lines(&err),
    }
}

/// A fresh directory with the spec and config the TypeScript tests write.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrule-convert-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("openapi.yml"), "paths:\n  /a:\n    get: { responses: {} }\n").expect("spec");
    std::fs::write(
        dir.join("stainless.yml"),
        format!("{HEAD}resources:\n  a:\n    methods:\n      list: get /a\n"),
    )
    .expect("config");
    dir
}

fn s(p: &Path) -> &str {
    p.to_str().expect("utf-8 path")
}

#[test]
fn writes_the_annotated_spec() {
    let dir = fixture("write");
    let target = dir.join("out.json");
    let r = run(&[
        s(&dir.join("openapi.yml")),
        s(&dir.join("stainless.yml")),
        "-o",
        s(&target),
    ]);
    assert_eq!(r.code, 0);
    assert_eq!(r.out, [format!("wrote {}", s(&target))]);
    let written = std::fs::read_to_string(&target).expect("output");
    assert!(written.ends_with("}\n"));
    let parsed = ferrule_converter::value::parse_json(&written).expect("json");
    let name = parsed
        .get("paths")
        .and_then(|p| p.get("/a"))
        .and_then(|i| i.get("get"))
        .and_then(|o| o.get("x-ferrule-method-name"));
    assert_eq!(name.and_then(ferrule_converter::Value::as_str), Some("list"));
}

#[test]
fn prints_usage_on_bad_arguments() {
    let dir = fixture("usage");
    let (spec, config) = (dir.join("openapi.yml"), dir.join("stainless.yml"));
    assert_eq!(run(&[s(&spec), s(&config)]).code, 2);
    let r = run(&[s(&spec), "--out", "x"]);
    assert!(r.err[0].starts_with("usage:"));
    assert_eq!(
        r.err[0],
        "usage: ferrule-convert <openapi.(yml|json)> <stainless.yml> -o <out.json> [--runtime-from <sdk checkout>]"
    );
}

#[test]
fn reports_unreadable_input() {
    let dir = fixture("unreadable");
    let r = run(&[
        s(&dir.join("missing.yml")),
        s(&dir.join("stainless.yml")),
        "-o",
        s(&dir.join("x.json")),
    ]);
    assert_eq!(r.code, 2);
    assert!(r.err[0].starts_with("ferrule-convert: "));
    assert_eq!(
        r.err[0],
        format!(
            "ferrule-convert: ENOENT: no such file or directory, open '{}'",
            s(&dir.join("missing.yml"))
        )
    );
}

#[test]
fn exits_1_and_writes_nothing_on_error_diagnostics() {
    let dir = fixture("errors");
    let bad = dir.join("bad.yml");
    std::fs::write(
        &bad,
        format!("{HEAD}resources:\n  a:\n    methods:\n      list: get /zzz\n"),
    )
    .expect("config");
    let target = dir.join("bad.json");
    let r = run(&[s(&dir.join("openapi.yml")), s(&bad), "-o", s(&target)]);
    assert_eq!(r.code, 1);
    assert!(r.err[0].contains("FER1001"));
    assert!(r.err[0].contains("#/resources/a/methods/list"));
    assert!(!target.exists());
}

#[test]
fn exits_1_with_located_diagnostics_on_an_invalid_config() {
    let dir = fixture("invalid");
    let invalid = dir.join("invalid.yml");
    std::fs::write(&invalid, "resources: {}\n").expect("config");
    let target = dir.join("invalid.json");
    let r = run(&[s(&dir.join("openapi.yml")), s(&invalid), "-o", s(&target)]);
    assert_eq!(r.code, 1);
    assert!(r.err[0].starts_with("error FER1111 "));
    assert!(!target.exists());
}
