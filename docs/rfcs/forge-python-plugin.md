# RFC draft: Python SDK target and Stainless migration tooling on Forge

> Draft text for an issue on cloudflare/forge. Not posted yet (see docs/HUMAN-ACTIONS.md).

**Title:** RFC: Python plugin and Stainless-migration tooling built on Forge

Hi Forge team. Thank you for releasing Forge under Apache-2.0.

We are starting **Ferrule**, an Apache-2.0 project that helps teams with existing
Stainless-generated Python SDKs keep generating the same SDK after the Stainless wind-down. We
want to build on Forge as a pinned dependency rather than write another generator, and we would
like to coordinate early. This relates to RFC #45.

## What we plan to build

1. **A converter** that reads `stainless.yml` and writes Forge overlays plus vendor extensions
   (`x-ferrule-*`: pagination, streaming, client settings, resource placement, method and model
   names) onto the spec.
2. **A Forge WASM plugin** (`forge:plugin@0.3.0`, TypeScript) that emits a Python SDK identical in
   public surface to the Stainless output: sync and async clients, resources, TypedDict params,
   pydantic models, pagination and SSE streaming.
3. **Migration tools** (Python): spec rescue, adoption, public-API diff against the last Stainless
   release, and three-way merge of custom code.

Forge types stay behind a small adapter in the plugin, and we pin Forge by commit with adapter
contract tests plus a weekly canary against your latest release.

## Questions

- Is a Python target on your roadmap (for example for cloudflare-python)? If so, we would rather
  contribute our plugin than compete with it.
- Would you accept upstream issues or PRs for gaps we find in the plugin interface during our
  spike (we will report them in one list)?
- Are vendor extensions guaranteed to pass through to plugins unchanged, and is there a preferred
  prefix convention?
- Does Forge collect any telemetry, and is there a supported way to disable it?
- Would you be interested in hosting migration tooling or the Python plugin under the Forge
  organization later, once it is proven?

We'll keep everything Apache-2.0, credit Forge clearly, and avoid using the Cloudflare or Forge
names in our product names. Happy to discuss here or in whatever channel you prefer.
