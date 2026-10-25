# request_bodies

Request bodies that are not a plain object, one directory per shape. Each holds an `openapi.yml`
and `stainless.yml` reduced from an oracle (phase 4 triage), and `annotated.json`, which is
`ferrule-gen convert openapi.yml stainless.yml -o annotated.json`. The expected Python is in
`../golden/request-bodies/<shape>/` and pinned by `../request_bodies.rs`.

- `nullable_object`: `anyOf: [$ref WorkspaceGet, null]`, FastAPI's optional body. From honcho
  `workspaces.list` (honcho-python-core `resources/workspaces/workspaces.py`,
  `types/workspace_list_params.py`): the object's properties are keyword arguments, sent as
  `body=maybe_transform({"filters": filters}, ...)`.
