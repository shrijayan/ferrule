# Ferrule

**Leave Stainless without your users noticing.** Same config, same Python SDK, proof of zero
breaking changes.

Ferrule is an open-source toolkit for teams leaving Stainless. It turns an OpenAPI spec and a
`stainless.yml` into the same idiomatic Python SDK Stainless would have generated, and proves
nothing public changed. It uses Cloudflare's Forge as the engine instead of building another
generator. Everything runs locally and in CI: no hosted service, no account.

> Status: Phase 0 (production baseline). Not usable yet.

```bash
uvx ferrule adopt .                     # rescue spec, convert config, record hand edits, open a PR
uvx ferrule verify --against v2.14.0    # public API: 0 breaking · 6 additive
```

## What it is

- A **converter**: `stainless.yml` to Forge overlays and `x-ferrule-*` extensions
- A **Forge plugin** that emits Stainless-identical Python
- **Migration tools**: rescue, adopt, verify, custom-code merge

## What it is not

Not a spec parser or general generator platform, not a fork of Forge, not a hosted service, and
not multi-language until Python is proven.

## Why Ferrule

- **Proof that nothing broke**: `ferrule verify` diffs the public API against your last Stainless release.
- **Spec rescue and one-command adoption.**
- **Output you own**: Apache-2.0, no licence obligations on generated code, no token, no telemetry, DCO and never a CLA.
- **Agent context in every wheel**, a **fake server in every SDK**, and a public **fidelity board**.

## Documentation

- Docs site: https://shrijayan.github.io/ferrule
- Plan: [docs/plan.html](./docs/plan.html) · Decisions: [docs/adr/](./docs/adr/)
- Contributing: [CONTRIBUTING.md](./CONTRIBUTING.md) · Governance: [GOVERNANCE.md](./GOVERNANCE.md) · Security: [SECURITY.md](./SECURITY.md)

## Licence

Apache-2.0. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE). Ferrule is independent and not
affiliated with Stainless or Cloudflare.
