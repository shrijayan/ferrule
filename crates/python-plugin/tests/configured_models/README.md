# configured_models

A model `stainless.yml` names whose schema only an error response reaches (the opencode SDK's
`…ErrorBody` types, ADR 0015). Stainless generates every model the config lists, so it is
declared and exported like `Thing`, which the success response reaches. `annotated.json` is
`ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
