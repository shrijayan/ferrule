//! `ferrule-gen`: stainless.yml + OpenAPI spec -> annotated spec -> Forge -> Python SDK files.
//!
//! ```text
//! ferrule-gen convert openapi.yml stainless.yml -o annotated.json
//! ferrule-gen generate annotated.json --out sdk
//! ```
//!
//! `generate` replaces `forge annotated.json --target wasm://plugin.wasm --out sdk`: the same
//! input (a JSON document, or `-` for standard input), the same output directory handling
//! (`forge::write`), errors on standard error as `ferrule-gen: <error>` with exit code 1, and
//! exit code 2 for a usage error.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferrule_python_plugin::PythonPlugin;

const USAGE: &str = "Usage: ferrule-gen convert <SPEC> <CONFIG> -o <OUT>
       ferrule-gen generate <SPEC> --out <OUT>

Commands:
  convert   Turn an OpenAPI spec and a stainless.yml into the annotated spec
  generate  Generate the Python SDK from an annotated spec (a JSON document, or - for stdin)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, &mut io::stdout().lock(), &mut io::stderr().lock());
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

/// Runs the command line `args` (without the program name); returns the exit code.
fn run(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match args.first().map(String::as_str) {
        Some("convert") => ferrule_converter::cli::main(&args[1..], stdout, stderr),
        Some("generate") => generate(&args[1..], stderr),
        Some("-h" | "--help" | "help") => {
            let _ = writeln!(stdout, "{USAGE}");
            0
        }
        Some("-V" | "--version") => {
            let _ = writeln!(stdout, "ferrule-gen {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some(other) => usage(stderr, &format!("unrecognized subcommand '{other}'")),
        None => usage(stderr, "a subcommand is required"),
    }
}

/// Prints a usage error the way the Forge CLI does; returns exit code 2.
fn usage(stderr: &mut dyn Write, message: &str) -> i32 {
    let _ = writeln!(
        stderr,
        "error: {message}\n\n{USAGE}\n\nFor more information, try '--help'."
    );
    2
}

/// The arguments of `generate`.
struct GenerateArgs {
    spec: PathBuf,
    out: PathBuf,
}

fn parse_generate(args: &[String]) -> Result<GenerateArgs, String> {
    let mut spec: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--out" || arg == "-o" {
            i += 1;
            let value = args
                .get(i)
                .ok_or_else(|| format!("a value is required for '{arg} <OUT>'"))?;
            out = Some(PathBuf::from(value));
        } else if let Some(value) = arg.strip_prefix("--out=") {
            out = Some(PathBuf::from(value));
        } else if arg.starts_with('-') && arg != "-" {
            return Err(format!("unexpected argument '{arg}' found"));
        } else if spec.is_none() {
            spec = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument '{arg}' found"));
        }
        i += 1;
    }
    match (spec, out) {
        (Some(spec), Some(out)) => Ok(GenerateArgs { spec, out }),
        (spec, out) => {
            let mut missing = Vec::new();
            if out.is_none() {
                missing.push("  --out <OUT>");
            }
            if spec.is_none() {
                missing.push("  <SPEC>");
            }
            Err(format!(
                "the following required arguments were not provided:\n{}",
                missing.join("\n")
            ))
        }
    }
}

fn generate(args: &[String], stderr: &mut dyn Write) -> i32 {
    let args = match parse_generate(args) {
        Ok(args) => args,
        Err(message) => return usage(stderr, &message),
    };
    match write_sdk(&args) {
        Ok(()) => 0,
        Err(err) => {
            let _ = writeln!(stderr, "ferrule-gen: {err}");
            1
        }
    }
}

/// Reads the annotated spec, runs Forge with the plugin and writes the files.
fn write_sdk(args: &GenerateArgs) -> Result<(), forge::Err> {
    let forge = read(&args.spec)?;
    forge::write(&args.out, forge::run(&forge, &PythonPlugin::default())?)
}

/// Reads the document at `path`, or from standard input when `path` is `-`.
fn read(path: &Path) -> Result<forge::Forge, forge::Err> {
    let parse = || -> Result<forge::Forge, forge::Err> {
        let json = if path == Path::new("-") {
            let mut json = Vec::new();
            io::stdin().read_to_end(&mut json)?;
            json
        } else {
            fs::read(path)?
        };
        Ok(serde_json::from_slice(&json)?)
    };
    parse().map_err(|err| format!("{}: {err}", path.display()).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|&a| a.to_owned()).collect()
    }

    #[test]
    fn parses_generate_arguments_in_any_order() {
        for args in [["a.json", "--out", "sdk"], ["--out", "sdk", "a.json"]] {
            let parsed = parse_generate(&strings(&args)).expect("valid");
            assert_eq!(
                (parsed.spec, parsed.out),
                (PathBuf::from("a.json"), PathBuf::from("sdk"))
            );
        }
        let parsed = parse_generate(&strings(&["-", "--out=sdk"])).expect("valid");
        assert_eq!(parsed.spec, PathBuf::from("-"));
    }

    #[test]
    fn reports_usage_errors_with_exit_code_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run(&strings(&["generate", "a.json"]), &mut out, &mut err), 2);
        assert!(String::from_utf8_lossy(&err).contains("--out <OUT>"));
        assert_eq!(run(&strings(&["bogus"]), &mut out, &mut err), 2);
        assert_eq!(run(&[], &mut out, &mut err), 2);
        assert_eq!(run(&strings(&["--help"]), &mut out, &mut err), 0);
    }
}
