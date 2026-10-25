# path_params

Two methods with two path parameters each, reduced from honcho `workspaces.peers.card` and an
imagekit-style `delete` (phase 4 triage honcho/04). Stainless makes only the last path parameter
positional; the earlier ones are required keyword arguments, in path order, before the other
arguments, and lead the params `TypedDict` as `Required[...]` (honcho-python-core
`resources/workspaces/peers/peers.py`, `types/workspaces/peer_card_params.py`; imagekit-python
`resources/files/versions.py`; knock-mgmt-python `types/workflows/step_preview_template_params.py`).
A method with path parameters alone has no params file.

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; the
expected files are in `../golden/path-params/` and pinned by `../path_params.rs`.
