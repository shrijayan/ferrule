# headers

Request headers, one directory per shape. Each holds an `openapi.yml` and `stainless.yml`
reduced from an oracle (phase 4 triage), and `annotated.json`, which is
`ferrule-gen convert openapi.yml stainless.yml -o annotated.json`. The expected Python is in
`../golden/headers/<shape>/` and pinned by `../headers.rs`.

- `params`: header parameters, from ogx `models.list` and `models.retrieve`
  (ogx-client-python `resources/models/models.py`, `types/model_list_params.py`; four optional
  string headers, no descriptions) and stagehand `sessions.act` and `sessions.end`
  (stagehand-python `resources/sessions.py`, `types/session_act_params.py`; the optional
  `x-stream-response` header, a string enum). Stainless writes:
  - each header as a keyword argument (`<type> | Omit = omit` when optional) after the query and
    body arguments, sorted by wire name, in every `@overload` of a param-switched method too;
    in the docstring after the others when the spec describes it;
  - in the params `TypedDict`, after the other fields, `Annotated[<type>,
    PropertyInfo(alias="<header>")]` (in `<Params>Base` for a param-switched method);
  - no params file for a method whose only keyword arguments are headers (ogx
    `models.retrieve`, stagehand `sessions.end`);
  - after the path checks, `extra_headers = {**strip_not_given({...}), **(extra_headers or
    {})}`, a `str` header as is, any other as `str(x) if is_given(x) else not_given`.
- `auth_opts`: client options that are API keys sent as headers, from stagehand
  (`stainless.yml` `client_settings.opts` `BROWSERBASE_API_KEY`, `BROWSERBASE_PROJECT_ID`,
  `MODEL_API_KEY`, all three schemes in one security requirement; stagehand-python
  `_client.py`, whose generated parts are outside its `CUSTOM CODE` blocks) and papr
  (papr-python `_client.py`: three requirements, one scheme each). Stainless writes:
  - each option under the `snake_case` of its config key (`browserbase_api_key`,
    `x_session_token` for `X-Session-Token`) in the class annotations, `__init__`, its
    docstring and errors, and `copy`; the environment variable as written;
  - with more than one scheme to send, `auth_headers` returning `{**self._<scheme>, ...}`, the
    requirements last to first (the first wins), the schemes of one requirement in order, and
    after it one property per scheme, named `_` + the `snake_case` of the scheme name
    (`_bb_api_key_auth`, papr `_x_api_key`), returning `{}` for an unset nullable option;
  - with one scheme, the header inline in `auth_headers`, as before.
