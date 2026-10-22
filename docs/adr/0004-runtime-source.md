---

status: Accepted
date: 2026-10-22
deciders: Ferrule maintainers
---

# 0004. Runtime source: Apache-2.0 Stainless-generated SDK runtimes, with attribution

## Context and problem statement

To be Stainless-identical, the Python runtime (`_base_client.py`, `_models.py`, `_types.py`,
`_response.py`, `_streaming.py`, `pagination.py`, ...) must behave exactly like the one in
Stainless-generated SDKs. Where does our runtime code come from, legally?

## Decision drivers

- Stainless-generated SDKs are published under Apache-2.0 and owned by their customers.
- Users must not notice the switch: identical behavior and import paths.
- The private Stainless CLI (`stlc`) is licensed and must never be used as a source.

## Considered options

1. Clean-room rewrite from behavior only.
2. Derive `runtime/` from public Apache-2.0 Stainless-generated SDK runtimes, with full attribution.
3. Always copy from the customer's own repository.

## Decision outcome

Chosen: option 2 for `runtime/`, and option 3 in adopt mode.
- `runtime/` is derived from public Apache-2.0 Stainless-generated SDK runtimes. We keep licence
  headers, record each source repository and commit in `runtime/NOTICE` and the root `NOTICE`,
  and mark every modified file (`# Modified by The Ferrule Authors: <what changed>`).
- In adopt mode (`ferrule adopt`), the runtime is copied from the customer's own repository,
  which they already own; Ferrule's runtime is used only for new SDKs or when the customer's
  copy is missing.
- Nobody who has seen the private `stlc` source contributes to Ferrule.
- A lawyer reviews this approach before the 0.1 release (plan Phase 6). Nothing here is legal advice.

### Consequences

- Good: behavior parity by construction; attribution is explicit and auditable.
- Bad: legal risk until reviewed; tracked as a release blocker for 0.1.
- Bad: upstream runtime fixes must be pulled manually; parity runs detect drift.
