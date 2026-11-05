//! `ferrule-gen generate --spec <openapi> --config <stainless.yml> --out <dir>`: the whole Python
//! pipeline in one command (ADR 0020).
//!
//! The same steps as `ferrule generate` (`migrate/src/ferrule/generate.py`): convert, Forge with
//! the Python plugin, the runtime from the embedded template ([`crate::runtime`]), `ruff` to sort
//! imports and format, `chmod +x scripts/*`. It refuses and fails with the same `FER4xxx`
//! diagnostics, and prints the same summary line, so the two commands are interchangeable.

use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use ferrule_python_plugin::PythonPlugin;
use sha2::{Digest, Sha256};

/// `ferrule generate`'s codes (`migrate/src/ferrule/errors.py`), for the same failures.
const MISSING_INPUT: &str = "FER4001";
const TOOL_NOT_FOUND: &str = "FER4002";
const STEP_FAILED: &str = "FER4003";
const NO_PACKAGE: &str = "FER4005";
const OUT_NOT_EMPTY: &str = "FER4006";

/// Directory names never counted, digested or copied (`SKIP_PARTS`).
const SKIP_PARTS: [&str; 5] = ["__pycache__", ".ruff_cache", ".mypy_cache", ".pytest_cache", ".git"];

/// The arguments of the one-command form.
#[derive(Debug, PartialEq, Eq)]
pub struct SdkArgs {
    /// `--spec`: the OpenAPI spec.
    pub spec: PathBuf,
    /// `--config`: the stainless.yml.
    pub config: PathBuf,
    /// `--out`: a new or empty directory.
    pub out: PathBuf,
}

/// A failure with its address: displays as `<code> at <pointer>: <message>. Fix: <fix>`.
#[derive(Debug)]
struct Diagnostic {
    code: &'static str,
    message: String,
    pointer: String,
    fix: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}: {}. Fix: {}",
            self.code, self.pointer, self.message, self.fix
        )
    }
}

fn diagnostic(code: &'static str, message: String, pointer: &str, fix: &str) -> Diagnostic {
    Diagnostic {
        code,
        message,
        pointer: pointer.to_owned(),
        fix: fix.to_owned(),
    }
}

/// A failed pipeline step: `<step> failed: <detail>`.
fn step_failed(step: &str, detail: &dyn std::fmt::Display) -> Diagnostic {
    diagnostic(
        STEP_FAILED,
        format!("{step} failed: {detail}"),
        step,
        &format!("fix the error {step} reported"),
    )
}

/// What a run produced.
struct Generated {
    package: String,
    files: usize,
    runtime_files: usize,
    digest: String,
}

/// Runs the pipeline; prints the summary line, or the diagnostic and exit code 1.
pub fn main(args: &SdkArgs, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let ruff = std::env::var("FERRULE_RUFF").unwrap_or_else(|_| "ruff".to_owned());
    match generate(args, &ruff, stderr) {
        Ok(g) => {
            let _ = writeln!(
                stdout,
                "generated {}: {} files ({} runtime) sha256 {}",
                g.package, g.files, g.runtime_files, g.digest
            );
            0
        }
        Err(d) => {
            let _ = writeln!(stderr, "ferrule-gen: {d}");
            1
        }
    }
}

fn generate(args: &SdkArgs, ruff: &str, stderr: &mut dyn Write) -> Result<Generated, Diagnostic> {
    check_inputs(args, ruff)?;
    fs::create_dir_all(&args.out).map_err(|err| step_failed("generate", &format!("{}: {err}", args.out.display())))?;
    let annotated = convert(args, stderr)?;
    let forge: forge::Forge = serde_json::from_str(&annotated).map_err(|err| step_failed("generate", &err))?;
    forge::run(&forge, &PythonPlugin::default())
        .and_then(|files| forge::write(&args.out, files))
        .map_err(|err| step_failed("generate", &err))?;

    let package = pick_package(&args.out)?;
    let package_dir = args.out.join("src").join(&package);
    let before = relative_files(&package_dir).len();
    let rendered = crate::runtime::render(&package).map_err(|err| step_failed("runtime", &err))?;
    for (rel, text) in rendered {
        let path = package_dir.join(rel);
        let written = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, text));
        written.map_err(|err| step_failed("runtime", &format!("{}: {err}", path.display())))?;
    }
    let runtime_files = relative_files(&package_dir).len().saturating_sub(before);

    let out = args.out.as_os_str();
    // Emitters write imports in one fixed order; the SDK's own isort settings restore the order
    // Stainless's lint-fix step gives.
    let sort = [
        "check",
        "--select",
        "I",
        "--fix",
        "--no-cache",
        "--quiet",
        "--exit-zero",
    ];
    run_tool("sort-imports", ruff, &sort, out)?;
    run_tool("format", ruff, &["format", "--no-cache", "--quiet"], out)?;
    mark_scripts_executable(&args.out).map_err(|err| step_failed("generate", &err))?;

    let files = relative_files(&args.out);
    Ok(Generated {
        package,
        runtime_files,
        digest: tree_digest(&args.out, &files).map_err(|err| step_failed("digest", &err))?,
        files: files.len(),
    })
}

fn check_inputs(args: &SdkArgs, ruff: &str) -> Result<(), Diagnostic> {
    for (label, path) in [("--spec", &args.spec), ("--config", &args.config)] {
        if !path.is_file() {
            let message = format!("{} does not exist", path.display());
            return Err(diagnostic(MISSING_INPUT, message, label, "pass an existing file"));
        }
    }
    if !on_path(ruff) {
        return Err(diagnostic(
            TOOL_NOT_FOUND,
            format!("{ruff} is not on PATH"),
            "FERRULE_RUFF",
            "install it (curl -LsSf https://astral.sh/ruff/install.sh | sh) or set FERRULE_RUFF",
        ));
    }
    let not_empty = fs::read_dir(&args.out).is_ok_and(|mut entries| entries.next().is_some());
    if not_empty {
        let message = format!("{} is not empty", args.out.display());
        return Err(diagnostic(OUT_NOT_EMPTY, message, "--out", "pass a new directory"));
    }
    Ok(())
}

/// `shutil.which`: a path with a separator is checked as is, a bare name in every `PATH` entry.
fn on_path(tool: &str) -> bool {
    let tool = Path::new(tool);
    if tool.components().count() > 1 {
        return tool.is_file();
    }
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE".to_owned())
            .split(';')
            .map(str::to_owned)
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| {
            exts.iter()
                .any(|ext| dir.join(format!("{}{ext}", tool.display())).is_file())
        })
    })
}

/// The annotated spec, with the converter's diagnostics on standard error as `convert` prints them.
fn convert(args: &SdkArgs, stderr: &mut dyn Write) -> Result<String, Diagnostic> {
    let read = |path: &Path| {
        fs::read_to_string(path).map_err(|err| step_failed("convert", &format!("{}: {err}", path.display())))
    };
    let (spec, config) = (read(&args.spec)?, read(&args.config)?);
    let result = ferrule_converter::convert_text(&spec, &config).map_err(|err| step_failed("convert", &err))?;
    // `ferrule generate` reports the last line `convert` printed.
    let mut last = None;
    for d in &result.diagnostics {
        let line = format!(
            "{} {} {}#{}: {} ({})",
            d.severity,
            d.code,
            args.config.display(),
            d.pointer,
            d.message,
            d.suggestion
        );
        let _ = writeln!(stderr, "{line}");
        last = Some(line);
    }
    let failed = result
        .diagnostics
        .iter()
        .any(|d| d.severity == ferrule_converter::Severity::Error);
    match last.filter(|_| failed) {
        Some(line) => Err(step_failed("convert", &line)),
        None => Ok(result.output),
    }
}

/// The one importable package under `<out>/src/` with an `__init__.py` (`pick_package`).
fn pick_package(out: &Path) -> Result<String, Diagnostic> {
    let mut names: Vec<String> = fs::read_dir(out.join("src"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().join("__init__.py").is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| is_identifier(name))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    if let [name] = names.as_slice() {
        return Ok(name.clone());
    }
    let found = if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    };
    Err(diagnostic(
        NO_PACKAGE,
        format!("expected exactly one package under src/, found {found}"),
        "--out",
        "point --runtime-from at a Stainless-style SDK with src/<package>/__init__.py",
    ))
}

/// `str.isidentifier` for the ASCII names a package has.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Runs `tool args.. out`; a non-zero exit is FER4003 with the last line it printed.
fn run_tool(step: &str, tool: &str, args: &[&str], out: &std::ffi::OsStr) -> Result<(), Diagnostic> {
    let output = Command::new(tool)
        .args(args)
        .arg(out)
        .output()
        .map_err(|err| step_failed(step, &err))?;
    if output.status.success() {
        return Ok(());
    }
    let text = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    let text = String::from_utf8_lossy(text);
    let last = text.trim().lines().last().map_or_else(
        || format!("exit code {}", output.status.code().unwrap_or(1)),
        str::to_owned,
    );
    Err(step_failed(step, &last))
}

/// `chmod +x` the files in `scripts/`, as Stainless SDKs ship them (mode 100755).
#[cfg(unix)]
fn mark_scripts_executable(out: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Ok(entries) = fs::read_dir(out.join("scripts")) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_file() {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o111);
            fs::set_permissions(&path, perms)?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn mark_scripts_executable(_: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Every regular file under `root` as a `/`-separated path, skipping [`SKIP_PARTS`], in the
/// order Python sorts `PurePosixPath`s (part by part).
fn relative_files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let rel = format!("{prefix}{name}");
            if kind.is_dir() && !SKIP_PARTS.contains(&name.as_str()) {
                walk(&entry.path(), &format!("{rel}/"), out);
            } else if kind.is_file() {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out.sort_by(|a, b| a.split('/').cmp(b.split('/')));
    out
}

/// `tree_digest`: sha256 over every file's path, length and bytes.
fn tree_digest(root: &Path, files: &[String]) -> std::io::Result<String> {
    let mut h = Sha256::new();
    for rel in files {
        let data = fs::read(root.join(rel))?;
        h.update(format!("{rel}\0{}\0", data.len()).as_bytes());
        h.update(&data);
    }
    Ok(h.finalize().iter().fold(String::with_capacity(64), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_paths_part_by_part() {
        let dir = std::env::temp_dir().join(format!("ferrule-gen-sdk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for rel in ["a.py", "a/b.py", "a/__pycache__/x.pyc", ".git/HEAD", "b"] {
            let path = dir.join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            fs::write(&path, rel).expect("file");
        }
        assert_eq!(relative_files(&dir), ["a/b.py", "a.py", "b"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diagnostics_have_addresses() {
        let d = diagnostic(
            OUT_NOT_EMPTY,
            "sdk is not empty".to_owned(),
            "--out",
            "pass a new directory",
        );
        assert_eq!(
            d.to_string(),
            "FER4006 at --out: sdk is not empty. Fix: pass a new directory"
        );
        assert!(is_identifier("acme_sdk") && !is_identifier("acme-sdk") && !is_identifier("1a"));
    }
}
