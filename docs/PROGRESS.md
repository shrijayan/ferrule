# Progress

Source of truth: [plan.html](plan.html). Resume from **Next**.

## Phase 0 — Production baseline: DONE (local gate passed 2026-10-23)

Gate: `mise run check && mise run docs:build && mise run release:dry-run && mise run oracles:fetch -- onebusaway` — all green.

- Root workspace (pnpm + uv, mise, lefthook, biome, strict TS/Python).
- Forge pinned by git commit `91f52a40` and built from source (ADR 0005); no telemetry (ADR 0006, `mise run forge:telemetry`).
- `converter/`, `python-plugin/` (TS) and `runtime/`, `migrate/` (Python) skeletons, ≥90% coverage.
- `.github/` workflows (ci, security, scorecard, release, docs, dco, commitlint, parity, corpus, forge-canary), release-please, Renovate, devcontainer, release dry run.
- Docs site (Starlight), governance files, NOTICE, CLAUDE.md, `.claude/`, ADRs 0001–0006, Forge RFC draft.
- Oracles pinned at last Stainless commit (`oracles/manifest.yaml`); sdk-config pinned at `27cdbb11` (last commit before upstream deleted `stainless-config.yml`, md5-paired with python-sdk 1.30.0). Corpus Tier 1 (14 specs) + Tier 2 pinned by sha256.
- Remote half of the gate (signed dev release, deployed docs) → [HUMAN-ACTIONS.md](HUMAN-ACTIONS.md). A separate session is setting up the GitHub repo at the user's request.

## Phase 1 — Plugin spike and adapter: DONE (local gate passed 2026-10-23)

Gate: `mise run check && mise run spike` — green. The spike converts the real pinned
OneBusAway pair, runs Forge + the WASM plugin, and the 5 generated files pass ruff,
pyright strict and mypy strict inside a copy of the oracle SDK and are byte-identical
to the oracle except the header line.

- Converter: resources → Fern grouping + `x-ferrule-*` (resource, method, pagination,
  package, models), `ferrule-convert` bin, OneBusAway snapshot.
- Plugin: adapter (only Forge importer) → SdkModel → pure emitters; golden fixtures
  (`mise run golden`); cursor pagination via synthetic fixture (ADR 0007).
- Gaps: [forge-gaps.md](forge-gaps.md) — 16 gaps, none blocking; no fork or upstream
  patch needed. G2/G3 upstream issues queued in HUMAN-ACTIONS.
- ADR 0008: Dependabot instead of Renovate (maintainer's choice at repo setup).

## Phase 2 — Converter: IN PROGRESS

See [phases/phase-2.md](phases/phase-2.md).
