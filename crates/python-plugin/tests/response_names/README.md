# response_names

Two responses with a nested `Summary | null`, reduced from honcho `sessions.summaries` and
`sessions.get_context`. When the response is a `$ref` to a components schema that the config
does not name, Stainless names its nested classes after the field alone (honcho-python-core
`types/workspaces/session_summaries_response.py`: `LongSummary`; imagekit-python
`types/file_upload_response.py`: `AITag`). An inline response prefixes them with its own name
(`SessionContextResponseSummary`, as OneBusAway's `StopRetrieveResponseData`).

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; the
expected files are in `../golden/response-names/` and pinned by `../response_names.rs`.
