---

status: Proposed
date: 2026-10-26
deciders: Ferrule maintainers
---

# 0021. Install Ferrule with curl, release it as one version

Amends the distribution part of [ADR 0020](./0020-one-command-generate-in-ferrule-gen.md).

## Context and problem statement

ADR 0020 made `ferrule-gen generate --spec --config --out` run the whole Python pipeline, so a
user needs one native binary and `ruff`, not a Python environment. The site still leads with
`uvx ferrule generate`, and the `ferrule` wheel cannot run without a `ferrule-gen` it does not
ship. Releases are two release-please components, `ferrule` and `ferrule-runtime`, each with its
own tag (`ferrule-v0.1.0`, `ferrule-runtime-v0.1.0`) and its own GitHub release, although they
are built from the same commit and only make sense together.

How do users install Ferrule, and what is a release?

## Decision drivers

- One command to install, like ruff and uv: `curl ... | sh`, no Python, no account.
- One version for everything a commit ships, so "Ferrule 0.2.0" means one thing.
- The release workflow stays release-please driven (one PR per release, changelog from commits).
- Every published artifact keeps a checksum and build provenance.
- `adopt`, `rescue`, `verify` and `merge` are Python today (griffe, libcst); users of them must
  not lose them while they are ported.

## Considered options

1. Keep `uvx ferrule` as the entry point; ship `ferrule-gen` as a binary wheel (ADR 0020's plan).
2. Ship `ferrule-gen` as release archives with a `curl | sh` installer served from the site,
   keep the Python CLI for the commands not yet in Rust, and remove it once they are.
3. Option 2, but port every command to Rust before shipping the installer.

## Decision outcome

Chosen option 2, because `generate` already works natively and the other commands can follow
one at a time.

### Install

```bash
curl -LsSf https://ferrule.cpluz.com/install.sh | sh
```

- `site/public/install.sh` is a POSIX `sh` script. It picks the archive for `uname -s` and
  `uname -m`, downloads it and its `.sha256` from the GitHub release, checks the digest, and
  installs `ferrule-gen` to `$FERRULE_INSTALL_DIR` (default `~/.local/bin`).
  `FERRULE_VERSION=vX.Y.Z` installs a given release; the default is the latest.
- Archives: `ferrule-gen-<target>.tar.gz` for `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, and
  `ferrule-gen-x86_64-pc-windows-msvc.zip`, each with `LICENSE` and `NOTICE`. Linux builds run on
  Ubuntu 22.04 so they need glibc 2.35 or newer. Windows users unzip the archive; a PowerShell
  installer can come later.
- The workflow builds them itself (`cargo build --release --locked`) rather than with
  cargo-dist, which wants to own tagging and the release, which release-please already does.
- Each archive gets a build provenance attestation; the install script checks its sha256.
- `ruff` stays an external tool (ADR 0020). The installer says how to get it when it is missing.
- The binary keeps the name `ferrule-gen` while the Python `ferrule` command exists, so the two
  never shadow each other on `PATH`. It becomes `ferrule` when the Python CLI is removed.

### One version

- release-please has one component, the repository root, with no component in the tag: one
  `vX.Y.Z` tag, one GitHub release titled `vX.Y.Z`, one `CHANGELOG.md`.
- The release PR bumps every version in step: the Cargo workspace version (and the `ferrule-*`
  entries in `Cargo.lock`), both `pyproject.toml` files, and their `uv.lock` entries, so
  `--locked` and `--frozen` builds keep passing after a release.
- The `ferrule-gen-v…`, `ferrule-v…` and `ferrule-runtime-v…` tags are not used. The 0.1.0
  pair is replaced by a single `v0.1.0` at the same commit.
- While the Python CLI exists, the same release also publishes `ferrule` and `ferrule-runtime`
  to PyPI at that version.

### Retiring the Python CLI

Each command moves to `ferrule-gen` in its own change, with a test that the two give the same
result, and the docs switch to the binary for it: `generate --target` for the other languages,
`rescue`, `verify` (needs a Rust Python-API reader to replace griffe), `merge` (replace
libcst), then `adopt`. When none is left, `migrate/` and the PyPI publishing are removed and the
binary is renamed `ferrule`. `ferrule-runtime` stays a workspace package for the runtime tests,
but is no longer published.

### Consequences

- Good: installing needs `curl` and `sh`; generating a Python SDK needs the binary and ruff.
- Good: one version, one tag, one release per commit, readable at a glance.
- Good: the Python CLI keeps working for adopt and the others during the move.
- Bad: two entry points (`ferrule-gen`, `ferrule`) until the port is done; the docs say which
  command needs which.
- Bad: `ferrule` and `ferrule-runtime` get a new version even when only Rust changed.
- Bad: binary builds for five targets add about ten minutes to a release.

## More information

- ADR 0014 (Rust for the converter and plugin), ADR 0020 (one-command generate).
- `release-please-config.json`, `.github/workflows/release.yml`, `site/public/install.sh`.
