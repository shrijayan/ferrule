//! The proof that replaces "byte-identical to Stainless" for the Rust target (ADR 0019): each
//! golden crate is copied out of the workspace, with its fixture's `Cargo.lock`, and must
//!
//! 1. be laid out as rustfmt lays it out (`cargo fmt --check`);
//! 2. build and lint clean with every feature (`cargo clippy --all-targets --all-features
//!    -- -D warnings`);
//! 3. pass its own tests: the runtime's unit tests, every method against the fake server, the
//!    blocking client and the README's example (`cargo test --all-features`);
//! 4. have a public API that `cargo-semver-checks` accepts against itself as the baseline, the
//!    check a release of the crate runs against the previous one.
//!
//! The crate is checked in a temporary directory, outside this workspace, with the toolchain
//! `rust-toolchain.toml` pins; builds share `target/rust-plugin-tests/target`. It needs the
//! crates.io dependencies (fetched once into the shared target directory);
//! `FERRULE_SKIP_SDK_BUILD=1` skips it offline, and step 4 is skipped with a note when
//! `cargo-semver-checks` is not installed (`mise install` installs it).

mod common;

use std::path::Path;
use std::process::Command;

use common::{FIXTURES, dir, root, walk};

/// The channel `rust-toolchain.toml` pins, for the cargo subcommands rustup resolves by
/// directory.
fn toolchain() -> String {
    let file = std::fs::read_to_string(root().join("rust-toolchain.toml")).expect("rust-toolchain.toml");
    file.lines()
        .find_map(|l| l.trim().strip_prefix("channel = "))
        .map(|c| c.trim_matches('"').to_owned())
        .expect("a channel")
}

/// An empty directory outside the workspace, so that cargo does not take the crate for one of
/// its members.
fn outside(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("ferrule-rust-plugin-tests").join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clears the directory");
    }
    std::fs::create_dir_all(&dir).expect("creates the directory");
    dir
}

fn cargo(crate_dir: &Path, args: &[&str]) {
    let target = root().join("target/rust-plugin-tests/target");
    let output = Command::new(env!("CARGO"))
        .args(args)
        .current_dir(crate_dir)
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTUP_TOOLCHAIN", toolchain())
        // The generated crate is not part of this workspace and must not inherit its settings.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo {} failed in {}:\n{}\n{}",
        args.join(" "),
        crate_dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn semver_checks_installed() -> bool {
    Command::new(env!("CARGO"))
        .args(["semver-checks", "--version"])
        .env("RUSTUP_TOOLCHAIN", toolchain())
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn golden_crates_build_lint_clean_and_pass_their_tests() {
    if std::env::var_os("FERRULE_SKIP_SDK_BUILD").is_some() {
        eprintln!("skipped: FERRULE_SKIP_SDK_BUILD is set");
        return;
    }
    for fixture in FIXTURES {
        let golden = dir(fixture).join("golden");
        let work = outside(fixture);
        for path in walk(&golden) {
            let to = work.join(&path);
            std::fs::create_dir_all(to.parent().expect("a parent")).expect("creates the directory");
            std::fs::copy(golden.join(&path), to).expect("copies the golden file");
        }
        std::fs::copy(dir(fixture).join("Cargo.lock"), work.join("Cargo.lock")).expect("copies the lockfile");
        cargo(&work, &["fmt", "--check"]);
        cargo(
            &work,
            &[
                "clippy",
                "--locked",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        );
        cargo(&work, &["test", "--locked", "--all-features"]);
        if semver_checks_installed() {
            let baseline = work.to_str().expect("a UTF-8 path");
            cargo(&work, &["semver-checks", "--baseline-root", baseline]);
        } else {
            eprintln!("skipped cargo-semver-checks for {fixture}: not installed (mise install)");
        }
    }
}
