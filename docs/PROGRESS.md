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

## Phase 1 — Plugin spike and adapter: IN PROGRESS

See [phases/phase-1.md](phases/phase-1.md).

### Done
- Converter spike (task A): resources → Fern grouping + x-ferrule-* extensions, `ferrule-convert` bin, OneBusAway snapshot.
- Plugin spike (task B): adapter → SdkModel → emitters; OneBusAway `agencies_with_coverage` output byte-identical to the oracle except the header line; cursor-page fixture; `mise run golden`.

### Next
1. Converter: emit `x-ferrule-package {name, repo}` and `x-ferrule-models` on every operation (gaps G2/G3, see `python-plugin/GAPS.md`).
2. Fold `python-plugin/GAPS.md` into `docs/forge-gaps.md`; amend ADR 0003 for both extensions.
3. Write `scripts/spike.sh` + `mise run spike` (convert real OneBusAway → forge → plugin → ruff/pyright → diff vs oracle); run the Phase 1 gate.

