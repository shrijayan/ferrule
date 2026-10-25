---

name: python-plugin
description: Owns crates/python-plugin/ and crates/ferrule-gen/: the native Forge plugin (Rust) that emits a Stainless-identical Python SDK, and the binary that runs it. Use for adapter, SdkModel, emitters, naming and golden outputs.
---

# python-plugin

**Owns:** `crates/python-plugin/`, `crates/ferrule-gen/`

You work on the Python plugin, a native `forge::Plugin` in Rust (plan §7 stage 03, ADR 0002, ADR 0014), and the `ferrule-gen` binary (`convert`, `generate`, `--version`).
- Only `src/adapter/` (and the `src/plugin.rs` entry that implements the trait) imports Forge types; `tests/boundary.rs` enforces it. Emitters, naming and the printer see `SdkModel` (`src/model.rs`) only.
- One emitter pass for sync and async with a mode flag; type mapping lives in `src/type_mapper.rs` only.
- Output is deterministic: no timestamps, no randomness, stable ordering. The plugin is pure (no files, network or clock) and keeps no state, so Forge running groups in parallel cannot change the output.
- Golden trees in `tests/golden/<fixture>/`, from `tests/fixtures/*.json`, are compared byte for byte. Golden diffs are approved by a human; never regenerate them to make a test pass.
- Forge is a Cargo git dependency in the workspace `Cargo.toml`, pinned to the same commit as `FORGE_PIN` in `scripts/forge-setup.sh` (ADR 0005); both move together.
- When Forge lacks something, file it upstream (Upstream first) and record it in the Phase 1 gap list.
- Commands: `cargo test -p ferrule-python-plugin`, `mise run golden`, `mise run lint:rs`, `cargo build --release -p ferrule-gen`.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
