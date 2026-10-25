# streaming

Streaming methods, one directory per shape. Each holds an `openapi.yml` and `stainless.yml`
reduced from an oracle (phase 4 triage), and `annotated.json`, which is
`ferrule-gen convert openapi.yml stainless.yml -o annotated.json`. The expected Python is in
`../golden/streaming/<shape>/` and pinned by `../streaming.rs`.

- `param_switched`: `streaming.param_discriminator`, from stagehand `sessions.act`
  (stagehand-python `resources/sessions.py`, `types/session_act_params.py`; switch
  `streamResponse`, event model `sessions.stream_event`) and ogx `completions.create`
  (ogx-client-python `resources/completions.py`, `types/completion_create_params.py`; a nullable
  switch `stream`, no event model, so it streams the response type). Stainless writes:
  - three `@overload`s with the method docstring: the switch `Literal[False] | Omit = omit`
    (`Optional[Literal[False]]` when nullable) in its place, returning the response; the switch
    `Literal[True]` and required, right after the required arguments (the docstring follows that
    order), returning `Stream[<event>]`; the same with `bool`, returning either;
  - `@required_args([<required>], [<required>, <switch>])`;
  - the implementation without a docstring, the switch typed `Literal[False] | Literal[True]`,
    the body checked against `<Params>Streaming if <switch> else <Params>NonStreaming` (split
    before `if` and `else` past 120 columns), `cast_to=<response>`,
    `stream=<switch> or False`, `stream_cls=Stream[<event>]`;
  - the params file: `<Params>Base` without the switch, its inline objects, then
    `<Params>NonStreaming(<Params>Base, total=False)` with the switch `Literal[False]`,
    `<Params>Streaming(<Params>Base)` with it `Required[Literal[True]]`, and
    `<Params> = Union[<Params>NonStreaming, <Params>Streaming]`; `__all__` lists the base, the
    inline objects and the two classes.

  The fixture's event model is reached through a `text/event-stream` response, as ogx's are;
  stagehand's `StreamEvent` is in no operation at all, which Forge drops (it keeps only schemas
  some operation uses).
