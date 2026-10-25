---
status: Proposed
date: 2026-10-26
deciders: Ferrule maintainers
---

# 0018. Emit an older generator's style when the adopted runtime is older

## Context and problem statement

Stainless changed several spellings in generated Python over time. Oracles generated before a
change differ from today's output in systematic ways: honcho had 8 breaking items, knock 553 and
opencode 84, almost all from this. Which style an SDK uses is decided by the Stainless generator
version, which Ferrule cannot see. The stainless.yml `edition` does not decide it: only one of
our 11 config oracles names an edition, and the other ten get the implicit `python.2025-10-08`
in both the old and the new style.

In adopt mode, though, the SDK keeps its own runtime (ADR 0004), and that runtime supports only
what its generator emitted. What should decide the style?

## Considered options

1. Always today's style (status quo): adopted older SDKs show hundreds of breaking changes.
2. A style per stainless.yml `edition`: not possible, the edition doesn't carry it.
3. Read the style from the adopted SDK's own runtime files.

## Decision outcome

Chosen option 3. `ferrule-gen convert --runtime-from <sdk>` reads the runtime (file access only
in the CLI; detection is pure in `crates/converter/src/style.rs`) and writes the switches that
differ from today as `x-ferrule-style` (root, mirrored on every operation, gap G2). Each switch
is detected on its own, because Stainless changed them at different times (dated from the SDKs'
changelogs). The rules, exact across all 11 oracles:

| Switch | Detected by | Older style |
| --- | --- | --- |
| `sentinel: NOT_GIVEN` (~2025-09) | `_types.py` has no module-level `omit` | defaults are `NOT_GIVEN`; no `omit`/`not_given` |
| `resources: eager` (2025-12) | `_client.py` has no `@cached_property` | resources set in `__init__`, also in the wrappers |
| `paths: fstring` (2026-03) | `_utils/__init__.py` has no `path_template` | paths built with f-strings |
| `multipart: deepcopy_minimal` | `_files.py` has no `deepcopy_with_paths` | multipart bodies copied with `deepcopy_minimal` |

A current runtime, or no runtime (a new SDK), writes nothing, so the output is byte-identical to
before. `ferrule generate` passes `--runtime-from` whenever it copies the runtime from the repo.

### Consequences

- Good: adopting an older SDK no longer reports the generator's own history as breaking changes
  (honcho 8 → 0 and passes the adopt gate; knock 735 → 183; opencode 109 → 25).
- Good: the style always matches the runtime the SDK ships, so it can't emit names the runtime
  lacks.
- Bad: plugin output now depends on `--runtime-from`; a malformed `x-ferrule-style` is FER101.
- Bad: every future Stainless style change needs a new switch and a detection rule.

## More information

ADR 0003 (extension shapes), ADR 0004 (runtime source). The generator-era goldens are in
`crates/python-plugin/tests/golden/generator-eras/`.
