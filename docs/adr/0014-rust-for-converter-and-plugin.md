---

status: Accepted
date: 2026-10-25
deciders: Ferrule maintainers
---

# 0014. Rust for the converter and the plugin

Amends [ADR 0001](./0001-languages.md). The runtime and the migration tools stay Python.

## Context and problem statement

ADR 0001 chose TypeScript for `converter/` (about 4k lines) and `python-plugin/` (about 11k
lines) because Forge plugins were TypeScript compiled to WebAssembly. Since then we build Forge
from source (ADR 0005), and the source shows Forge is a Rust crate. Plugin execution (`wasm`)
and the CLI (`cli`) are optional features. Its `Plugin` trait has two methods, `generate` and
`generate_group`, and Forge's own TypeScript generator implements that trait natively
(`src/plugin.rs`, `src/bin/forge/ts.rs`). A Rust plugin can depend on Forge through Cargo and
run in the same process.

Today a generation run needs Node, pnpm, jco and a WASM build, and the Python CLI shells out to
Node. Should the converter and the plugin move to Rust, and if so, how does the plugin talk to
Forge?

## Decision drivers

- Forge is a Rust crate with a native `Plugin` trait (`generate`, `generate_group`).
- One native binary: no Node, pnpm, jco or WASM build to generate an SDK.
- A faster pipeline: no WASM boundary and no Node start-up on each run.
- Closer to Forge's own code, which makes upstream issues and PRs easier.
- The golden outputs already pin the expected bytes, so a port can be checked file by file.
- The runtime ships inside every Python SDK, and griffe and libcst have no Rust equivalent;
  those reasons from ADR 0001 still hold.

## Considered options

1. Stay on TypeScript, compiled to WASM against `forge:plugin`.
2. Rust, implementing Forge's native `Plugin` trait, with Forge as a Cargo dependency.
3. Rust, compiled to a WASM component against `forge.wit` with wit-bindgen.

## Decision outcome

Chosen option 2: Rust with the native `Plugin` trait. It removes the most moving parts and runs
the plugin in one process with Forge.

- A Cargo workspace holds three crates:
  - `crates/converter`: stainless.yml to annotated spec, replacing `converter/`.
  - `crates/python-plugin`: the Python emitter, replacing `python-plugin/`.
  - `crates/ferrule-gen`: the binary that runs the converter, then Forge with the plugin.
- Forge is a Cargo git dependency pinned to the same commit as `FORGE_PIN` in
  `scripts/forge-setup.sh` (ADR 0005). One pin, one place to bump it.
- `runtime/` and `migrate/` stay Python.

The native trait is an internal API. It may churn more than the versioned WIT interface. We
accept that risk and contain it three ways:

- the commit pin, so Forge only changes when we bump it;
- the weekly `forge-canary.yml` run against Forge `main`, which shows breakage early;
- the adapter rule from ADR 0002: only an `adapter` module in `crates/python-plugin` imports
  Forge types. Emitters see our `SdkModel` only.

If the trait churns too much, we fall back to option 3. Because of the adapter rule, that
change stays inside `adapter`.

`generate_group` runs groups in parallel. Output must still be byte-identical across runs, so
the plugin must not depend on the order groups finish in.

### Process

- Port golden-first. The converter goes first, then the plugin, one module at a time.
- Pass/fail: every existing golden output is byte-identical, and `mise run parity -- onebusaway`
  reports 0 breaking changes and 1110 tests passing.
- Goldens are never regenerated to make a test pass. A golden change still needs human
  approval and, if it moves away from Stainless output, an ADR.
- The TypeScript packages stay until the Rust version matches. Then `ferrule generate` in
  `migrate/` is pointed at the `ferrule-gen` binary, and the TypeScript packages are deleted.
- `cargo fmt --check`, `cargo clippy` with warnings denied, and `cargo test` join
  `mise run check` and CI.

### Consequences

- Good: users no longer need Node to generate an SDK; one binary does the work.
- Good: no jco or WASM build step, and a faster run.
- Good: the plugin reads like Forge's own code, which helps upstream work.
- Good: one Forge pin for both the CLI build and the plugin.
- Bad: about 15k lines of TypeScript are rewritten.
- Bad: the Rust toolchain becomes a required part of the repo, added through mise.
- Bad: contributors to the converter and plugin need Rust.
- Bad: the npm release of the plugin package goes away; anyone using it must switch to
  `ferrule-gen`.
- Bad: the native trait may break on a Forge bump; see the mitigations above.

## More information

- Plan §9, "Proposed: move to Rust" (`docs/plan.html`).
- [ADR 0001](./0001-languages.md): languages, amended by this ADR.
- [ADR 0002](./0002-adapter-model.md): adapter model; the same rule applies to the Rust crate.
- [ADR 0005](./0005-pin-forge-by-git-commit.md): pin Forge by git commit.
- Forge sources: `src/plugin.rs` (the `Plugin` trait), `packages/forge/plugin/forge.wit`
  (the WIT interface for option 3).
