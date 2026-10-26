# request_option_clash

A query parameter and a body field named `timeout`, and a body field named `self`, reduced from
docker `ContainerStop`, twilio `CreateCall` and jira (Tier 1 corpus run, 2026-10-26). Stainless
names a parameter that would shadow a per-request option `api_<name>` and keeps the wire name as
an alias (`api_timeout: Annotated[int, PropertyInfo(alias="timeout")]` in browserbase-python
`session_create_params.py` and parallel-sdk-python `task_run_result_params.py`, a query
parameter). `self` follows the same rule (ADR 0020).

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; pinned by
`../param_clashes.rs`.
