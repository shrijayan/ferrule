# param_twins

Named models used in a request: Stainless types the request with a `TypedDict` twin of each
model (knock-mgmt-python `types/chat_template_param.py` beside `types/chat_template.py`, and
`types/shared_params/message_type_boolean_field.py` for a `$shared` model, same class name),
so callers pass dicts whose keys go out under their wire names (`provider_id` ->
`providerID`). Twins name the twins of the models they reference. The opencode SDK's
`session.create(model=...)` sent `provider_id` before this. `annotated.json` is
`ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
