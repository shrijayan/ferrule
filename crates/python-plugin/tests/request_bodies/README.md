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
- `union_of_objects`: a `oneOf` of two `$ref`s beside query parameters, from knock-mgmt
  `guides.activate` (knock-mgmt-python `resources/guides.py`, `types/guide_activate_params.py`),
  and a discriminated `oneOf` of inline objects behind a named model, from imagekit
  `accounts.origins.create` (imagekit-python `resources/accounts/origins.py`,
  `types/accounts/origin_create_params.py`). Stainless writes:
  - one `@overload` per variant, in spec order: the query parameters and the variant's fields,
    sorted as any params (required first), with the method docstring listing them; body `...`;
  - `@required_args` with each variant's required arguments, identical lists once, when any
    variant has one;
  - the implementation without a docstring: every variant's arguments once, in the order they
    first appear, required only where every variant requires them, typed as each variant types
    them (`Literal["S3"] | Literal["WEB_PROXY"]`); the body dict and query dict in that order;
  - the params file: a `TypedDict` per variant (query parameters included) named after the
    schema it comes from, or its discriminator value, each followed by its inline objects
    (`S3Credentials`, as imagekit `ChangePublicationStatusPublish`), then
    `OriginCreateParams: TypeAlias = Union[...]`.

  Stainless names inline variants after their `title` (imagekit `GoogleCloudStorageGcs` for
  discriminator `GCS`, `UpdateFileDetails`); Forge does not expose titles, so the fixture's
  titles spell what the discriminator values give.
- `multipart`: a `multipart/form-data` body with a binary field, from honcho
  `sessions.messages.upload` (honcho-python-core `resources/workspaces/sessions/messages.py`;
  current-era text from ogx-client-python `resources/files.py`): the body dict goes through
  `deepcopy_with_paths(..., [["file"]])`, `extract_files(..., paths=[["file"]])` takes the files
  out, `Content-Type: multipart/form-data` is set, and `files=files` is passed beside the body.
  Only the request is pinned (in `../request_bodies.rs`, no golden files): Stainless types the
  binary field `FileTypes`, which needs a type the project-file emitters know too.
- `binary`: an `application/octet-stream` body beside a query parameter, from the opencode
  SDK's `file.write` (`POST /api/experimental/fs/write`, spec from ADR 0015). No oracle SDK has
  one, so the expected text follows the runtime: `post`/`put`/`patch` take raw bytes as
  `content=` (bytes as `body=` are deprecated there) and `_files.read_file_content` reads a
  path. The argument is the one required keyword `body`, as for a map or array body; it is not
  in the params file.
- `option_names`: a body field `timeout` and a query parameter `extra_query`, named like the
  request options every method takes, from the opencode SDK's `shell.create`. Without a rename
  the method has two `timeout` arguments, a `SyntaxError`. Stainless prefixes such a name with
  `api_` (browserbase-python `sessions.create(api_timeout=...)` sends `"timeout"`); the params
  `TypedDict` keeps the spec's name.
