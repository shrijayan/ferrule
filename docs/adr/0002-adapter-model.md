---

status: Accepted, amended by [0017](./0017-multi-language-targets.md) (proposed)
date: 2026-10-22
deciders: Ferrule maintainers
---

# 0002. Adapter model: only adapter/ imports Forge; emitters see SdkModel

## Context and problem statement

Forge's plugin interface is pre-1.0 and has already broken once. If emitters consume Forge's data
types directly, every Forge change ripples through the whole plugin, and replacing Forge would
mean a rewrite.

## Decision drivers

- Principle "Forge stays behind the adapter".
- Forge upgrades should be decided by adapter contract tests and parity, not by refactors.
- Cloudflare may ship its own Python target; our emitters should survive a change of engine.

## Considered options

1. Emitters use Forge types directly.
2. A thin adapter in `python-plugin/src/adapter/` maps Forge data into our own `SdkModel`
   (`python-plugin/src/model/`); everything else depends only on `SdkModel`.

## Decision outcome

Chosen option 2.
- `adapter/to-model.ts` converts Forge clients, groups, methods, models and types to `SdkModel`.
- `adapter/extensions.ts` is the only reader of `x-ferrule-*` extensions (ADR 0003) and turns
  them into typed `SdkModel` fields.
- `SdkModel` describes the SDK, not the spec: resources (tree, sync/async neutral), methods
  (HTTP verb, path, params, body, response, pagination, streaming), types (models, TypedDict
  params, Literal enums) and client settings.
- `plugin.ts` wires `definePlugin({ generate, generateGroup })` to `toModel` and the emitters.
- A Biome/tsc import restriction (or a test) fails the build if any file outside `adapter/` and
  `plugin.ts` imports `@cloudflare/forge`.

### Consequences

- Good: Forge upgrades touch one directory; adapter contract tests run on every Forge bump.
- Good: emitters are pure functions of `SdkModel`, unit-testable without Forge.
- Bad: one more mapping layer to maintain, and fields Forge adds are invisible until mapped.
