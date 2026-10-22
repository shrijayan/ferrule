# Phase 4 — Adopt, verify, keep regenerating

Plan §16 Phase 4: spec rescue, adopt, verify, generated branch with three-way merge, GitHub
Action with semver labels. Gate: five public Stainless Python repos adopted privately (local
clones, nothing pushed) with 0 breaking changes and custom code intact.

Targets: the five config oracles (ADR 0009) with the best Phase 3 baseline, chosen by the lead
from `uv run python -m scripts.parity.run --skip-tests <oracle>`; groq/lithic custom-code cases
are exercised by the merge task's fixtures.

## Tasks

| # | Task | Owner (files) | Independent |
|---|------|---------------|-------------|
| A | `ferrule rescue` (spec from `.stats.yml`, md5-checked) and `ferrule adopt <repo>` (rescue, convert, runtime from the repo, record hand edits as `.ferrule/patches/`, `ferrule.lock`, `.github/workflows/sdk.yml`, local branch + commit; no push, no PR) | migrate agent (`migrate/src/ferrule/{rescue,adopt,lock}.py` + tests) | yes |
| B | Generated branch + three-way merge with libcst AST fallback, conflict report, `lib/` and `keep_files` untouched; `ferrule merge` | migrate agent 2 (`migrate/src/ferrule/merge/**` + tests) | yes |
| C | GitHub Action `action/` (composite: generate, verify, label major/minor/patch from the verify report, open PR) + regeneration workflow template; dry-run locally | ci-docs agent (`action/**`) | yes |
| D | Close emitter/converter gaps blocking the five targets (custom_casings, environments, …), test-first per gap | python-plugin agent (`python-plugin/**`, `converter/**` for config-carried gaps) | yes |
| E | Wire CLI, run adopt on the five targets, fix integration, gate | lead | after A–D |

## Exit gate

```bash
mise run check && mise run adopt:gate
```

`adopt:gate` clones each target into `.cache/adopt/<name>`, runs `ferrule adopt`, then
`ferrule verify` against the pinned release (0 breaking) and the repo's own tests, and checks
that every non-generated file (lib/, examples, keep_files) is byte-identical after adoption.
