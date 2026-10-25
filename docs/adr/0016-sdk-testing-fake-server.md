---
status: Accepted
date: 2026-10-23
deciders: Ferrule maintainers
---

# 0016. Ship a `<pkg>.testing` fake server in generated SDKs

## Context and problem statement

Plan §5 item 5 asks for a way to run SDK code without network access or API keys, so users,
CI and coding agents can exercise an SDK safely. Stainless SDKs ship nothing like this, so any
such module is a deviation from the oracle output and needs a record.

## Decision drivers

- Stainless output is the spec: existing files must stay byte-identical; only additions allowed.
- No new runtime dependencies and deterministic responses (no clocks, no randomness).
- Per-API data (routes, response schemas) comes from the plugin; behaviour lives in the runtime.

## Considered options

1. An httpx transport rendered from the runtime template, reading a plugin-emitted `_routes.json`.
2. Generating a fake per operation in python-plugin (Python code per route).
3. Recommending a third-party mock library (respx, prism).

## Decision outcome

Chosen option 1. `runtime/template/testing/` holds `__init__.py` and `_fake.py`; `render_files`
emits them only with `testing=True`, so the existing byte-parity tests and the public API of
every oracle are unchanged unless the option is set, and then only gain the new `testing`
subpackage. The generated `__init__.py` does not import it. Responses come from the response
schema: `example`, `examples[0]`, `default`, `const`, `enum[0]`, then type defaults; optional
properties without examples are left out so paginators stop after one page.

### Consequences

- Good: one implementation, tested once in `runtime/tests`; plugin only emits JSON data.
- Good: additive public API (`<pkg>.testing.*`); no change to any Stainless-identical file.
- Bad: the module is Ferrule-only output; adopting SDKs gain a subpackage Stainless never had.

## More information

Plan §5 item 5. Data contract: `runtime/README.md` ("Testing fake server").
