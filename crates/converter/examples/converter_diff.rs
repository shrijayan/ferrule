//! Differential driver for `scripts/rs/converter_diff.py`: runs `convert_text` (or the CLI) on
//! each case of a JSON list and writes one JSON line per case, in the shape the TypeScript
//! driver writes, so the script can compare the two byte for byte.
//!
//! Usage: `converter_diff <cases.json> <out.jsonl>`, where each case is
//! `{"spec": path, "config": path}` or `{"cli": [args...]}`.

use ferrule_converter::value::{Map, Value, parse_json, to_json};
use ferrule_converter::{Diagnostic, cli, convert_text};
use std::io::Write as _;

fn diagnostic(d: &Diagnostic) -> Value {
    let mut m = Map::new();
    m.insert("code", Value::from(d.code.as_str()));
    m.insert("severity", Value::from(d.severity.to_string()));
    m.insert("message", Value::from(d.message.as_str()));
    m.insert("pointer", Value::from(d.pointer.as_str()));
    m.insert("suggestion", Value::from(d.suggestion.as_str()));
    Value::Object(m)
}

fn run(case: &Value) -> Value {
    let mut out = Map::new();
    if let Some(Value::Array(args)) = case.get("cli") {
        let args: Vec<String> = args.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = cli::main(&args, &mut stdout, &mut stderr);
        out.insert("code", Value::Number(f64::from(code)));
        out.insert("stdout", Value::from(String::from_utf8_lossy(&stdout).into_owned()));
        out.insert("stderr", Value::from(String::from_utf8_lossy(&stderr).into_owned()));
        return Value::Object(out);
    }
    let read = |k: &str| std::fs::read_to_string(case.get(k).and_then(Value::as_str).unwrap_or("")).unwrap_or_default();
    match convert_text(&read("spec"), &read("config")) {
        Ok(r) => {
            out.insert("output", Value::from(r.output));
            out.insert(
                "diagnostics",
                Value::Array(r.diagnostics.iter().map(diagnostic).collect()),
            );
        }
        Err(e) => {
            out.insert("error", Value::from(e));
        }
    }
    Value::Object(out)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cases, out_path] = args.as_slice() else {
        eprintln!("usage: converter_diff <cases.json> <out.jsonl>");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(cases).unwrap_or_default();
    let Some(Value::Array(cases)) = parse_json(&text) else {
        eprintln!("converter_diff: {cases} is not a JSON list");
        std::process::exit(2);
    };
    let Ok(mut file) = std::fs::File::create(out_path) else {
        eprintln!("converter_diff: cannot write {out_path}");
        std::process::exit(2);
    };
    for case in &cases {
        let line = to_json(&run(case));
        if writeln!(file, "{line}").is_err() {
            std::process::exit(2);
        }
    }
}
