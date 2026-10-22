# 0010. Converter oracle snapshots store digests, not specs

- Status: Accepted
- Date: 2026-10-23
- Refines: plan §16 Phase 2 ("converter snapshots committed")

## Context

A converted spec is the third-party OpenAPI document plus our annotations.
Some config oracles come from repositories under AGPL-3.0 (papr) or with no
licence (imagekit's config repo); full snapshots would commit their text, and
were 6 MB.

## Decision

`converter/test/__snapshots__/oracles/<name>.json.snap` stores:

- `sha256` of the whole converted output (any byte change fails the gate);
- `root`: the root `x-ferrule-*` extensions;
- `mirrored`: values copied onto every operation (Forge gaps G2/G3), once;
- `operations`: each operation's `x-` extensions, keyed `VERB /path`.

Only our own output and the facts needed to key it (paths, verbs) are
committed. The OneBusAway snapshot (Apache-2.0) stays a full snapshot.

## Consequences

Reviewers see annotation diffs directly; a diff that changes only the hash
means the converter changed spec content, and is investigated by running
`mise run converter:oracles` locally against the cache.
