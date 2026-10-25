# generator_eras

Two resources (one with a subresource) and three methods, reduced from honcho and knock, run
through the plugin with each `x-ferrule-style` an adopted SDK's runtime selects. Older
Stainless generators emitted three things differently, and an SDK keeps the spelling its own
runtime supports:

- `resources: eager`: client resources and `with_raw_response`/`with_streaming_response` are
  attributes declared on the class and set at the end of `__init__`, the wrappers set them in
  `__init__` too (honcho-python-core and papr-python `_client.py`, before "speedup initial
  import" in 2025-12).
- `paths: fstring`: paths are f-strings, not `path_template(...)`; the runtime has no
  `_utils/_path.py` (honcho-python-core `resources/workspaces/workspaces.py`, before "sanitize
  endpoint path params" in 2026-03).
- `sentinel: NOT_GIVEN`: optional arguments default to `NOT_GIVEN` (`| NotGiven`), the client's
  `__init__` spells `timeout` as `Union[float, Timeout, None, NotGiven]`, and the package
  exports no `omit`/`not_given`; the runtime's `_types.py` defines neither (knock-python and
  opencode-sdk-python, before "change optional parameter type from NotGiven to Omit" in
  2025-09).

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; the test
adds `x-ferrule-style` to every operation. The expected files are in
`../golden/generator-eras/` and pinned by `../generator_eras.rs`.
