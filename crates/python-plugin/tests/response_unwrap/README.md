# response_unwrap

Response envelopes unwrapped (`settings.unwrap_response_fields: [data]`, Stainless). From ogx
`providers.list` (ogx-client-python `resources/providers.py`, `types/provider_list_response.py`,
`_wrappers.py`): the method returns the `data` field's type, named after the method when it is
not a model, through `post_parser=DataWrapper[...]._unwrapper` and
`cast_to=cast(Type[...], DataWrapper[...])`. `sessions.retrieve` is the opencode SDK's
`{location, data}` envelope (`unwrap_multiproperty_response_fields`), whose `data` is a model.
`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
