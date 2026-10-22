# Human actions

Things the autonomous build must not do. Each has exact steps.

1. **Name check and registration** — confirm `ferrule` is free on PyPI, npm and GitHub; create the GitHub org `ferrule-dev`; reserve PyPI `ferrule` and npm `@ferrule/python-plugin`.
2. **Create the remote and push** — `gh repo create ferrule-dev/ferrule --public --source . --push`.
3. **Branch protection on `main`** — require PR, linear history, signed commits, status checks `ci`, `security`, DCO app.
4. **Trusted publishing** — add PyPI and TestPyPI trusted publishers for `ferrule-dev/ferrule`, workflow `release.yml`, environment `release`; npm: enable provenance and add `NPM_TOKEN`-free OIDC publishing for `@ferrule`.
5. **GitHub Pages** — Settings → Pages → Source: GitHub Actions.
6. **Install apps** — Renovate, DCO (probot/dco), OpenSSF Scorecard (runs via workflow).
7. **Forge RFC** — open an issue on cloudflare/forge describing the Python plugin and migration tools; link RFC #45. Draft: `docs/rfcs/forge-python-plugin.md`.
8. **Legal review of runtime reuse** before 0.1 (plan §4 legal guardrails).
