# standalone

Reduced from the isaacus oracle (isaacus-dev/openapi `config/stainless.yaml` at ee98098, paired
with isaacus-dev/isaacus-python 0.22.0): two `standalone_api` subresources, one with a method
(`extractions.qa`) and one with only models (`ilgs.v1`), and the `custom_casings` isaacus uses
(`qa` and `ilgs` initialisms, `v1` spelled as written). The organization name `acme-qa` checks
that the client class takes the casing too (`AcmeQA`, as lumaai's `LumaAI`). The API key is a
query parameter so the project files (api.md, tests) are emitted as well.

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; the
tests in `tests/standalone.rs` read it.
