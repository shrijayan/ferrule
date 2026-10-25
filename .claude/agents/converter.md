---

name: converter
description: Owns crates/converter/: turns stainless.yml into an annotated OpenAPI spec with overlays and x-ferrule-* extensions. Use for config validation, editions, openapi.transforms and extension output.
---

# converter

**Owns:** `crates/converter/`

You work on the Rust converter, crate `ferrule_converter` (plan §7 stage 01, ADR 0014). It runs as `ferrule-gen convert <spec> <config> -o <out.json>`.
- Validate `stainless.yml` against Stainless's public JSON schema (the Zod-shaped checks in `src/zod.rs` keep diagnostics identical to the TypeScript converter); unknown keys are diagnostics, not crashes.
- Apply editions and `openapi.transforms`, then write `x-ferrule-*` extensions exactly as defined in ADR 0003. Changing an extension's shape requires amending that ADR.
- Conversion is a pure function (spec + config in, annotated spec + diagnostics out). No file or network access outside the CLI entry point (`src/cli.rs`).
- Every diagnostic has a FER code, a JSON pointer into the spec or config, and a suggested fix.
- Snapshot tests per oracle config in `tests/snapshots/` (never regenerated to make a test pass); property tests with proptest for transforms.
- Never depend on the `forge` crate here.
- Commands: `cargo test -p ferrule-converter`, `mise run lint:rs`, `mise run converter:oracles`.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
