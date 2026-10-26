---

status: Proposed
date: 2026-10-26
deciders: Ferrule maintainers
---

# 0020. Generate a Python SDK with `ferrule-gen` alone

Amends [ADR 0014](./0014-rust-for-converter-and-plugin.md). `migrate/` stays Python.

## Context and problem statement

`ferrule generate --spec openapi.yml --config stainless.yml --out sdk` is the command the docs
lead with. It is a Python CLI that runs `ferrule-gen convert`, then `ferrule-gen generate`,
then renders the runtime template with `ferrule_runtime.render`, then runs `ruff` to sort
imports and format. The `ferrule` wheel does not contain `ferrule-gen`: it looks for
`$FERRULE_GEN` or `<repo>/target/release/ferrule-gen`, so `uvx ferrule generate` fails on any
machine that has not built the workspace.

Everything except the runtime render and the ruff calls is already in the Rust binary. The
render `ferrule generate` uses is small: it walks `runtime/template`, resolves `#ferrule:if`
directives and replaces eight tokens, all with default options. Should one native binary do the
whole Python pipeline, and how do users get it?

## Decision drivers

- One download to generate an SDK: no Python environment just to run the generator.
- `curl | sh`, Homebrew, `cargo install` and `uvx` should all work, as they do for ruff and uv.
- "Runtime written once": the template stays in `runtime/template`, the only copy.
- "Deterministic output": both paths must give byte-identical trees.
- griffe and libcst (verify, merge, adopt) have no Rust equivalent; ADR 0014's reasons hold.

## Considered options

1. Keep the pipeline in Python; ship `ferrule-gen` as a binary wheel the `ferrule` wheel depends on.
2. Move the Python pipeline into `ferrule-gen` (convert, Forge, runtime render, ruff), with
   the template embedded at build time; ship the binary on its own and as a binary wheel.
3. Option 2 and also replace `ruff` with formatted output from the emitters.

## Decision outcome

Chosen option 2. `ferrule-gen generate --spec <openapi> --config <stainless.yml> --out <dir>`
runs the whole Python pipeline and prints the same summary line as `ferrule generate`, digest
included. The annotated-spec form (`ferrule-gen generate <annotated.json> --out <dir>`) stays,
for `ferrule generate --target ...`, adopt and the golden tests.

- `crates/ferrule-gen/build.rs` embeds every file of `runtime/template` (with the same skips as
  `ferrule_runtime`'s walk), so the template is still written once and the binary carries it.
- `crates/ferrule-gen/src/runtime.rs` ports the part of `_render.py` the default render uses:
  directives, tokens, leftover-token and collision checks. A test in `migrate/tests` compares its
  output with `ferrule_runtime.render` on the onebusaway oracle, so the two engines cannot drift
  without a failing test.
- `ruff` stays an external tool (option 3 is left for later): it is one static binary with its
  own `curl | sh` installer. `$FERRULE_RUFF` overrides it, as in `ferrule generate`.
- Only the Python target, without `--runtime-from`, runs in one command. Other targets, and
  adopt mode, still go through `ferrule generate`, which renders their runtimes and runs their
  formatters.
- `ferrule` finds `ferrule-gen` on `PATH` when neither `--gen`, `$FERRULE_GEN` nor a repo
  build is there.

### Distribution

Superseded by [ADR 0021](./0021-curl-install-and-one-version.md): release archives and a
`curl | sh` installer, no binary wheel.

### Consequences

- Good: `ferrule-gen generate --spec --config --out` needs one binary and `ruff`.
- Good: the Python pipeline is about one process start instead of five.
- Bad: two engines render the Python template, Python's for the runtime tests and other
  languages, Rust's for one-command generate; the parity test is what keeps them equal.
- Bad: the binary grows by the size of the template (about 300 KB).

## More information

- [ADR 0014](./0014-rust-for-converter-and-plugin.md): Rust for the converter and the plugin.
- [ADR 0016](./0016-sdk-testing-fake-server.md): the `testing/` fake server, not rendered by default.
- `runtime/src/ferrule_runtime/_render.py`: the engine this ports.
