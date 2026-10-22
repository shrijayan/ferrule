# Human actions

Things the autonomous build must not do. Each has exact steps. The repository lives at
`shrijayan/ferrule` (Apache-2.0); a move to an org can come later via Settings → Transfer.

1. **Name check and registration** — confirm `ferrule` is free on PyPI and npm; reserve PyPI `ferrule` and npm `@ferrule/python-plugin`.
2. [x] **Create the remote and push** — `shrijayan/ferrule`, public, `main` pushed.
3. [x] **Rulesets** — `main`: PR with 1 approval (stale dismissed, last-push approval, threads resolved), squash/rebase only, linear history, signed commits, no force-push/deletion, strict status checks `ci`, `dco`, `commitlint`, `codeql (javascript-typescript|python|actions)`, `zizmor`, `gitleaks`, `dependency-review`. `release-tags`: `v*` and `*-v*` tags cannot be updated or deleted. Repository admins bypass both, so a solo maintainer can merge their own PRs; remove the bypass once there is a second maintainer. Existing commits are unsigned: set up commit signing (`git config commit.gpgsign true` with an SSH or GPG key) before pushing as a non-admin.
4. **Trusted publishing** — add PyPI and TestPyPI trusted publishers for `shrijayan/ferrule`, workflow `release.yml`, environment `release`; npm: enable provenance and OIDC trusted publishing for `@ferrule`.
5. [x] **GitHub Pages** — source set to GitHub Actions.
6. **Install apps** — Renovate (https://github.com/apps/renovate) and DCO (https://github.com/apps/dco) on `shrijayan/ferrule`. Scorecard runs via workflow.
7. **Forge RFC** — open an issue on cloudflare/forge describing the Python plugin and migration tools; link RFC #45. Draft: `docs/rfcs/forge-python-plugin.md`.
8. **Legal review of runtime reuse** before 0.1 (plan §4 legal guardrails).
9. [x] **`release` environment** — created, deployments limited to `main`, `@shrijayan` required reviewer.
10. [x] **Security settings** — dependency graph and Dependabot alerts on, Dependabot security updates off (Renovate replaces it), secret scanning and push protection on, private vulnerability reporting on, code scanning via the CodeQL job in `security.yml` (default setup left off).
11. **release-please token** — add a GitHub App or fine-grained PAT so release PRs trigger CI (Actions may already create PRs: workflow permissions are read-only by default with PR creation allowed).
12. **CODEOWNERS and links** — replace `@ferrule-dev/maintainers` with `@shrijayan`, and `ferrule-dev/ferrule` / docs base URL with `shrijayan/ferrule` (`.github/ISSUE_TEMPLATE/config.yml`, `.github/CODEOWNERS`, docs site config).
