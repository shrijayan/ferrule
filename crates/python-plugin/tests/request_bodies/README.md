# request_bodies

Request bodies that are not a plain object, one directory per shape. Each holds an `openapi.yml`
and `stainless.yml` reduced from an oracle (phase 4 triage), and `annotated.json`, which is
`ferrule-gen convert openapi.yml stainless.yml -o annotated.json`. The expected Python is in
`../golden/request-bodies/<shape>/` and pinned by `../request_bodies.rs`.

- `nullable_object`: `anyOf: [$ref WorkspaceGet, null]`, FastAPI's optional body. From honcho
  `workspaces.list` (honcho-python-core `resources/workspaces/workspaces.py`,
  `types/workspace_list_params.py`): the object's properties are keyword arguments, sent as
  `body=maybe_transform({"filters": filters}, ...)`.
- `map_and_array`: a map body and an array body. From honcho `sessions.peers.add` and
  `sessions.peers.remove` (honcho-python-core `resources/workspaces/sessions/peers.py`,
  `types/workspaces/sessions/peer_add_params.py`, `peer_remove_params.py`): one required keyword
  argument `body`, described by the body schema; a map is sent as
  `maybe_transform(body, peer_add_params.PeerAddParams)`, an array as
  `maybe_transform(body, SequenceNotStr[str])` without importing its params module. honcho's map
  values are the named model `SessionPeerConfig` (`SessionPeerConfigParam` in Stainless); the
  fixture maps to strings, since `_param` twins of named models are not emitted yet.
