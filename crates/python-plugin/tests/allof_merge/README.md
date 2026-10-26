# allof_merge

One body whose `allOf` names the same property three times, once in a nested `allOf`, reduced
from openai `CreateChatCompletionRequest.top_logprobs` (Tier 1 corpus run, 2026-10-26). The
property is one argument and one `TypedDict` key, typed and described by the last member that
declares it, as Stainless prints it (openai-python `chat/completions.py`:
`top_logprobs: Optional[int]`).

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; pinned by
`../param_clashes.rs`.
