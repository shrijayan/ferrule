# Ferrule: instructions for coding agents

Ferrule is an open-source toolkit for teams leaving Stainless: it turns an OpenAPI spec and a
`stainless.yml` into the same idiomatic Python SDK Stainless would have generated, using
Cloudflare Forge as the engine, and proves nothing public changed. The full plan is
`docs/plan.html`; the current phase is in `docs/phases/`, progress in `docs/PROGRESS.md`,
decisions in `docs/adr/`.

## Engineering principles

The rules the code is reviewed against (plan §8, word for word).

**Stainless output is the spec**
When unsure, the oracle repos decide. Any deviation needs an ADR.

**Forge stays behind the adapter**
Only adapter/ imports Forge types. Emitters see our model only.

**DRY across sync and async**
One emitter pass with a mode flag. Type mapping lives in one function.

**Runtime written once**
The runtime lives in runtime/, tested once, copied into each SDK. SDKs never depend on Ferrule or Forge.

**Deterministic output**
Same inputs give byte-identical files. No timestamps, no LLM calls at generation time.

**Pure functions**
Convert and emit are pure. File and network access only in the CLI.

**Diagnostics with addresses**
Every error has a FER code, a pointer into the spec or config, and a suggested fix.

**Upstream first**
Anything generic (a Forge bug, a missing field) goes to Forge as an issue or PR, not a local workaround.

## Repository layout

```
crates/          Rust workspace (ADR 0014); Forge is a Cargo git dependency at the FORGE_PIN commit
  converter/     stainless.yml -> annotated spec (overlays + x-ferrule-*)
  python-plugin/ native forge::Plugin; emits the Python SDK. Only src/adapter/ (+ the src/plugin.rs entry) imports Forge.
  ferrule-gen/   binary: `ferrule-gen convert` and `ferrule-gen generate`
runtime/         Python runtime + NOTICE, copied into every generated SDK
migrate/         Python CLI `ferrule`: generate, adopt, rescue, verify, merge
action/          GitHub Action + regeneration bot
oracles/         manifest.yaml (repo, last Stainless commit, hashes) + harness
corpus/          manifest.yaml, fixtures/, baselines/
site/            Astro: landing page at /, Starlight docs under /docs, fidelity board
docs/            adr/, rfcs/, phases/, research/, plan.html
.claude/         agents/, skills/, settings.json (hooks)
.github/         workflows, CODEOWNERS, templates
```

Each Claude session owns one part (see `.claude/agents/`). Do not edit files owned by another
part; report the change you need instead.

## Commands

```bash
mise install                    # Rust, Python, uv, Node + pnpm (site only), lefthook, gitleaks
mise run install                # pnpm install --frozen-lockfile (site) + uv sync
mise run check                  # everything ci.yml runs (lint, types, tests: Rust, Python, site TS)
mise run lint:rs                # cargo fmt --check + clippy, warnings denied (lint:py: ruff, lint:ts: biome)
mise run test:rs                # cargo test (test:py: pytest)
mise run typecheck:py           # pyright + mypy (typecheck:ts: tsc for the site)
mise run golden                 # the Rust golden tests (crates/python-plugin/tests/golden/)
cargo build --release -p ferrule-gen  # target/release/ferrule-gen, used by `ferrule generate|adopt`
mise run forge:setup            # build the Forge CLI into .cache/forge (telemetry check, canary only)
mise run docs:build             # build the docs site
mise run parity -- onebusaway   # one oracle
mise run corpus -- --tier 1
```

Run `mise run check` before every commit. Zero warnings; a rule is loosened only by a written
exception with its reason next to it.

## Commits

- Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `refactor:`, `ci:`), enforced by commitlint.
- Always sign off with `git commit -s` (DCO). There is no CLA.
- Small, logical commits; linear history.
- Bugs are fixed test-first: reduce the failure to a minimal fixture, commit the fixture and a
  failing test, then commit the fix (skill: `.claude/skills/reduce-fixture`).
- Golden-output changes are approved by a human; never regenerate goldens just to make a test pass.

## Licensing rules

- Ferrule is Apache-2.0. Never relicense, never add a CLA.
- Reuse code only from the Apache-2.0/MIT sources listed in the plan: Stainless-generated SDK
  runtimes (Apache-2.0), Fern (Apache-2.0/MIT), stainful (MIT), perseid (Apache-2.0),
  TypeSpec http-specs (MIT). Cloudflare Forge (Apache-2.0) is a dependency, not copied.
- Keep original licence headers, credit the source in `NOTICE` in the same commit, and mark
  modified files with a comment such as `# Modified by The Ferrule Authors: <what changed>`.
- Never use the Stainless or Cloudflare names in product, package or command names. Mentioning
  them descriptively ("for teams leaving Stainless", "built on Cloudflare Forge") is fine.
- Never copy, read or paraphrase the private Stainless CLI (`stlc`) source. Anyone who has seen
  it does not contribute.
