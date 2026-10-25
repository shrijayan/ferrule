//! `ferrule-gen generate` end to end: files, errors and exit codes, as the Forge CLI gives them.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ferrule-gen"))
}

fn plugin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../python-plugin")
}

/// A fresh directory outside the repository, so the generated Python never meets the repo's
/// ruff and pyright runs; removed again when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ferrule-gen-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch directory");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

fn scratch(name: &str) -> Scratch {
    Scratch::new(name)
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
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .arg("--out")
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    assert_eq!(files(&out), files(&plugin_dir().join("tests/golden/cursor-page")));
}

#[test]
fn reports_plugin_errors_on_stderr_with_exit_code_1() {
    let dir = scratch("bad");
    let fixture = fs::read_to_string(plugin_dir().join("tests/fixtures/cursor-page.json")).expect("fixture");
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

#[test]
fn generates_python_for_an_explicit_python_target() {
    let out = scratch("target-python");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .args(["--target", "python", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    assert_eq!(files(&out), files(&plugin_dir().join("tests/golden/cursor-page")));
}

#[test]
fn reports_targets_without_a_plugin_as_fer202() {
    let dir = scratch("target-go");
    let output = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .args(["--target=go", "--out"])
        .arg(dir.join("out"))
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("ferrule-gen: FER202 at --target: target go is not supported yet. Fix: "),
        "{stderr}"
    );
    assert!(!dir.join("out").exists());
}

#[test]
fn generates_a_typescript_sdk() {
    let out = scratch("target-ts");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .args(["--target", "typescript", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    let written: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for path in [
        "src/client.ts",
        "src/resources/index.ts",
        "api.md",
        "package.json",
        "README.md",
    ] {
        assert!(written.iter().any(|p| p == path), "{path} missing in {written:?}");
    }
}

#[test]
fn generates_a_node_layout_sdk() {
    let out = scratch("target-node");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .args(["--target", "node", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    let written: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for path in [
        "src/index.ts",
        "src/resources/index.ts",
        "api.md",
        ".eslintrc.js",
        "scripts/mock",
    ] {
        assert!(written.iter().any(|p| p == path), "{path} missing in {written:?}");
    }
    assert!(!written.iter().any(|p| p == "src/client.ts"), "{written:?}");
}

/// `package.json` runs `./scripts/test`, so the scripts must be executable, as in the oracles.
#[cfg(unix)]
#[test]
fn typescript_scripts_are_executable() {
    use std::os::unix::fs::PermissionsExt;
    let out = scratch("target-ts-modes");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .args(["--target", "typescript", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    let mode = |p: &str| fs::metadata(out.join(p)).expect("written").permissions().mode();
    for p in ["scripts/test", "scripts/build", "scripts/utils/git-swap.sh"] {
        assert_eq!(mode(p) & 0o111, 0o111, "{p} is not executable");
    }
    assert_eq!(mode("package.json") & 0o111, 0, "package.json is executable");
}

#[test]
fn reports_unknown_targets_as_usage_errors() {
    let output = bin()
        .args(["generate", "a.json", "--out", "x", "--target", "cobol"])
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("possible values: python"));
}

fn model(spec: &Path) -> std::process::Output {
    bin().arg("model").arg(spec).output().expect("runs")
}

#[test]
fn prints_the_neutral_model_deterministically() {
    let spec = plugin_dir().join("tests/fixtures/cursor-page.json");
    let first = model(&spec);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert_eq!(first.stdout, model(&spec).stdout);
    let text = String::from_utf8(first.stdout).expect("utf-8");
    assert!(text.ends_with("}\n"));
    let json: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    let method = &json["resources"][0]["methods"][0];
    assert_eq!(method["name"]["written"], "list");
    assert_eq!(method["pagination"]["scheme"], "cursor");
}

#[test]
fn reads_the_model_spec_from_stdin() {
    use std::io::Write as _;
    let path = plugin_dir().join("tests/fixtures/cursor-page.json");
    let mut child = bin()
        .args(["model", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&fs::read(&path).expect("fixture"))
        .expect("write");
    let output = child.wait_with_output().expect("exits");
    assert!(output.status.success());
    assert_eq!(output.stdout, model(&path).stdout);
}

#[test]
fn reports_model_errors_with_exit_code_1() {
    let dir = scratch("model-bad");
    let fixture = fs::read_to_string(plugin_dir().join("tests/fixtures/cursor-page.json")).expect("fixture");
    let bad = fixture.replacen("\"x-ferrule-method-name\": \"list\"", "\"x-ferrule-method-name\": 3", 1);
    let spec = dir.join("bad.json");
    fs::write(&spec, bad).expect("write");
    let output = model(&spec);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("ferrule-gen: FER101 at #/paths/~1v1~1items/get/x-ferrule-method-name: "),
        "{stderr}"
    );
    assert_eq!(model(Path::new("/nonexistent/spec.json")).status.code(), Some(1));
    assert_eq!(bin().arg("model").output().expect("runs").status.code(), Some(2));
}

#[test]
fn generates_go_with_the_runtime_options() {
    let out = scratch("target-go");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
        .args(["--target", "go", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    let names: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for expected in [
        "client.go",
        "agency.go",
        "agency_test.go",
        "shared/shared.go",
        "go.mod",
        ".ferrule/go-runtime.json",
    ] {
        assert!(names.iter().any(|n| n == expected), "{expected} not in {names:?}");
    }
    let options: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join(".ferrule/go-runtime.json")).expect("options")).expect("JSON");
    assert_eq!(options["package_name"], "onebusaway");
    assert!(
        options["client_options"]
            .as_str()
            .is_some_and(|s| s.contains("func WithAPIKey("))
    );
}
