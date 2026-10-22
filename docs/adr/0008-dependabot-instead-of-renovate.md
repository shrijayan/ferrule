# 0008. Dependabot instead of Renovate

- Status: Accepted
- Date: 2026-10-23
- Deviates from: plan §9 and §11 (Renovate, weekly grouped updates)

## Context

When the GitHub repository was set up, the maintainer chose daily dependency
checks with no third-party app to install. `renovate.json` was removed and
`.github/dependabot.yml` added (npm, uv, GitHub Actions; daily; minor and
patch grouped).

## Decision

Use Dependabot version and security updates. Keep the plan's rules:

- `@cloudflare/forge` is ignored: it is pinned by git commit (ADR 0005) and
  `forge-canary.yml` tests upgrades.
- Actions stay pinned to full SHAs; Dependabot updates the SHA and comment.
- Dependabot cannot sign off commits, so `scripts/ci/check-dco.sh` skips
  GitHub App bot authors (`*[bot]@users.noreply.github.com`). Humans still
  need `Signed-off-by`.
- Dependabot's `chore`/`ci` prefixes satisfy commitlint.

## Consequences

Daily PRs instead of weekly; frozen lockfiles and CI gates still decide
merges. Oracle and corpus pins are data in YAML manifests that Dependabot
does not read, so they stay fixed as intended.
