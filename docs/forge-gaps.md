# Forge plugin interface gaps

Pinned Forge `91f52a40`, interface `forge:plugin@0.3.0` (`.cache/forge/packages/forge/plugin/forge.wit`).
Each gap: what Ferrule needs, what the interface lacks, and the workaround. "Upstream" marks
gaps worth an issue on cloudflare/forge (queued in [HUMAN-ACTIONS.md](HUMAN-ACTIONS.md); not filed).

| # | Need | Interface today | Workaround | Upstream |
|---|------|-----------------|------------|----------|
| G1 | Group operations by stainless.yml resources | Groups come only from `x-fern-sdk-group-name`; an operation without it makes no group, and `forge.groups` is empty | Converter writes `x-fern-sdk-group-name` / `x-fern-sdk-method-name` (ADR 0003 amendment) | No (works as designed) |
| G2 | Root-level settings (package name, client, environments, auth) | `forge-info` has only `version`, `description`, `base-url`; root `x-` extensions are dropped | Mirror root `x-ferrule-*` values onto every operation the plugin reads (`x-ferrule-package`, …) | Yes: expose document extensions |
| G3 | Schema-level names and settings (`x-ferrule-model-name`) | `model-info` and `field-info` carry no extensions; only `x-fern-type-name` / `x-fern-property-name` are read | Use `x-fern-type-name` for names; mirror other model settings per method in `x-ferrule-models` | Yes: pass schema extensions like method extensions |
| G4 | Exact spelling of names (`$shared`, acronyms, `list` vs `list_`) | `name` is a list of lower-case words | Carry spelling in `x-ferrule-resource` / `x-ferrule-method-name` and use those over `Name` | No |
| G5 | Servers / environments list | Only one `base-url` | `x-ferrule-client.environments` (mirrored per G2) | Yes: expose all servers |
| G6 | Security schemes (api-key header/query, bearer) | Not exposed | `x-ferrule-client.auth` from stainless.yml `client_settings` | Yes |
| G7 | Client-wide files from group data (api.md, `_client.py` resource list) | `generate` sees all groups; `generate-group` runs isolated per group | Emit client-wide files from `generate` only (plan §3 note) | No |
| G8 | Formatting (ruff) and file reads (runtime copy) | Sandboxed: no fs, clock, process | Stage 04 "Finish" in Python after Forge (plan §7) | No |

Further gaps found while building the spike are appended below by the plugin and converter work.
