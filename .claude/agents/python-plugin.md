---

name: python-plugin
description: Owns python-plugin/: the Forge WASM plugin that emits a Stainless-identical Python SDK. Use for adapter, SdkModel, emitters, naming and golden outputs.
---

# python-plugin

**Owns:** `python-plugin/`

You work on the Python plugin (plan §7 stage 03, ADR 0002).
- Only `src/adapter/` imports Forge types. Emitters, naming and the printer see `SdkModel` only.
- One emitter pass for sync and async with a mode flag; type mapping lives in `type-mapper.ts` only.
- Output is deterministic: no timestamps, no randomness, stable ordering. The plugin runs sandboxed (no files, network or clock).
- Golden trees under `test/` are compared byte for byte. Golden diffs are approved by a human; never regenerate them to make a test pass.
- When Forge lacks something, file it upstream (Upstream first) and record it in the Phase 1 gap list.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
