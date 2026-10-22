# 0005. Pin Forge by git commit and build it from source

- Status: Accepted
- Date: 2026-10-22
- Deviates from: plan §3, §9, §10 (`"@cloudflare/forge": exact version` in package.json)

## Context

The plan pins `@cloudflare/forge` as an exact npm version. On 2026-10-22 the
package is not on the npm registry (`npm view @cloudflare/forge` → 404). The
repository `cloudflare/forge` (Apache-2.0) contains it at `packages/forge`,
version 0.1.0, plugin interface `forge:plugin@0.3.0`. Its GitHub releases
only carry `openapi.forge.json` artefacts, not binaries. The Rust CLI and the
TypeScript plugin library are built from the same repo.

## Decision

Pin Forge by full commit SHA (`91f52a40c746e8a2ac0271d8be0b7c82cdf28798`)
in `scripts/forge-setup.sh`. `mise run forge:setup` clones it into the
gitignored `.cache/forge`, checks the SHA, and builds the CLI with
`cargo build --release --locked --features wasm`. The plugin package links to
the plugin library from that checkout. This is a pin, not a fork: no patch is
applied.

## Consequences

- Contributors and CI need a Rust toolchain until Forge publishes to npm.
  CI caches `.cache/forge` keyed on the SHA.
- Upgrades are a one-line SHA change gated by adapter contract tests and the
  weekly `forge-canary.yml` run against `main`.
- When `@cloudflare/forge` appears on npm, switch to the exact npm version and
  supersede this ADR.
