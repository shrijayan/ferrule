//! `ferrule-gen generate` end to end: files, errors and exit codes, as the Forge CLI gives them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ferrule-gen"))
}

fn plugin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python-plugin")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for e in fs::read_dir(dir).expect("directory") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .expect("under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, fs::read(&p).expect("file")));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn generates_the_golden_tree() {
    // cursor-page has no x-ferrule-client, so it has no project files or agent context.
    let out = scratch("cursor-page");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("test/fixtures/cursor-page.json"))
        .arg("--out")
        .arg(&out)
        .status()
        .expect("runs");
    assert!(status.success());
    assert_eq!(files(&out), files(&plugin_dir().join("test/golden/cursor-page")));
}

#[test]
fn reports_plugin_errors_on_stderr_with_exit_code_1() {
    let dir = scratch("bad");
    let fixture = fs::read_to_string(plugin_dir().join("test/fixtures/cursor-page.json")).expect("fixture");
    let bad = fixture.replacen("\"x-ferrule-method-name\": \"list\"", "\"x-ferrule-method-name\": 3", 1);
    assert_ne!(bad, fixture);
    let spec = dir.join("bad.json");
    fs::write(&spec, bad).expect("write");
    let output = bin()
        .arg("generate")
        .arg(&spec)
        .arg("--out")
        .arg(dir.join("out"))
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("ferrule-gen: FER101 at #/paths/~1v1~1items/get/x-ferrule-method-name: "),
        "{stderr}"
    );
    assert!(!dir.join("out").exists());
}

#[test]
fn reports_unreadable_specs_with_exit_code_1() {
    let output = bin()
        .args(["generate", "/nonexistent/spec.json", "--out", "x"])
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("ferrule-gen: /nonexistent/spec.json: "));
}

#[test]
fn reports_usage_errors_with_exit_code_2() {
    let output = bin().arg("generate").output().expect("runs");
    assert_eq!(output.status.code(), Some(2));
}
