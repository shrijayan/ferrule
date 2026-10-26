//! `ferrule-gen`: stainless.yml + OpenAPI spec -> annotated spec -> Forge -> SDK files.
//!
//! ```text
//! ferrule-gen convert openapi.yml stainless.yml -o annotated.json
//! ferrule-gen generate annotated.json --out sdk [--target python|typescript|node]
//!     [--config stainless.yml] [--openapi openapi.yml]
//! ferrule-gen model annotated.json
//! ```
//!
//! `generate` replaces `forge annotated.json --target wasm://plugin.wasm --out sdk`: the same
//! input (a JSON document, or `-` for standard input), the same output directory handling
//! (`forge::write`), errors on standard error as `ferrule-gen: <error>` with exit code 1, and
//! exit code 2 for a usage error. `--target` names the SDK language (ADR 0017): `python` (the
//! default), `typescript` and `node` (`crates/ts-plugin`) and `go` (`crates/go-plugin`) have
//! plugins; the others fail with FER202. `node` writes Stainless's legacy TypeScript layout;
//! `typescript` writes the layout of the TypeScript target the config has (`targets.typescript`,
//! else the legacy `targets.node`).
//!
//! For `typescript` and `node` only, until the annotated spec carries them: `--config` reads the version-less
//! project settings (organization, README examples, license) from the stainless.yml, and
//! `--openapi` reads examples, tag descriptions and the written property order from the original
//! spec (the annotated one is written with sorted keys). Without them the plugin uses
//! placeholders and the annotated spec.
//!
//! For Go, the output holds the generated files and [`GO_RUNTIME_OPTIONS`], the options the Go
//! runtime template is rendered with (`ferrule_runtime.render_go_files`); `ferrule generate`
//! renders the runtime from it and removes it, as it renders Python's runtime.
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
       ferrule-gen generate <SPEC> --out <OUT> [--target <TARGET>] [--config <YML>] [--openapi <SPEC>]
       ferrule-gen model <SPEC>

Commands:
  convert   Turn an OpenAPI spec and a stainless.yml into the annotated spec
  generate  Generate an SDK from an annotated spec (a JSON document, or - for stdin)
  model     Print the language-neutral SDK model of an annotated spec as JSON

Targets: python (default), typescript, node (the legacy TypeScript layout), go; java, kotlin,
ruby, php, csharp, cli and terraform are planned and not supported yet";

/// `ferrule-gen <crate version> forge <40-hex rev>`: the rev is the Forge commit pinned in the
/// workspace `Cargo.lock` (read by `build.rs`). `ferrule.lock` records both parts.
const VERSION: &str = concat!(
    "ferrule-gen ",
    env!("CARGO_PKG_VERSION"),
    " forge ",
    env!("FERRULE_FORGE_REV")
);

/// The SDK languages `--target` accepts, by their stainless.yml `targets` key; [`PLUGINS`] have a
/// plugin.
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

const TYPESCRIPT: &str = "typescript";

const NODE: &str = "node";

const GO: &str = "go";

/// The targets with a plugin.
const PLUGINS: [&str; 4] = [DEFAULT_TARGET, TYPESCRIPT, NODE, GO];

/// Where `generate --target go` writes the Go runtime's options (JSON), relative to `--out`.
pub const GO_RUNTIME_OPTIONS: &str = ".ferrule/go-runtime.json";

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
    /// `--config`: the stainless.yml (TypeScript project settings).
    config: Option<PathBuf>,
    /// `--openapi`: the original spec (TypeScript examples and property order).
    openapi: Option<PathBuf>,
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
    let mut config: Option<PathBuf> = None;
    let mut openapi: Option<PathBuf> = None;
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
        } else if let Some(value) = flag_value(args, &mut i, "--config", None, "YML") {
            config = Some(PathBuf::from(value?));
        } else if let Some(value) = flag_value(args, &mut i, "--openapi", None, "SPEC") {
            openapi = Some(PathBuf::from(value?));
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
            config,
            openapi,
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
        format!(
            "generate with --target {}, the targets with a plugin so far",
            PLUGINS.join(" or ")
        ),
    )
}

fn generate(args: &[String], stderr: &mut dyn Write) -> i32 {
    let args = match parse_generate(args) {
        Ok(args) => args,
        Err(message) => return usage(stderr, &message),
    };
    let written = match args.target.as_str() {
        DEFAULT_TARGET => write_sdk(&args),
        TYPESCRIPT => write_typescript(&args, None),
        NODE => write_typescript(&args, Some(ferrule_ts_plugin::Layout::Node)),
        GO => write_go(&args),
        other => return fail(stderr, &unsupported_target(other)),
    };
    match written {
        Ok(()) => 0,
        Err(err) => fail(stderr, &err),
    }
}

/// Reads the annotated spec, runs Forge with the plugin and writes the files.
fn write_sdk(args: &GenerateArgs) -> Result<(), forge::Err> {
    let forge = read(&args.spec)?;
    forge::write(&args.out, forge::run(&forge, &PythonPlugin::default())?)
}

/// One file of the TypeScript SDK, for `forge::write`.
struct TsFile(String, String);

impl forge::File for TsFile {
    fn path(&self) -> &str {
        &self.0
    }

    fn content(&self) -> Result<std::borrow::Cow<'_, [u8]>, forge::Err> {
        Ok(std::borrow::Cow::Borrowed(self.1.as_bytes()))
    }
}

/// Reads a JSON or YAML document with the converter's reader, as `serde_json` in written order.
fn read_document(path: &Path) -> Result<serde_json::Value, forge::Err> {
    let text = fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let value = ferrule_converter::parse_spec(&text)
        .or_else(|_| ferrule_converter::yaml::parse(&text))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(converter_json(&value))
}

/// A converter value as `serde_json`, keys in the same order.
fn converter_json(v: &ferrule_converter::Value) -> serde_json::Value {
    use ferrule_converter::Value as V;
    match v {
        V::Null => serde_json::Value::Null,
        V::Bool(b) => serde_json::Value::Bool(*b),
        V::Number(n) => {
            #[expect(clippy::cast_possible_truncation, reason = "checked to be integral and in range")]
            let int = (n.fract() == 0.0 && n.abs() < 9.0e15).then_some(*n as i64);
            int.map_or_else(
                || serde_json::Number::from_f64(*n).map_or(serde_json::Value::Null, serde_json::Value::Number),
                |i| serde_json::Value::Number(i.into()),
            )
        }
        V::String(s) => serde_json::Value::String(s.clone()),
        V::Array(a) => serde_json::Value::Array(a.iter().map(converter_json).collect()),
        V::Object(m) => serde_json::Value::Object(m.iter().map(|(k, v)| (k.clone(), converter_json(v))).collect()),
    }
}

/// Reads the annotated spec, maps it to the neutral model and writes the TypeScript SDK's files
/// in `layout`, else the layout of the config's TypeScript target (the runtime comes from
/// `ferrule_runtime.render_typescript_files` or `render_typescript_node_files`, as Python's
/// does).
fn write_typescript(args: &GenerateArgs, layout: Option<ferrule_ts_plugin::Layout>) -> Result<(), forge::Err> {
    let forge = read(&args.spec)?;
    let Some(client) = ferrule_sdk_model::forge_to_model(&forge)? else {
        return forge::write(&args.out, Vec::<TsFile>::new());
    };
    let meta = match &args.config {
        Some(path) => ferrule_ts_plugin::TsMeta::from_config(&read_document(path)?),
        None => ferrule_ts_plugin::TsMeta::placeholder(),
    };
    let doc = match &args.openapi {
        Some(path) => read_document(path)?,
        None => read_document(&args.spec)?,
    };
    let extras = ferrule_ts_plugin::SpecExtras::new(doc);
    let layout = layout.unwrap_or_else(|| ferrule_ts_plugin::Layout::of(&client));
    let files = ferrule_ts_plugin::generate_layout(&client, &meta, &extras, layout);
    let executable: Vec<String> = files
        .iter()
        .map(|(p, _)| p.clone())
        .filter(|p| ferrule_ts_plugin::is_executable(p))
        .collect();
    forge::write(&args.out, files.into_iter().map(|(p, c)| TsFile(p, c)).collect())?;
    for p in executable {
        make_executable(&args.out.join(p))?;
    }
    Ok(())
}

/// `chmod +x`, as the oracles have `scripts/*`.
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), forge::Err> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|err| format!("{}: {err}", path.display()))?
        .permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms).map_err(|err| format!("{}: {err}", path.display()).into())
}

#[cfg(not(unix))]
fn make_executable(_: &Path) -> Result<(), forge::Err> {
    Ok(())
}

/// One file of the Go plugin's output, for `forge::write`.
struct GoFile {
    path: String,
    content: String,
}

impl forge::File for GoFile {
    fn path(&self) -> &str {
        &self.path
    }

    fn content(&self) -> Result<std::borrow::Cow<'_, [u8]>, forge::Err> {
        Ok(std::borrow::Cow::Borrowed(self.content.as_bytes()))
    }
}

/// Reads the annotated spec, maps it to the neutral model and writes the Go plugin's files with
/// the runtime options; nothing for a document without operations.
fn write_go(args: &GenerateArgs) -> Result<(), forge::Err> {
    let forge = read(&args.spec)?;
    let Some(client) = ferrule_sdk_model::forge_to_model(&forge)? else {
        return forge::write(&args.out, Vec::<GoFile>::new());
    };
    let out = ferrule_go_plugin::generate(&client, &ferrule_go_plugin::GoMeta::default())?;
    let mut files: Vec<GoFile> = out
        .files
        .into_iter()
        .map(|(path, content)| GoFile { path, content })
        .collect();
    files.push(GoFile {
        path: GO_RUNTIME_OPTIONS.to_owned(),
        content: out.runtime.to_json(),
    });
    forge::write(&args.out, files)
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
        let parsed = parse_generate(&strings(&[
            "a.json",
            "-o",
            "sdk",
            "--target",
            "typescript",
            "--config",
            "s.yml",
            "--openapi=o.yml",
        ]))
        .expect("valid");
        assert_eq!(
            (parsed.target.as_str(), parsed.config, parsed.openapi),
            ("typescript", Some(PathBuf::from("s.yml")), Some(PathBuf::from("o.yml")))
        );
    }

    #[test]
    fn rejects_unknown_targets_and_missing_values() {
        let err = parse_generate(&strings(&["a.json", "--out", "sdk", "--target", "cobol"])).expect_err("unknown");
        assert!(err.contains("possible values: python, typescript"), "{err}");
        let fix = unsupported_target("ruby").fix;
        assert!(fix.contains("python or typescript or node or go"), "{fix}");
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
