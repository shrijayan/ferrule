# Contributing to Ferrule

Thanks for helping teams leave Stainless without their users noticing. Read
[CLAUDE.md](./CLAUDE.md) for the engineering principles every change is reviewed against.

## Setup

1. Install [mise](https://mise.jdx.dev/).
2. Clone and install tools and dependencies:

   ```bash
   git clone https://github.com/ferrule-dev/ferrule && cd ferrule
   mise install          # Node 24, pnpm, Python 3.12, uv, lefthook, gitleaks
   mise run install      # pnpm + uv workspaces from frozen lockfiles
   mise run forge:setup  # build the pinned Forge commit
   lefthook install      # pre-commit: format, lint, typecheck, secret scan
   ```

## Commands

```bash
mise run check                  # everything CI runs; must pass before you push
mise run lint:ts                # also lint:py, typecheck:ts, typecheck:py, test:ts, test:py
mise run docs:build             # build the docs site
mise run parity -- onebusaway   # one oracle
```

## Commits: Conventional Commits + DCO

- Messages follow [Conventional Commits](https://www.conventionalcommits.org/): `feat(converter): ...`, `fix(runtime): ...`, `docs: ...`.
- Every commit must be signed off under the [Developer Certificate of Origin](https://developercertificate.org/).
  Use `git commit -s`, which adds `Signed-off-by: Your Name <you@example.com>` using your Git
  `user.name` and `user.email`. Forgot? `git commit --amend -s` for the last commit, or
  `git rebase --signoff main` for a branch.
- There is no CLA.

## Bugs: test first

Reduce the problem to a minimal fixture, commit the fixture with a failing test, then commit the
fix. Golden-output changes need maintainer approval.

## Licensing

Only reuse code from Apache-2.0 or MIT sources listed in CLAUDE.md, keep their headers, credit
them in `NOTICE`, and mark modified files. If you have seen the private Stainless CLI (`stlc`)
source, please do not contribute code.

## Decisions

Larger changes and any deviation from Stainless output need an ADR in `docs/adr/`.
