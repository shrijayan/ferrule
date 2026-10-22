---

name: runtime
description: Owns runtime/: the Python runtime copied into every generated SDK (client, retries, pagination, streaming). Use for runtime behavior and fault-injection tests.
---

# runtime

**Owns:** `runtime/`

You work on the Python runtime (ADR 0004).
- The runtime must match Stainless-generated SDK runtimes: 60 s timeout, 2 retries (0.5-8 s backoff, 408/409/429/5xx, retry-after), idempotency keys, NOT_GIVEN/Omit, pagination and SSE/JSONL streaming.
- Sources are Apache-2.0 Stainless-generated SDK runtimes only. Keep licence headers, credit in `runtime/NOTICE` and root `NOTICE`, and mark every modified file.
- Generated SDKs never depend on Ferrule or Forge; the runtime has no imports from them.
- Support pydantic v1 and v2; keep the httpx transport abstracted.
- Retry, pagination and streaming code must kill 80% of mutants (mutmut nightly).
- Never look at the private Stainless CLI (stlc) source.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
