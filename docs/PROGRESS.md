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

## Phase 4 — Adopt, verify, keep regenerating: gate passed locally (2026-10-26), PRs in review

See [phases/phase-4.md](phases/phase-4.md).

### Phase 4 status (2026-10-23, paused at usage limit)
- Baseline (`--skip-tests`): onebusaway 0 breaking; isaacus 0 breaking / 24 additive; lumaai
  9 breaking; honcho, stagehand, ogx, imagekit, knock, knock-mgmt, opencode, papr fail before
  output (contract mismatches, union models, non-object bodies, Forge parse errors).
- Agents dispatched in worktrees (branches `worktree-agent-*`, unmerged): A rescue/adopt,
  B three-way merge, C GitHub Action, D1 plugin gaps, D2 Forge-parse normalisation.
- Next on resume: cherry-pick each finished branch (linear history), wire `adopt:gate` in
  mise.toml, pick five targets with breaking 0, run the Phase 4 gate.

### Where to continue (handoff, 2026-10-25)
Already on main: rescue/adopt, three-way merge, GitHub Action, Forge-parse normalisation,
`adopt:gate` task, fidelity board, Tier 1 corpus harness. Plan: [plan.html](plan.html).

1. Fold in side-session branches (local only, not pushed; rebase onto main, one at a time,
   `mise run check` after each):
   - `claude/eloquent-moser-9f0e7c`: make Tier 1 corpus specs readable by Forge (8 commits)
   - `claude/busy-greider-31053b`: regenerate opencode-python-sdk with Ferrule (5)
   - `claude/blissful-leakey-c6f34e`: fake server in every SDK (3)
   - `claude/wizardly-cray-eea71f`: agent context in wheels, ADR 0013 (3)
   - `claude/suspicious-clarke-833d81`: draft opencode migration report (1)
2. Check whether these agent branches were fully cherry-picked (likely, as amended copies):
   `worktree-agent-abb6070…` (plugin union/alias fixes), `worktree-agent-a1f47d7…` (merge),
   `worktree-agent-a8eae2c…` (runtime tests).
3. Pick five targets with 0 breaking and run the Phase 4 gate:
   `mise run check && mise run adopt:gate`. Record the result here. Done; see below.

### Phase 4 gate result (2026-10-26)

`mise run check && mise run adopt:gate` is green on a local tree that combines main with the open
PRs below. Every target adopts with 0 breaking API changes, custom code byte-identical and the
SDK's own tests passing:

| Target | Breaking | Custom code changed | Its own tests |
| --- | --- | --- | --- |
| onebusaway | 0 | 0 | 1110 passed |
| lumaai | 0 | 0 | 728 passed |
| isaacus | 0 (hand-written `Span.decode` kept as a patch) | 0 | 472 passed |
| honcho | 0 (older generator style read from its runtime, ADR 0018) | 0 | 1241 passed |
| knock-mgmt | 0 | 0 | 468 passed |

The gate passes on main once these PRs merge (stacked; merge in this order):
#13 (custom casings, standalone APIs), #16 (pagination items), #18 (project files for every
auth style), #21 (union, map and array bodies, environments, path params, multipart), #26 (older
generator styles), #30 (knock-mgmt model shapes), plus the independent #24 (busy mock port) and
#28 (executable scripts). Already merged: #11, #12 (aliases, object-form and non-object models),
#14 (hand edits carried by later Stainless commits). PRs #16, #22 and #30 carry snapshot changes
and #21, #26, #30 new goldens that need the maintainer's approval.

Remaining gaps (not needed for the gate): knock 138 breaking (`positional_params`, object query
params), imagekit 81 (union variant names from schema `title`, webhooks, camelCase method names),
opencode 15, papr (its public config doesn't pair with its SDK). Param-switched streaming (#33) and
header params (#36) now let stagehand generate (109 breaking, a third of it hand-written client
code); ogx next needs array-of-union names Forge doesn't expose (gap G30).

## Rust move (ADR 0014): done (2026-10-25)

The converter and the plugin are Rust: `crates/converter`, `crates/python-plugin` (a native
`forge::Plugin`; only `src/adapter/` and the `src/plugin.rs` entry import Forge) and the
`ferrule-gen` binary in `crates/ferrule-gen`. Forge is a Cargo git dependency at the same commit
as `scripts/forge-setup.sh`. `converter/` and `python-plugin/` (TypeScript) are deleted;
`runtime/` and `migrate/` stay Python, and `site/` is the only TypeScript left.

Proof, all-Rust pipeline:
- OneBusAway parity: 0 breaking, 0 additive, 1,110 of its own tests passing, deterministic.
- `mise run adopt:gate -- onebusaway`: OK. `mise run spike`: OK.
- All 11 converter oracle snapshots match (`crates/converter/tests/snapshots/`).
- Plugin output byte-identical to every golden (5 + 7 + 85 files), and 0 code-file differences
  against the TypeScript plugin on 243 files before it was deleted.

What changed for users:
- No Node is needed to generate an SDK: `cargo build --release -p ferrule-gen`, then
  `ferrule generate|adopt`. They take `--gen PATH` (default `$FERRULE_GEN`, else
  `target/release/ferrule-gen` in the checkout). `FERRULE_FORGE`, `FERRULE_PLUGIN`,
  `FERRULE_NODE`, `FERRULE_CONVERTER` and `plugin.wasm` are gone; `mise run forge:setup` is only
  needed for the Forge telemetry check and the canary.
- `ferrule.lock` is `lock_version` 2: `generator: "ferrule-gen <version>"` replaces
  `plugin_sha256`, and `forge_commit` comes from `ferrule-gen --version`. Version 1 locks still
  read.

## Multi-language SDKs (ADR 0017): started (2026-10-26)

Ferrule generates every major Stainless language from one language-neutral model
(`crates/sdk-model`, the only Forge adapter) and one plugin crate per language. Each language is
proven against public Stainless SDKs paired with our configs (`oracles/manifest.yaml`, 23 new
pins). Open PRs, with what is proven:

| Language | PRs | Runtime template | Generated SDK |
| --- | --- | --- | --- |
| model and config | #17 (sdk-model, `--target`), #22 (per-target config) | | Python re-derived exactly from the neutral model |
| oracles | #20 | | TypeScript, Go, Java, Kotlin, Ruby, PHP pinned |
| TypeScript, current layout | #19, #27 | isaacus, imagekit, knock-mgmt byte-identical | isaacus-typescript 84/84 files; its lint and 216 jest tests pass |
| TypeScript, node layout | #29 | onebusaway 36/36 | onebusaway-typescript 108/109 |
| TypeScript pagination | #34 | | every oracle pagination file byte-identical, both layouts |
| Go v1 | #19, #23 | onebusaway-go 25/25 | onebusaway-go 65/69 (4 need spec examples, gap G18); builds, tests pass |
| Go pagination, v2 layout | #37 | imagekit-go, knock-mgmt-go, ogx-go 54/54 | imagekit-go 17/45, knock-mgmt-go 8/50 (floors); knock-go cursor pages 3/4 |
| Java, Kotlin | #25, #32 | onebusaway-kotlin 61/61, imagekit-java 67/67 | onebusaway-java 328/343, onebusaway-kotlin 282/338, imagekit-java 186/291 |
| Ruby | #25, #35 | onebusaway-ruby 58/58 | onebusaway-ruby 326/329, imagekit-ruby 348/432; both gems' own tests pass |
| PHP | #38 | imagekit-php 48/48 | imagekit-php 268/336 shared files (86 more under other names); its 87 pest tests pass |

In progress: C# (imagekit-dotnet, which pairs with the `csharp` branch of
imagekit's config repo; stagehand-net, MIT, for comparison). Then CLI and Terraform. Python moves onto `sdk-model` once the Phase 4 PRs
land (`crates/sdk-model/MIGRATION.md`).

