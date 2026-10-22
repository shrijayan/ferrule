# 0011. Keep files the generator no longer writes

- Status: Accepted
- Date: 2026-10-23

## Context

OneBusAway python-sdk 1.30.0 (pinned oracle) ships
`types/search_for_{route,stop}_retrieve_{params,response}.py`, but its config
has no such methods and its `types/__init__.py` does not export them: modules
from an earlier generation that Stainless left in place. Deleting them on
regeneration made `ferrule verify` report 4 breaking changes (a user may still
`import onebusaway.types.search_for_route_retrieve_params`).

## Decision

In adopt mode (`ferrule generate --runtime-from <existing SDK>`), every file in
the existing package that the generator did not write is kept: the runtime,
hand-written `lib/`, and modules a regeneration no longer produces. Generated
files always win. Symlinks are refused (FER4007).

## Consequences

Matches Stainless output (OneBusAway parity: 0 breaking). Stale modules
accumulate as they do under Stainless; removing one is a deliberate,
reviewable change by the SDK owner. Phase 4's three-way merge builds on this.
