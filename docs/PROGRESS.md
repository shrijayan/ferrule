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
   `mise run check && mise run adopt:gate`. Record the result here.

### Phase 4 gate: passed (2026-10-26)

`mise run adopt:gate` — OK on all five targets (adopt exit 0, custom code changed 0):

| SDK | Own tests |
| --- | --- |
| onebusaway | 1110 passed, 0 failed |
| isaacus | 472 passed, 0 failed |
| lumaai | 728 passed, 0 failed |
| honcho | 1241 passed, 0 failed |
| knock-mgmt | 468 passed, 0 failed |

Side branches checked: every unmerged commit is superseded on main (TypeScript-era converter
and plugin work reimplemented in the Rust crates; fake-server and agent-context ADRs landed as
0016 and 0013; `ferrule merge`/`gitio.py` and `runtime/template/` already present).
`mise run check`: green (after deleting stale untracked Astro output in `docs/site/`; the site
lives in `site/`).

## Tier 1 corpus: end-to-end results (2026-10-26)

All 14 Tier 1 specs ran through `mise run corpus -- --tier 1` (`ferrule-gen 0.0.0`, Forge
`91f52a40`). The results are published on the site: `docs/real-world` (board, from
`site/src/data/corpus.json`, built by `mise run board:corpus -- --date 2026-10-26`) and
`docs/corpus-findings` (each problem with its evidence). cloudflare was rerun alone after the
second converter fix; the change does not touch the other specs, whose specs carry no Fern or
Forge extensions.

| Spec | Format | Ops | Generated | Skipped | Stopped at | ruff / pyright / mypy errors | Tests passed / total | Deterministic | Official SDK |
|---|---|---|---|---|---|---|---|---|---|
| stripe | openapi 3.0.0 | 612 | 314 | 298 | pyright | 0 / 1,039 / 210 | — | yes | none |
| github | openapi 3.0.3 | 1,242 | 1,226 | 16 | ruff | 20 / 1,487 / 6 | 8,069 / 22,428 | yes | none |
| openai | openapi 3.1.0 | 361 | 346 | 15 | ruff | 8 / 136 / 2 | 442 / 7,710 | yes | 260/275 endpoints, 537 breaking |
| kubernetes | swagger 2.0 | 1,158 | 1,146 | 12 | ruff | 3,500 / — / 24 | 575 / 1,262 | yes | none |
| twilio | openapi 3.0.1 | 197 | 197 | 0 | ruff | 10 / 39 / 2 | 513 / 5,015 | yes | none |
| cloudflare | openapi 3.0.3 | 3,767 | 2,368 | 905 | ruff | 94 / 1,714 / 2 | — | yes | 1456/2423 endpoints, 1923 breaking |
| azure-storage | swagger 2.0 | 139 | — | — | generate | — | — | — | none |
| slack | swagger 2.0 | 174 | 174 | 0 | pyright | 0 / 11 / 11 | 2,095 / 3,767 | yes | none |
| plaid | openapi 3.0.0 | 350 | 294 | 56 | pyright | 0 / 33 / 6 | 6,144 / 6,328 | yes | none |
| square | openapi 3.0.0 | 332 | 332 | 0 | pyright | 0 / 26 / 11 | 7,074 / 7,250 | yes | none |
| digitalocean | openapi 3.0.0 | 858 | 858 | 0 | pyright | 0 / 5 / 5 | — | yes | 188/193 endpoints, 184 breaking |
| discord | openapi 3.1.0 | 246 | 242 | 4 | pyright | 0 / 357 / 159 | 5,677 / 5,759 | yes | none |
| docker | swagger 2.0 | 108 | 100 | 8 | ruff | 2 / 99 / 2 | 1,232 / 2,666 | yes | none |
| jira | openapi 3.0.1 | 630 | 606 | 24 | ruff | 104 / 1,137 / 4 | — | yes | none |

- Every generation is deterministic (same tree digest twice) and every generated SDK builds a
  wheel. azure-storage stops in generate on `$ref`s into other files (G29).
- The compile gate passes for none of them. The largest cause by far is a parameter emitted
  twice: `allOf` duplicates, path or query parameters clashing with body fields, and body fields
  named `timeout`. One `SyntaxError` sinks every test that imports the module.
- digitalocean's 858 methods are empty: every operation is a `$ref` to an unfetched sibling file,
  and Forge reads that as an empty operation without a diagnostic (new gap G32, issue drafted in
  HUMAN-ACTIONS.md item 16).
- Official Stainless SDKs (verified statically, never run): openai 260 of 275 endpoints,
  cloudflare 1,456 of 2,423, digitalocean gradient 188 of 193 (paths only). The breaking counts
  are large by construction: the official `stainless.yml` files are private.
- Fixed test-first on the way: the runtime named after the client, not the package (migrate);
  Fern grouping and Forge aliases that a spec carries itself, removed from operations the config
  leaves out (converter, two fixes); the Jira re-pin.
- Harness: per stage pass/fail with the most frequent causes, refused operations fixed or skipped
  with their FER code, `--stage compare`, external `$ref` counts.

Next: the open items in `docs/corpus-findings`, in order (duplicate parameters first, then
form-encoded bodies), and a bundling step for multi-file specs (G29).

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
