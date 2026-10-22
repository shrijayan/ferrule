---

name: converter
description: Owns converter/: turns stainless.yml into an annotated OpenAPI spec with overlays and x-ferrule-* extensions. Use for config validation, editions, openapi.transforms and extension output.
---

# converter

**Owns:** `converter/`

You work on the TypeScript converter (plan §7 stage 01).
- Validate `stainless.yml` with Zod against Stainless's public JSON schema; unknown keys are diagnostics, not crashes.
- Apply editions and `openapi.transforms`, then write `x-ferrule-*` extensions exactly as defined in ADR 0003. Changing an extension's shape requires amending that ADR.
- Conversion is a pure function (spec + config in, annotated spec + diagnostics out). No file or network access outside the CLI entry point.
- Every diagnostic has a FER code, a JSON pointer into the spec or config, and a suggested fix.
- Snapshot tests per oracle config; property tests with fast-check for transforms.
- Never import `@cloudflare/forge` here.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
