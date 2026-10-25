//! `ferrule-gen`: stainless.yml + OpenAPI spec -> annotated spec -> Forge -> SDK files.
//!
//! ```text
//! ferrule-gen convert openapi.yml stainless.yml -o annotated.json
//! ferrule-gen generate annotated.json --out sdk [--target python]
//! ferrule-gen model annotated.json
//! ```
//!
//! `generate` replaces `forge annotated.json --target wasm://plugin.wasm --out sdk`: the same
//! input (a JSON document, or `-` for standard input), the same output directory handling
//! (`forge::write`), errors on standard error as `ferrule-gen: <error>` with exit code 1, and
//! exit code 2 for a usage error. `--target` names the SDK language (ADR 0017); Python, the
//! default, is the only one with a plugin so far, and the others fail with FER202.
//!
//! `model` prints the language-neutral SDK model (`ferrule-sdk-model`) of an annotated spec as
//! deterministic JSON, for debugging and for plugin tests.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ferrule_python_plugin::PythonPlugin;
use ferrule_sdk_model::{FerCode, FerruleError};

const USAGE: &str = "Usage: ferrule-gen convert <SPEC> <CONFIG> -o <OUT>
       ferrule-gen generate <SPEC> --out <OUT> [--target <TARGET>]
       ferrule-gen model <SPEC>

Commands:
  convert   Turn an OpenAPI spec and a stainless.yml into the annotated spec
  generate  Generate an SDK from an annotated spec (a JSON document, or - for stdin)
  model     Print the language-neutral SDK model of an annotated spec as JSON

Targets: python (default); typescript, node, go, java, kotlin, ruby, php, csharp, cli and
terraform are planned and not supported yet";

/// `ferrule-gen <crate version> forge <40-hex rev>`: the rev is the Forge commit pinned in the
/// workspace `Cargo.lock` (read by `build.rs`). `ferrule.lock` records both parts.
const VERSION: &str = concat!(
    "ferrule-gen ",
    env!("CARGO_PKG_VERSION"),
    " forge ",
    env!("FERRULE_FORGE_REV")
);

/// The SDK languages `--target` accepts, by their stainless.yml `targets` key; only the first
/// has a plugin.
const TARGETS: [&str; 11] = [
    "python",
    "typescript",
    "node",
    "go",
    "java",
    "kotlin",
    "ruby",
    "php",
    "csharp",
    "cli",
    "terraform",
];

const DEFAULT_TARGET: &str = "python";

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
        Some("model") => model(&args[1..], stdout, stderr),
        Some("-h" | "--help" | "help") => {
            let _ = writeln!(stdout, "{USAGE}");
            0
        }
        Some("-V" | "--version") => {
            let _ = writeln!(stdout, "{VERSION}");
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

/// Prints `ferrule-gen: <error>`; returns exit code 1.
fn fail(stderr: &mut dyn Write, err: &dyn std::fmt::Display) -> i32 {
    let _ = writeln!(stderr, "ferrule-gen: {err}");
    1
}

/// The arguments of `generate`.
#[derive(Debug)]
struct GenerateArgs {
    spec: PathBuf,
    out: PathBuf,
    target: String,
}

/// The value of `--name <value>` or `--name=value` at `args[*i]`, advancing `i` past it.
fn flag_value(
    args: &[String],
    i: &mut usize,
    long: &str,
    short: Option<&str>,
    meta: &str,
) -> Option<Result<String, String>> {
    let arg = &args[*i];
    if arg == long || short.is_some_and(|s| arg == s) {
        *i += 1;
        return Some(
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("a value is required for '{arg} <{meta}>'")),
        );
    }
    arg.strip_prefix(long)
        .and_then(|rest| rest.strip_prefix('='))
        .map(|value| Ok(value.to_owned()))
}

fn parse_generate(args: &[String]) -> Result<GenerateArgs, String> {
    let mut spec: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut target: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if let Some(value) = flag_value(args, &mut i, "--out", Some("-o"), "OUT") {
            out = Some(PathBuf::from(value?));
        } else if let Some(value) = flag_value(args, &mut i, "--target", None, "TARGET") {
            let value = value?;
            if !TARGETS.contains(&value.as_str()) {
                return Err(format!(
                    "invalid value '{value}' for '--target <TARGET>'\n  [possible values: {}]",
                    TARGETS.join(", ")
                ));
            }
            target = Some(value);
        } else if args[i].starts_with('-') && args[i] != "-" {
            return Err(format!("unexpected argument '{}' found", args[i]));
        } else if spec.is_none() {
            spec = Some(PathBuf::from(&args[i]));
        } else {
            return Err(format!("unexpected argument '{}' found", args[i]));
        }
        i += 1;
    }
    match (spec, out) {
        (Some(spec), Some(out)) => Ok(GenerateArgs {
            spec,
            out,
            target: target.unwrap_or_else(|| DEFAULT_TARGET.to_owned()),
        }),
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

/// FER202 for a target that has no plugin yet.
fn unsupported_target(target: &str) -> FerruleError {
    FerruleError::new(
        FerCode::Fer202,
        "--target",
        format!("target {target} is not supported yet"),
        format!("generate with --target {DEFAULT_TARGET}, the only target with a plugin so far"),
    )
}

fn generate(args: &[String], stderr: &mut dyn Write) -> i32 {
    let args = match parse_generate(args) {
        Ok(args) => args,
        Err(message) => return usage(stderr, &message),
    };
    if args.target != DEFAULT_TARGET {
        return fail(stderr, &unsupported_target(&args.target));
    }
    match write_sdk(&args) {
        Ok(()) => 0,
        Err(err) => fail(stderr, &err),
    }
}

/// Reads the annotated spec, runs Forge with the plugin and writes the files.
fn write_sdk(args: &GenerateArgs) -> Result<(), forge::Err> {
    let forge = read(&args.spec)?;
    forge::write(&args.out, forge::run(&forge, &PythonPlugin::default())?)
}

fn parse_model(args: &[String]) -> Result<PathBuf, String> {
    match args {
        [spec] if !spec.starts_with('-') || spec == "-" => Ok(PathBuf::from(spec)),
        [] => Err("the following required arguments were not provided:\n  <SPEC>".to_owned()),
        [arg] => Err(format!("unexpected argument '{arg}' found")),
        [_, extra, ..] => Err(format!("unexpected argument '{extra}' found")),
    }
}

/// `model`: the neutral model of the annotated spec on standard output.
fn model(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let spec = match parse_model(args) {
        Ok(spec) => spec,
        Err(message) => return usage(stderr, &message),
    };
    let forge = match read(&spec) {
        Ok(forge) => forge,
        Err(err) => return fail(stderr, &err),
    };
    match ferrule_sdk_model::forge_to_model(&forge) {
        Ok(client) => match stdout.write_all(ferrule_sdk_model::to_json(client.as_ref()).as_bytes()) {
            Ok(()) => 0,
            Err(err) => fail(stderr, &err),
        },
        Err(err) => fail(stderr, &err),
    }
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
        for args in [
            &["a.json", "--out", "sdk"][..],
            &["--out", "sdk", "a.json"],
            &["--target", "python", "a.json", "-o", "sdk"],
        ] {
            let parsed = parse_generate(&strings(args)).expect("valid");
            assert_eq!(
                (parsed.spec, parsed.out, parsed.target.as_str()),
                (PathBuf::from("a.json"), PathBuf::from("sdk"), "python")
            );
        }
        let parsed = parse_generate(&strings(&["-", "--out=sdk", "--target=go"])).expect("valid");
        assert_eq!((parsed.spec, parsed.target.as_str()), (PathBuf::from("-"), "go"));
    }

    #[test]
    fn rejects_unknown_targets_and_missing_values() {
        let err = parse_generate(&strings(&["a.json", "--out", "sdk", "--target", "cobol"])).expect_err("unknown");
        assert!(err.contains("possible values: python, typescript"), "{err}");
        let err = parse_generate(&strings(&["a.json", "--out", "sdk", "--target"])).expect_err("no value");
        assert!(err.contains("'--target <TARGET>'"), "{err}");
    }

    #[test]
    fn parses_model_arguments() {
        assert_eq!(parse_model(&strings(&["a.json"])), Ok(PathBuf::from("a.json")));
        assert_eq!(parse_model(&strings(&["-"])), Ok(PathBuf::from("-")));
        assert!(parse_model(&[]).is_err());
        assert!(parse_model(&strings(&["--out", "x"])).is_err());
        assert!(parse_model(&strings(&["a.json", "b.json"])).is_err());
    }

    #[test]
    fn version_names_the_pinned_forge_rev() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run(&strings(&["--version"]), &mut out, &mut err), 0);
        let line = String::from_utf8(out).expect("utf-8");
        let words: Vec<&str> = line.trim_end().split(' ').collect();
        assert_eq!(words[..3], ["ferrule-gen", env!("CARGO_PKG_VERSION"), "forge"]);
        assert_eq!(words.len(), 4);
        // The rev from Cargo.lock is the one the workspace manifest pins.
        let manifest = include_str!("../../../Cargo.toml");
        assert!(manifest.contains(&format!("rev = \"{}\"", words[3])), "{line}");
    }

    #[test]
    fn reports_usage_errors_with_exit_code_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run(&strings(&["generate", "a.json"]), &mut out, &mut err), 2);
        assert!(String::from_utf8_lossy(&err).contains("--out <OUT>"));
        assert_eq!(run(&strings(&["bogus"]), &mut out, &mut err), 2);
        assert_eq!(run(&strings(&["model"]), &mut out, &mut err), 2);
        assert_eq!(run(&[], &mut out, &mut err), 2);
        assert_eq!(run(&strings(&["--help"]), &mut out, &mut err), 0);
    }
}
