//! Test inputs: annotated specs from the converter and the neutral model from them, and ktfmt.
//! This is the only place in the crate that names Forge (to parse the annotated spec for
//! `ferrule_sdk_model::forge_to_model`); `src/` never does (see `boundary.rs`).

// Each test binary that includes this module uses only part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use ferrule_sdk_model::model::Client;

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The neutral model of an annotated spec.
pub fn model(annotated: &str) -> Client {
    let forge: forge::Forge = serde_json::from_str(annotated).expect("a document Forge reads");
    ferrule_sdk_model::forge_to_model(&forge)
        .expect("converts")
        .expect("has operations")
}

/// The annotated spec of a config oracle in `oracles/cache` (paths relative to it), or `None`
/// when it is not fetched.
pub fn annotated(spec: &str, config: &str) -> Option<String> {
    let dir = root().join("oracles/cache");
    let spec = std::fs::read_to_string(dir.join(spec)).ok()?;
    let config = std::fs::read_to_string(dir.join(config)).ok()?;
    Some(
        ferrule_converter::convert_text(&spec, &config)
            .expect("converts")
            .output,
    )
}

/// An oracle SDK checkout, or `None` (with a note) when it is not fetched.
pub fn oracle(name: &str) -> Option<PathBuf> {
    let dir = root().join("oracles/cache").join(name);
    if dir.join("settings.gradle.kts").is_file() {
        Some(dir)
    } else {
        eprintln!("skipped: oracles/cache/{name} is not fetched");
        None
    }
}

/// The ktfmt jar: `$FERRULE_KTFMT_JAR`, else the one mise installs (`mise.toml`).
pub fn ktfmt_jar() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FERRULE_KTFMT_JAR") {
        return Some(PathBuf::from(p));
    }
    let out = Command::new("mise")
        .args(["where", "github:facebook/ktfmt"])
        .current_dir(root())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    let jar = dir.join("ktfmt-0.61-with-dependencies.jar");
    jar.is_file().then_some(jar)
}

/// The `java` to run ktfmt with: `$JAVA_HOME/bin/java`, else the one mise installs.
pub fn java() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let p = PathBuf::from(home).join("bin/java");
        if p.is_file() {
            return Some(p);
        }
    }
    let out = Command::new("mise")
        .args(["where", "java"])
        .current_dir(root())
        .output()
        .ok()?;
    let p = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim()).join("bin/java");
    (out.status.success() && p.is_file()).then_some(p)
}

/// The files with every `.kt` file formatted by `ktfmt --kotlinlang-style`, or `None` (with a
/// note) when ktfmt or a JDK is not installed. Panics on a syntax error.
pub fn ktfmt(files: &[(String, String)], scratch: &str) -> Option<Vec<(String, String)>> {
    let (Some(jar), Some(java)) = (ktfmt_jar(), java()) else {
        eprintln!("skipped: ktfmt (mise install) or a JDK is not installed");
        return None;
    };
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(scratch);
    let _ = std::fs::remove_dir_all(&dir);
    let mut kt = Vec::new();
    for (path, text) in files {
        if Path::new(path).extension().is_some_and(|e| e == "kt") {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            std::fs::write(&p, text).expect("write");
            kt.push(p);
        }
    }
    if !kt.is_empty() {
        let out = Command::new(java)
            .arg("-jar")
            .arg(jar)
            .arg("--kotlinlang-style")
            .args(&kt)
            .output()
            .expect("ktfmt runs");
        assert!(out.status.success(), "ktfmt: {}", String::from_utf8_lossy(&out.stderr));
    }
    Some(
        files
            .iter()
            .map(|(path, text)| {
                let p = dir.join(path);
                if Path::new(path).extension().is_some_and(|e| e == "kt") {
                    (path.clone(), std::fs::read_to_string(p).expect("formatted"))
                } else {
                    (path.clone(), text.clone())
                }
            })
            .collect(),
    )
}

/// Ferrule's branding of the oracle's text (`src/emit/templates/BRANDING.md`): the header line,
/// the `.keep` files, `SECURITY.md`, and the CI workflow's Stainless-only runners and upload steps.
pub fn brand(path: &str, text: &str) -> String {
    let mut text = text
        .replace(
            "// File generated from our OpenAPI spec by Stainless.",
            ferrule_jvm_plugin::kt::HEADER,
        )
        .replace(
            "File generated from our OpenAPI spec by Stainless.",
            "File generated from our OpenAPI spec by Ferrule.",
        )
        .replace(
            "It is ignored by Stainless code generation",
            "It is ignored by Ferrule code generation",
        )
        .replace(
            "This SDK is generated by [Stainless Software Inc](http://stainless.com). Stainless takes security seriously, and encourages you to report any security vulnerability promptly so that appropriate action can be taken.",
            "This SDK is generated by Ferrule. We take security seriously, and encourage you to report any security vulnerability promptly so that appropriate action can be taken.",
        )
        .replace(
            "To report a security issue, please contact the Stainless team at security@stainless.com.",
            "To report a security issue in this SDK, please contact the maintainers listed under Terms and Policies below.",
        )
        .replace(
            "${{ startsWith(github.repository, 'stainless-sdks/') && 'depot-ubuntu-24.04' || 'ubuntu-latest' }}",
            "ubuntu-latest",
        );
    if path == ".github/workflows/ci.yml"
        && let Some(start) = text.find("\n      - name: Get GitHub OIDC Token\n")
        && let Some(end) = text.find("        run: ./scripts/upload-artifacts\n")
    {
        let end = end + "        run: ./scripts/upload-artifacts\n".len();
        text.replace_range(start..end, "\n");
    }
    text
}

/// Every file under `dir`, relative and `/`-separated, without `.git`.
pub fn walk(dir: &Path) -> Vec<String> {
    fn go(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).expect("readable") {
            let e = e.expect("entry");
            let p = e.path();
            if p.is_dir() {
                if e.file_name() != ".git" {
                    go(root, &p, out);
                }
            } else {
                out.push(
                    p.strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    go(dir, dir, &mut out);
    out.sort();
    out
}

/// Writes `files` under `$<var>` when that variable is set (a debugging aid).
pub fn dump(var: &str, files: &[(String, String)]) {
    if let Some(dir) = std::env::var_os(var) {
        for (path, text) in files {
            let p = Path::new(&dir).join(path);
            std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            std::fs::write(p, text).expect("write");
        }
    }
}
