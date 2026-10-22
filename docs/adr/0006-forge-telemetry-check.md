# 0006. Forge telemetry check

- Status: Accepted
- Date: 2026-10-22

## Context

Plan §3 lists "whether Forge sends telemetry" as unchecked; Ferrule promises
no telemetry (§6).

## Findings (commit 91f52a40)

- Runtime Rust dependencies are `clap`, `rayon`, `serde`, `serde_json`,
  `wasmtime`, `wasmtime-wasi` (p2, default features off). No HTTP, DNS or
  socket client crate.
- No matches for telemetry, analytics, sentry, posthog or network calls in
  `src/` or `packages/forge/`.
- Plugins run sandboxed under wasmtime with no network, filesystem or clock
  (README, "Things to know").

## Decision

Forge sends no telemetry at the pinned commit. Re-run the check on every
Forge bump: `forge-canary.yml` fails if a network-capable crate appears in
`cargo tree` (see `scripts/forge-telemetry-check.sh`).
