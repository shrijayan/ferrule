# Contributing to Ferrule

Thanks for helping teams leave Stainless without their users noticing. Read
[CLAUDE.md](./CLAUDE.md) for the engineering principles every change is reviewed against.

## Setup

1. Install [mise](https://mise.jdx.dev/).
2. Clone and install tools and dependencies:

   ```bash
   git clone https://github.com/shrijayan/ferrule && cd ferrule
   mise install          # Rust 1.99.0, Python 3.12, uv, Node 24 + pnpm (site only), lefthook, gitleaks
   mise run install      # uv workspace and the site's pnpm packages, from frozen lockfiles
   lefthook install      # pre-commit: format, lint, typecheck, secret scan
   ```

The converter and the plugin are Rust crates under `crates/` (ADR 0014); Forge comes in as a
pinned Cargo dependency, so there is no separate Forge build to generate an SDK.

## Commands

```bash
mise run check                        # everything CI runs; must pass before you push
mise run lint:rs                      # cargo fmt + clippy; also lint:py, lint:ts (site)
mise run test:rs                      # cargo test; also test:py, typecheck:py
mise run golden                       # the plugin's golden tests
cargo build --release -p ferrule-gen  # the binary `ferrule generate` and `adopt` run
mise run docs:build                   # build the docs site
mise run parity -- onebusaway         # one oracle
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
