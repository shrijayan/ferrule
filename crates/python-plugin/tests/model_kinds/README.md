# model_kinds

Named models (`models:` in `stainless.yml`) whose schema is not an object: a union of named
models with a discriminator (`Audience`), a union of inline objects (`ConditionGroup`), a string
enum (`StreamingResolution`), an array of a discriminated union (`Extensions`), a map
(`ChannelData`) and a recursive union (`WorkflowStep` through `WorkflowBranchStep`). Reduced from
the knock-mgmt, imagekit and knock oracles (phase 4 triage: knock-mgmt/01, knock/01, knock/02).

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml`. The expected Python is in
`../golden/model-kinds/` and pinned by `../model_kinds.rs`.
