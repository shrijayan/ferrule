# Phase 2 — Converter

Plan §16 Phase 2: stainless.yml validated against the public schema, editions,
`openapi.transforms`, resources, pagination, streaming and client settings written
as extensions. Oracle configs: ADR 0009.

## Tasks

| # | Task | Owner (files) | Independent |
|---|------|---------------|-------------|
| A | Validation against the public JSON schema (`converter/schema/stainless-config.schema.json`) with FER-coded diagnostics and pointers; editions and their defaults | agent-schema (`converter/src/schema/**`, `converter/src/editions/**`) | yes |
| B | `openapi.transforms` (every transform the public schema defines), pure, each with tests | agent-transforms (`converter/src/transforms/**` except `annotate.ts`) | yes |
| C | Extensions: `x-ferrule-client` (client_settings, environments, auth, retries, timeout, idempotency), `x-ferrule-streaming`, all pagination schemes, `$client` methods, subresources, method order (gap G11), model names (`x-fern-type-name`) | agent-extensions (`converter/src/transforms/annotate.ts`, new `converter/src/extensions/**`) | yes |
| D | Oracle config pairs per ADR 0009 in `oracles/manifest.yaml` + fetch support | agent-oracles (`oracles/**`) | yes |
| E | Wire A–C in `converter/src/index.ts` / `cli.ts`, snapshot every oracle config, run the gate | lead | after A–D |

## Exit gate

```bash
mise run check && mise run converter:oracles
```

`converter:oracles` fetches every `config` oracle and converts it with zero error
diagnostics, comparing against committed snapshots in `converter/test/__snapshots__/`.
