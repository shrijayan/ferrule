//! `ferrule-gen convert`: the only module in the converter crate that touches the file system.
//! Port of `converter/src/cli.ts`.

use crate::{Severity, convert_text};
use std::io::Write;

const USAGE: &str = "usage: ferrule-convert <openapi.(yml|json)> <stainless.yml> -o <out.json>";

/// Node's message for a failed `readFileSync`, such as
/// `ENOENT: no such file or directory, open 'x.yml'`.
fn read_error(path: &str, err: &std::io::Error) -> String {
    let (code, text) = match err.kind() {
        std::io::ErrorKind::NotFound => ("ENOENT", "no such file or directory"),
        std::io::ErrorKind::PermissionDenied => ("EACCES", "permission denied"),
        std::io::ErrorKind::IsADirectory => ("EISDIR", "illegal operation on a directory"),
        std::io::ErrorKind::InvalidData => return format!("{path}: stream did not contain valid UTF-8"),
        _ => return format!("{err}, open '{path}'"),
    };
    let verb = if code == "EISDIR" { "read" } else { "open" };
    if verb == "read" {
        format!("{code}: {text}, read")
    } else {
        format!("{code}: {text}, {verb} '{path}'")
    }
}

/// Run the CLI on `args` (after the subcommand); returns the exit code (0 ok, 1 error
/// diagnostics, 2 usage or input error).
pub fn main(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let mut positional: Vec<&str> = Vec::new();
    let mut out: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "-o" || arg == "--out" {
            i += 1;
            out = args.get(i).map(String::as_str);
        } else {
            positional.push(arg);
        }
        i += 1;
    }
    let (Some(out), [spec_path, config_path]) = (out.filter(|o| !o.is_empty()), positional.as_slice()) else {
        let _ = writeln!(stderr, "{USAGE}");
        return 2;
    };
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| read_error(p, &e));
    let result = read(spec_path).and_then(|spec| read(config_path).and_then(|config| convert_text(&spec, &config)));
    let result = match result {
        Ok(r) => r,
        Err(message) => {
            let _ = writeln!(stderr, "ferrule-convert: {message}");
            return 2;
        }
    };
    for d in &result.diagnostics {
        let _ = writeln!(
            stderr,
            "{} {} {config_path}#{}: {} ({})",
            d.severity, d.code, d.pointer, d.message, d.suggestion
        );
    }
    if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return 1;
    }
    if let Err(e) = std::fs::write(out, &result.output) {
        // The TypeScript CLI does not catch this: Node prints the error and exits 1.
        let _ = writeln!(stderr, "ferrule-convert: {}", read_error(out, &e));
        return 1;
    }
    let _ = writeln!(stdout, "wrote {out}");
    0
}
