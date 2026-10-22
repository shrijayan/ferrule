# 0007. Phase 1 spike resource and synthetic pagination

- Status: Accepted
- Date: 2026-10-23

## Context

Plan §16 Phase 1 asks for "one OneBusAway resource with pagination". The
pinned OneBusAway config (`27cdbb11`) marks both list methods
`paginated: false` and has no `pagination:` section; the python-sdk has no
`pagination.py`. There is no paginated OneBusAway endpoint.

## Decision

Generate `agencies_with_coverage` (the README's `pagination` example
endpoint, and the case where `paginated: false` must stop list-method
pagination matching). Exercise the cursor pagination path through the same
adapter with a minimal synthetic fixture under `python-plugin/test/fixtures/`.
Real pagination parity is proven in Phase 3 against groq/lithic.

## Consequences

The spike proves the adapter carries `x-ferrule-pagination`, not that the
page classes match Stainless; that is a Phase 3 gate.
