//! Test inputs: annotated specs from the converter and the neutral model from them, and the
//! `syntax_tree` formatter the gem's `rake format` runs on the `.rbi` and `.rbs` views. This is the
//! only place in the crate that names Forge (to parse the annotated spec for
//! `ferrule_sdk_model::forge_to_model`); `src/` never does (see `boundary.rs`).

// Each test binary that includes this module uses only part of it.
#![allow(dead_code)]

use std::fmt::Write as _;
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

/// An oracle gem checkout, or `None` (with a note) when it is not fetched.
pub fn oracle(name: &str) -> Option<PathBuf> {
    let dir = root().join("oracles/cache").join(name);
    if dir.join("Gemfile").is_file() {
        Some(dir)
    } else {
        eprintln!("skipped: oracles/cache/{name} is not fetched");
        None
    }
}

/// A formatter's command: `$<var>` (words split on spaces, such as `bundle exec stree`), else
/// `name` on the `PATH`; `None` when neither is there.
pub fn tool(var: &str, name: &str) -> Option<Vec<String>> {
    if let Some(cmd) = std::env::var_os(var) {
        let words: Vec<String> = cmd.to_string_lossy().split_whitespace().map(str::to_owned).collect();
        return (!words.is_empty()).then_some(words);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
            .map(|p| vec![p.to_string_lossy().into_owned()])
    })
}

/// `syntax_tree`'s `stree` (`$FERRULE_STREE`) and rubocop (`$FERRULE_RUBOCOP`), if both are there.
pub fn formatters() -> Option<(Vec<String>, Vec<String>)> {
    Some((tool("FERRULE_STREE", "stree")?, tool("FERRULE_RUBOCOP", "rubocop")?))
}

const MARK: &str = "ferrule-rbs-alias";

/// The Rakefile's `format:rbs` workaround: `syntax_tree` cannot parse `class X = Y`, so such lines
/// become a marker comment and a constant `X: Y`, and come back after formatting.
fn hide_aliases(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        let ws = &body[..body.len() - body.trim_start().len()];
        let rest = body.trim_start();
        let alias = ["class ", "module "].iter().find_map(|k| {
            let after = rest.strip_prefix(k)?;
            let (name, value) = after.split_once(" = ")?;
            (!name.contains(' ')).then(|| (k.trim_end(), name, value))
        });
        if let Some((kind, name, value)) = alias {
            let _ = write!(out, "{ws}# {kind} {MARK}\n{ws}{name}: {value}\n");
        } else {
            out.push_str(line);
        }
    }
    out
}

fn restore_aliases(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        let t = l.trim_start();
        let ws = &l[..l.len() - t.len()];
        let kind = ["class", "module"].into_iter().find(|k| t == format!("# {k} {MARK}"));
        if let (Some(kind), Some(next)) = (kind, lines.get(i + 1))
            && let Some((name, value)) = next.trim_start().split_once(": ")
        {
            out.push(format!("{ws}{kind} {name} = {value}"));
            i += 2;
            continue;
        }
        out.push(l.to_owned());
        i += 1;
    }
    out.join("\n")
}

static RUN: AtomicUsize = AtomicUsize::new(0);

/// The file's extension: `rb`, `rbi` or `rbs`.
fn ext(path: &str) -> &str {
    Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("")
}

fn run(cmd: &[String], dir: &Path, args: &[&str], paths: &[PathBuf]) {
    if paths.is_empty() {
        return;
    }
    let out = Command::new(&cmd[0])
        .args(&cmd[1..])
        .args(args)
        .arg("--")
        .args(paths)
        .current_dir(dir)
        // As the Rakefile's `format:*` tasks run them.
        .env("RUBYOPT", "--encoding=UTF-8")
        .output()
        .expect("the formatter runs");
    assert!(
        out.status.success(),
        "{}: {}{}",
        cmd.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `files` laid out as the gem's `rake format` lays out a gem: `rubocop -a` on the `.rb` files
/// (with the generated `.rubocop.yml`), `stree write` on the `.rbi` files and, with the RBS plugin,
/// on the `.rbs` files. `None` without both formatters.
pub fn format(files: &[(String, String)]) -> Option<Vec<(String, String)>> {
    let (stree, rubocop) = formatters()?;
    let dir = std::env::temp_dir().join(format!(
        "ferrule-ruby-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::SeqCst)
    ));
    let (mut rb, mut rbi, mut rbs) = (Vec::new(), Vec::new(), Vec::new());
    for (path, text) in files {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        if ext(path) == "rbs" {
            std::fs::write(&p, hide_aliases(text)).expect("write");
            rbs.push(PathBuf::from(path));
        } else {
            std::fs::write(&p, text).expect("write");
            if ext(path) == "rbi" {
                rbi.push(PathBuf::from(path));
            } else if ext(path) == "rb" {
                rb.push(PathBuf::from(path));
            }
        }
    }
    run(
        &rubocop,
        &dir,
        &["--fail-level", "F", "--autocorrect", "--format", "quiet"],
        &rb,
    );
    run(&stree, &dir, &["write"], &rbi);
    run(&stree, &dir, &["write", "--plugin=rbs"], &rbs);
    let formatted = files
        .iter()
        .map(|(path, _)| {
            let text = std::fs::read_to_string(dir.join(path)).expect("read");
            let text = if ext(path) == "rbs" {
                restore_aliases(&text)
            } else {
                text
            };
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

/// Ferrule's branding of the oracle's text (`crates/python-plugin/src/emit/files/BRANDING.md`):
/// the `.keep` files.
pub fn brand(text: &str) -> String {
    text.replace(
        "File generated from our OpenAPI spec by Stainless.\n",
        "File generated from our OpenAPI spec by Ferrule.\n",
    )
    .replace(
        "It is ignored by Stainless code generation",
        "It is ignored by Ferrule code generation",
    )
}
