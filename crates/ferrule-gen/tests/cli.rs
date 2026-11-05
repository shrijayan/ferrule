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
    let dir = scratch("target-terraform");
    let output = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/cursor-page.json"))
        .args(["--target=terraform", "--out"])
        .arg(dir.join("out"))
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("ferrule-gen: FER202 at --target: target terraform is not supported yet. Fix: "),
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
fn generates_a_rust_crate_with_its_runtime() {
    let dir = scratch("target-rust");
    let config = dir.join("ferrule.yml");
    fs::write(&config, "targets:\n  rust:\n    crate_name: oba\n    version: 2.1.0\n").expect("config");
    let out = dir.join("sdk");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
        .args(["--target", "rust", "--out"])
        .arg(&out)
        .arg("--ferrule-config")
        .arg(&config)
        .status()
        .expect("runs");
    assert!(status.success());
    let names: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for expected in [
        "Cargo.toml",
        "src/lib.rs",
        "src/client.rs",
        "src/resources/agency.rs",
        "src/rt/http.rs",
        "src/testing/routes.rs",
        "tests/api.rs",
    ] {
        assert!(names.iter().any(|n| n == expected), "{expected} not in {names:?}");
    }
    let manifest = fs::read_to_string(out.join("Cargo.toml")).expect("manifest");
    assert!(
        manifest.starts_with("[package]\nname = \"oba\"\nversion = \"2.1.0\"\n"),
        "{manifest}"
    );
}

#[test]
fn reports_bad_rust_settings_as_fer101() {
    let dir = scratch("target-rust-bad");
    let config = dir.join("ferrule.yml");
    fs::write(&config, "targets:\n  rust:\n    edition: '2021'\n").expect("config");
    let output = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
        .args(["--target", "rust", "--out"])
        .arg(dir.join("sdk"))
        .arg("--ferrule-config")
        .arg(&config)
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("ferrule-gen: FER101 at /targets/rust/edition"),
        "{stderr}"
    );
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

#[test]
fn generates_java_and_kotlin_with_the_runtime_options() {
    for (target, artifact) in [("java", "onebusaway-sdk-java"), ("kotlin", "onebusaway-sdk-kotlin")] {
        let out = scratch(&format!("target-{target}"));
        let status = bin()
            .arg("generate")
            .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
            .args(["--target", target, "--out"])
            .arg(&*out)
            .status()
            .expect("runs");
        assert!(status.success());
        let names: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
        // The fixture has no `x-ferrule-targets`, so the package falls back to `com.<organization>`.
        for expected in [
            format!("{artifact}-core/src/main/kotlin/com/onebusawaysdk/client/OnebusawaySdkClient.kt"),
            format!("{artifact}-core/src/main/kotlin/com/onebusawaysdk/core/ClientOptions.kt"),
            "settings.gradle.kts".to_owned(),
            ".ferrule/jvm-runtime.json".to_owned(),
        ] {
            assert!(names.contains(&expected), "{expected} not in {names:?}");
        }
        let options: serde_json::Value =
            serde_json::from_slice(&fs::read(out.join(".ferrule/jvm-runtime.json")).expect("options")).expect("JSON");
        assert_eq!(options["package_name"], "com.onebusawaysdk");
        assert_eq!(options["target"], target);
        assert_eq!(options["artifact_name"], artifact);
    }
}

#[test]
fn generates_ruby_with_the_runtime_options() {
    let out = scratch("target-ruby");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
        .args(["--target", "ruby", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    // The fixture has no `x-ferrule-targets`, so the gem is named after `x-ferrule-package`.
    let names: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for expected in [
        "lib/onebusaway.rb",
        "lib/onebusaway/client.rb",
        "rbi/onebusaway/resources/agency.rbi",
        "sig/onebusaway/models/references.rbs",
        "test/onebusaway/client_test.rb",
        ".rubocop.yml",
        ".ferrule/ruby-runtime.json",
    ] {
        assert!(names.iter().any(|n| n == expected), "{expected} not in {names:?}");
    }
    let options: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join(".ferrule/ruby-runtime.json")).expect("options")).expect("JSON");
    assert_eq!(options["gem_name"], "onebusaway");
    assert_eq!(options["module_name"], "Onebusaway");
    assert_eq!(options["auth_in"], "query");
}

#[test]
fn generates_php_with_the_runtime_options() {
    let out = scratch("target-php");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
        .args(["--target", "php", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    // The fixture has no `x-ferrule-targets`, so the namespace is named after `x-ferrule-package`.
    let names: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for expected in [
        "src/Client.php",
        "src/Version.php",
        "src/Services/AgencyService.php",
        "src/ServiceContracts/AgencyRawContract.php",
        "tests/ClientTest.php",
        "composer.json",
        ".php-cs-fixer.dist.php",
        ".ferrule/php-runtime.json",
    ] {
        assert!(names.iter().any(|n| n == expected), "{expected} not in {names:?}");
    }
    let options: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join(".ferrule/php-runtime.json")).expect("options")).expect("JSON");
    assert_eq!(options["namespace"], "Onebusaway");
    assert_eq!(options["max_retries"], 2);
}

#[test]
fn generates_csharp_with_the_runtime_options() {
    let out = scratch("target-csharp");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/fixtures/onebusaway.json"))
        .args(["--target", "csharp", "--out"])
        .arg(&*out)
        .status()
        .expect("runs");
    assert!(status.success());
    // The fixture has no `x-ferrule-targets`, so the namespace is named after the client.
    let names: Vec<String> = files(&out).into_iter().map(|(p, _)| p).collect();
    for expected in [
        "src/OnebusawaySdk/OnebusawaySdkClient.cs",
        "src/OnebusawaySdk/Services/IAgencyService.cs",
        "src/OnebusawaySdk.Tests/TestBase.cs",
        "OnebusawaySdk.sln",
        ".ferrule/csharp-runtime.json",
    ] {
        assert!(names.iter().any(|n| n == expected), "{expected} not in {names:?}");
    }
    let options: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join(".ferrule/csharp-runtime.json")).expect("options")).expect("JSON");
    assert_eq!(options["namespace"], "OnebusawaySdk");
    assert_eq!(options["client_name"], "OnebusawaySdk");
}

fn repo_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A `ruff` stand-in that appends its arguments to `ruff.log` next to it and exits with `code`.
#[cfg(unix)]
fn fake_ruff(dir: &Path, code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("ruff");
    let log = dir.join("ruff.log");
    let script = format!(
        "#!/bin/sh\necho \"$@\" >> '{}'\necho 'ruff: boom' >&2\nexit {code}\n",
        log.display()
    );
    fs::write(&path, script).expect("fake ruff");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

/// `generate --spec` on the environments fixture, with `ruff` replaced by [`fake_ruff`].
#[cfg(unix)]
fn one_command(tools: &Path, out: &Path, ruff: &Path) -> std::process::Output {
    let fixture = plugin_dir().join("tests/environments");
    bin()
        .args(["generate", "--spec"])
        .arg(fixture.join("openapi.yml"))
        .arg("--config")
        .arg(fixture.join("stainless.yml"))
        .arg("--out")
        .arg(out)
        .env("FERRULE_RUFF", ruff)
        .current_dir(tools)
        .output()
        .expect("runs")
}

#[cfg(unix)]
#[test]
fn one_command_writes_the_sdk_with_the_runtime_python_renders() {
    let tools = scratch("one-command-tools");
    let out = tools.join("sdk");
    let ruff = fake_ruff(&tools, 0);
    let run = one_command(&tools, &out, &ruff);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));

    // The plugin's files are the annotated-spec form's files.
    let annotated = tools.join("annotated");
    let status = bin()
        .arg("generate")
        .arg(plugin_dir().join("tests/environments/annotated.json"))
        .arg("--out")
        .arg(&annotated)
        .status()
        .expect("runs");
    assert!(status.success());
    let all = files(&out);
    for (path, bytes) in files(&annotated) {
        assert!(all.contains(&(path.clone(), bytes)), "{path} differs");
    }

    // The runtime is byte for byte what `ferrule_runtime` renders for the package.
    let package = fs::read_dir(out.join("src"))
        .expect("src")
        .filter_map(Result::ok)
        .find(|e| e.path().join("__init__.py").is_file())
        .and_then(|e| e.file_name().into_string().ok())
        .expect("one package");
    let python = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(
            "import json, sys\n\
             sys.path.insert(0, sys.argv[1])\n\
             import ferrule_runtime as r\n\
             print(json.dumps(r.render_files(sys.argv[2], r.default_client_name(sys.argv[2]))))",
        )
        .arg(repo_dir().join("runtime/src"))
        .arg(&package)
        .output()
        .expect("python3 runs");
    assert!(python.status.success(), "{}", String::from_utf8_lossy(&python.stderr));
    let expected: std::collections::BTreeMap<String, String> = serde_json::from_slice(&python.stdout).expect("JSON");
    assert!(!expected.is_empty());
    for (rel, text) in &expected {
        let written = fs::read_to_string(out.join("src").join(&package).join(rel)).expect(rel);
        assert_eq!(&written, text, "{rel}");
    }
    let runtime_new = expected
        .keys()
        .filter(|rel| {
            !files(&annotated)
                .iter()
                .any(|(p, _)| *p == format!("src/{package}/{rel}"))
        })
        .count();

    // ruff sorts imports, then formats, over the whole output.
    let log = fs::read_to_string(tools.join("ruff.log")).expect("ruff ran");
    let o = out.display();
    assert_eq!(
        log,
        format!("check --select I --fix --no-cache --quiet --exit-zero {o}\nformat --no-cache --quiet {o}\n")
    );
    let summary = format!(
        "generated {package}: {} files ({runtime_new} runtime) sha256 ",
        all.len()
    );
    assert!(stdout.starts_with(&summary), "{stdout}");
    assert_eq!(stdout.trim_end().len(), summary.len() + 64, "{stdout}");
}

#[cfg(unix)]
#[test]
fn one_command_is_deterministic() {
    let tools = scratch("one-command-twice");
    let ruff = fake_ruff(&tools, 0);
    let first = one_command(&tools, &tools.join("a"), &ruff);
    let second = one_command(&tools, &tools.join("b"), &ruff);
    assert!(first.status.success() && second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(files(&tools.join("a")), files(&tools.join("b")));
}

#[cfg(unix)]
#[test]
fn one_command_reports_failures_with_fer_codes() {
    let tools = scratch("one-command-errors");
    let fixture = plugin_dir().join("tests/environments");
    let stderr = |run: &std::process::Output| {
        assert_eq!(run.status.code(), Some(1));
        String::from_utf8_lossy(&run.stderr).into_owned()
    };

    let ruff = fake_ruff(&tools, 2);
    let err = stderr(&one_command(&tools, &tools.join("sdk"), &ruff));
    assert!(
        err.contains("FER4003 at sort-imports: sort-imports failed: ruff: boom"),
        "{err}"
    );

    let err = stderr(&one_command(&tools, &tools.join("sdk"), &ruff));
    assert!(err.contains("FER4006 at --out:"), "{err}");

    let err = stderr(&one_command(&tools, &tools.join("other"), &tools.join("no-ruff")));
    assert!(
        err.contains("FER4002 at FERRULE_RUFF:") && err.contains("astral.sh/ruff"),
        "{err}"
    );

    let run = bin()
        .args(["generate", "--spec", "missing.yml", "--config"])
        .arg(fixture.join("stainless.yml"))
        .args(["--out", "sdk"])
        .current_dir(&*tools)
        .output()
        .expect("runs");
    assert!(stderr(&run).contains("FER4001 at --spec: missing.yml does not exist"));

    let bad = tools.join("bad.yml");
    fs::write(&bad, "organization: {}\nresources: 3\n").expect("config");
    let run = bin()
        .args(["generate", "--spec"])
        .arg(fixture.join("openapi.yml"))
        .arg("--config")
        .arg(&bad)
        .arg("--out")
        .arg(tools.join("bad-sdk"))
        .env("FERRULE_RUFF", fake_ruff(&tools, 0))
        .output()
        .expect("runs");
    let err = stderr(&run);
    assert!(err.contains("FER4003 at convert: convert failed: error"), "{err}");
}
