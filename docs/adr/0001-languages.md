---

status: Accepted, amended by [0014](./0014-rust-for-converter-and-plugin.md)
date: 2026-10-22
deciders: Ferrule maintainers
---

# 0001. Languages: TypeScript for converter and plugin, Python for runtime and migration tools

Supersedes the Python-core decision in plan draft 2.

Amended by [ADR 0014](./0014-rust-for-converter-and-plugin.md): the converter and the plugin
move to Rust. The runtime and the migration tools stay Python.

## Context and problem statement

Ferrule has four parts: the converter (stainless.yml to annotated spec), the Forge plugin that
emits Python, the runtime shipped inside each SDK, and the migration tools (adopt, verify, merge).
Which language does each part use?

## Decision drivers

- Forge plugins are TypeScript compiled to WebAssembly (`forge:plugin@0.3.0`, `npx forge-plugin build`).
- The runtime ships inside a Python SDK.
- API diffing (griffe) and AST merging and codemods (libcst) are Python libraries.
- Users should run the tools with `uvx`, without a Node toolchain.

## Considered options

1. Everything in Python (draft 2), calling Forge as a subprocess.
2. Everything in TypeScript.
3. TypeScript where Forge runs (converter, plugin); Python where the SDK lives (runtime, migrate).

## Decision outcome

Chosen option 3. The plugin must be TypeScript, and the converter shares its toolchain
(Node 24, pnpm, Biome, Vitest, Zod). The runtime is Python because it ships in the SDK. The
migration tools stay Python because griffe and libcst are Python, and users run them with `uvx`.

### Consequences

- Good: each part uses the native ecosystem of what it touches; no FFI.
- Good: the CLI is a single `uvx ferrule` install for users.
- Bad: two toolchains in one repo; mitigated by mise, one `mise run check` and identical CI commands.
- Bad: the Python CLI must invoke Node/Forge for generation; that boundary is a subprocess with a versioned contract.
