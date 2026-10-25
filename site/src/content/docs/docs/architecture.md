---

title: Architecture
description: Five stages; Forge is stage two.
---

**Inputs:** OpenAPI spec · `stainless.yml` · `.ferrule/patches/` · previous release
**Outputs:** Python SDK · `api.md` · `.stats.yml` · agent context · `ferrule.lock` · verify report

| Stage      | What happens                                                                                                              | Part                       |
| ---------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| 01 Convert | Validate `stainless.yml`, apply editions and `openapi.transforms`, write an annotated spec with `x-ferrule-*` extensions. | converter (TS)             |
| 02 Forge   | Pinned `@cloudflare/forge` parses the annotated spec and runs the plugin.                                                 | dependency                 |
| 03 Emit    | Adapter maps Forge data to our model; emitters write the SDK files.                                                       | python-plugin (TS to WASM) |
| 04 Finish  | Copy runtime, format with ruff, merge custom code onto the generated branch.                                              | migrate (Python)           |
| 05 Verify  | API diff, type-check, tests against the mock, semver verdict.                                                             | migrate (Python)           |

```ts
// python-plugin/src/adapter.ts: the only file that imports Forge types
import { definePlugin } from '@cloudflare/forge/plugin';
import { toModel } from './adapter/to-model'; // Forge -> SdkModel
import { emitClient, emitGroup } from './emit'; // SdkModel -> files

export const { generate, generateGroup } = definePlugin({
  generate: (forge) => emitClient(toModel.client(forge)),
  generateGroup: (group) => emitGroup(toModel.group(group)),
});
```

## Plugin limits we design around

Plugins run sandboxed (no files, network or clock), so ruff formatting and checks run after
Forge. Each group is generated in isolation, so client-wide files come from the one-time generate
call. Plugins see only the spec, so all `stainless.yml` settings travel as extensions or overlays
(see ADR 0003).
