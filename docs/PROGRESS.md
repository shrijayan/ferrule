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

## Phase 2 — Converter: DONE (local gate passed 2026-10-23)

Gate: `mise run check && mise run converter:oracles` — green on all 11 config oracles
(OneBusAway + 10 public config/SDK pairs, 8 md5-proven, ADR 0009); `mise run spike` still green.

- Validation against the public JSON schema (own validator, FER11xx with line/column), editions
  and defaults (`applyEdition`).
- `openapi.transforms` (update, append, merge, remove, copy, move; JSONPath Plus subset, FER12xx)
  plus legacy `oneOfToAnyOf` in deprecated `transformations`.
- Extensions: `x-ferrule-client` (FER13xx), `x-ferrule-streaming` (FER14xx), all six pagination
  schemes with nested/header fields (FER15xx), resource/method order (gap G11), `x-fern-type-name`,
  `$client` methods via the reserved Forge group `$ferrule_client`. Shapes in ADR 0003.
- Oracle snapshots store digests of our output only (ADR 0010).
- Fixed from oracle runs (test-first): missing model schemas warn (opencode), endpointless
  `webhook_unwrap` methods accepted (imagekit), legacy `oneOfToAnyOf` (ogx).
- Security fix: generated docstrings escape spec text; repo slug validated.

### Carried into Phase 3
- `webhook_unwrap` methods are accepted but not yet emitted (`client.webhooks.unwrap`).
- Plugin must read `x-ferrule-client`, order, streaming, all pagination schemes, `$client`.

## Phase 3 — Python parity: DONE (local gate passed 2026-10-23)

Gate: `mise run check && mise run golden && mise run parity -- onebusaway` — green.

**OneBusAway: public API 0 breaking · 0 additive; its own test suite 1110 passed, 0 failed**
(against a Steady mock, as its `scripts/mock` uses); deterministic; text closeness 93.67%.

- Runtime (`runtime/`): Stainless-generated runtime (Apache-2.0, OneBusAway 1.30.0) as a
  tokenised template; renders byte-identical runtimes for onebusaway, groq, lithic, isaacus,
  perplexity, knock-mgmt, imagekit; `pagination.py` for all six schemes (offset, cursor_url and
  page_number-without-total have no oracle yet: behaviour-tested only). Fault-injection tests.
- Plugin: whole SDK code (client, resources incl. nested/paths/bodies/streaming/pagination,
  types, `__init__`/`_version`); OneBusAway 80/80 generatable files identical; lumaai 21/25.
- Project files: README, api.md, .stats.yml, pyproject, tests, scripts; 77/78 identical,
  branding swaps listed in `python-plugin/src/emit/files/BRANDING.md`.
- `ferrule generate` (convert → Forge → plugin → runtime → ruff isort → ruff format) and
  `ferrule verify` (griffe, never imports the SDK); parity harness writes `reports/parity/`.
- ADR 0011 (keep files the generator no longer writes), ADR 0012 (type-check generated SDKs
  with their own config).
- Security: four code-injection paths in emitters closed (identifiers validated, literals
  escaped); runtime copy refuses symlinks.

### Known gaps carried forward
- `custom_casings` not carried (lumaai `LumaAI`); Forge can't put one endpoint in two methods
  (G19); spec `example`/`minimum`/`maximum` not exposed by Forge (G18), so generated test
  examples use defaults; several environments and param-switched streaming are FER201.
- Placeholders: version (0.0.1 unless adopted), README example heuristic, readme title.
- `webhook_unwrap` not emitted.

## Phase 4 — Adopt, verify, keep regenerating: NEXT

See [phases/phase-4.md](phases/phase-4.md).
