//! Test inputs: annotated specs from the converter and the neutral model from them, and
//! php-cs-fixer, which the package's `scripts/format` runs. This is the only place in the crate
//! that names Forge (to parse the annotated spec for `ferrule_sdk_model::forge_to_model`); `src/`
//! never does (see `boundary.rs`).

// Each test binary that includes this module uses only part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// The annotated spec of a config oracle in `oracles/cache`, or `None` when it is not fetched.
pub fn annotated(config_dir: &str, spec: &str, config: &str) -> Option<String> {
    let dir = root().join("oracles/cache").join(config_dir);
    let spec = std::fs::read_to_string(dir.join(spec)).ok()?;
    let config = std::fs::read_to_string(dir.join(config)).ok()?;
    Some(
        ferrule_converter::convert_text(&spec, &config)
            .expect("converts")
            .output,
    )
}

/// An oracle package checkout, or `None` (with a note) when it is not fetched.
pub fn oracle(name: &str) -> Option<PathBuf> {
    let dir = root().join("oracles/cache").join(name);
    if dir.join("composer.json").is_file() {
        Some(dir)
    } else {
        eprintln!("skipped: oracles/cache/{name} is not fetched");
        None
    }
}

/// Why the formatter tests skip when [`formatter`] finds nothing.
pub const NOT_INSTALLED: &str = "php-cs-fixer is not installed: run `mise run php:tools` (or set FERRULE_PHP_CS_FIXER)";

/// php-cs-fixer's command: `$FERRULE_PHP_CS_FIXER` (words split on spaces), else
/// `crates/php-plugin/tools/bin/php-cs-fixer` once `mise run php:tools` has installed it into
/// `.cache/php-tools`; `None` when neither is there.
pub fn formatter() -> Option<Vec<String>> {
    if let Some(cmd) = std::env::var_os("FERRULE_PHP_CS_FIXER") {
        let words: Vec<String> = cmd.to_string_lossy().split_whitespace().map(str::to_owned).collect();
        return (!words.is_empty()).then_some(words);
    }
    let bin = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/bin/php-cs-fixer");
    root()
        .join(".cache/php-tools/friendsofphp/php-cs-fixer/php-cs-fixer")
        .is_file()
        .then(|| vec![bin.to_string_lossy().into_owned()])
}

static RUN: AtomicUsize = AtomicUsize::new(0);

/// Writes `files` into a new directory under the system temporary directory.
pub fn write_tree(files: &[(String, String)], tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ferrule-php-{tag}-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::SeqCst)
    ));
    for (path, text) in files {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(&p, text).expect("write");
    }
    // The configuration's finder reads both directories.
    for d in ["src", "tests"] {
        std::fs::create_dir_all(dir.join(d)).expect("mkdir");
    }
    dir
}

/// Runs php-cs-fixer in `dir` (with the package's own `.php-cs-fixer.dist.php` and
/// `composer.json`), as `scripts/format` does.
pub fn fix(dir: &Path) -> Option<()> {
    let cmd = formatter()?;
    let out = Command::new(&cmd[0])
        .args(&cmd[1..])
        .args(["fix", "--quiet", "--using-cache=no"])
        .current_dir(dir)
        .output()
        .expect("php-cs-fixer runs");
    assert!(
        out.status.success(),
        "{}: {}{}",
        cmd.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Some(())
}

/// `files` laid out as the package's `scripts/format` lays them out; `None` without
/// php-cs-fixer.
pub fn format(files: &[(String, String)]) -> Option<Vec<(String, String)>> {
    formatter()?;
    let dir = write_tree(files, "fmt");
    fix(&dir)?;
    let formatted = files
        .iter()
        .map(|(path, _)| {
            let text = std::fs::read_to_string(dir.join(path)).expect("read");
            (path.clone(), text)
        })
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    Some(formatted)
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

/// Writes `files` under `$<var>` when it is set (a debugging aid).
pub fn dump(var: &str, files: &[(String, String)]) {
    if let Some(dir) = std::env::var_os(var) {
        for (path, text) in files {
            let p = Path::new(&dir).join(path);
            std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            std::fs::write(p, text).expect("write");
        }
    }
}
