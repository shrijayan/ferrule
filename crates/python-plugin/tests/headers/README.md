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
