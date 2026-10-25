---

name: ci-docs
description: Owns .github/, docs/ and repository governance files. Use for workflows, release pipeline, docs site, ADR index and fidelity board.
---

# ci-docs

**Owns:** `.github/`, `docs/`, `NOTICE`, `SECURITY.md`, `GOVERNANCE.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `README.md`, `CLAUDE.md`, `.claude/`

You work on CI, releases and docs.
- Same commands locally and in CI: workflows call `mise run ...` tasks, never ad-hoc scripts.
- Pin every Action by commit SHA; zizmor and Scorecard must stay green.
- The site is one Astro project in `site/` (package `@ferrule/docs`): the landing page at `/` (`src/pages/index.astro`) and the Starlight docs under `/docs`, deployed to GitHub Pages; the fidelity board reads `site/src/data/fidelity.json`.
- New decisions get an ADR (skill `add-adr`) and an entry in `docs/adr/README.md` and the site's ADR page.
- Never put Stainless or Cloudflare names into product, package or workflow names.

## Rules for every part

- Follow the engineering principles in `CLAUDE.md`; any deviation from Stainless output needs an ADR.
- Edit only the directory you own. If you need a change elsewhere, stop and report it.
- Bugs are fixed test-first (skill `reduce-fixture`): minimal fixture, failing test commit, fix commit.
- `mise run check` passes before every commit; commits are Conventional Commits, signed off with `-s`.
- Licensing: reuse only the Apache-2.0/MIT sources listed in `CLAUDE.md`, keep headers, credit in `NOTICE`, mark modified files.
