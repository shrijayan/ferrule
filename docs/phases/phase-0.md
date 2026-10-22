# Phase 0 — Production baseline

Plan §16 Phase 0. Remote-only steps (branch protection, live workflows, TestPyPI/npm dev release, Pages deploy, Forge RFC) are queued in [../HUMAN-ACTIONS.md](../HUMAN-ACTIONS.md); everything else must work locally.

## Tasks

| # | Task | Owner (files) | Independent |
|---|------|---------------|-------------|
| A | Root workspace: pnpm, uv, mise, lefthook, biome, tsconfig, ruff/pyright/mypy, .gitignore | lead (root files) | — (done first) |
| B | Forge pin: build from pinned commit, `scripts/forge-setup.sh`, telemetry check | lead (`scripts/`, ADR 0005–0006) | yes |
| C | TS packages: `converter/`, `python-plugin/` skeletons, strict tsconfig, Vitest + coverage, one test each | agent-ts | yes |
| D | Python packages: `runtime/`, `migrate/` (CLI `ferrule`) skeletons, pytest + coverage, one test each | agent-py | yes |
| E | CI/release: `.github/` workflows (§14), release-please, Renovate, CODEOWNERS, templates, devcontainer, dry-run scripts | agent-ci | yes |
| F | Docs + governance: Starlight site in `docs/site/`, LICENSE/NOTICE/SECURITY/GOVERNANCE/CONTRIBUTING/CODE_OF_CONDUCT, CLAUDE.md, `.claude/`, ADRs 0001–0004 | agent-docs | yes |
| G | Oracle + corpus manifests, `oracles/` harness that clones into `oracles/cache/` pinned to last Stainless commit | agent-oracles | yes |

## Exit gate (local)

```bash
mise run check && mise run docs:build && mise run release:dry-run && mise run oracles:fetch -- onebusaway
```

The remote half of the plan's gate (a PR on GitHub producing a signed dev release and deployed docs) is a human action.
