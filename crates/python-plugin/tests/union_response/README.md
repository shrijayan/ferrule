# union_response

A method returning a named union model (knock-mgmt-python `audiences.retrieve` and
`audiences.list`): a union is not a type the request helpers take, so the call casts it,
`cast(Audience, self._get(..., cast_to=cast(Any, Audience),  # Union types ...))`, and a page
passes `model=cast(Any, Audience)` with the same comment; a stream of them (the Stainless
opencode SDK's `event.list`) passes `cast_to=cast(Any, ...)` alone. Reduced from the opencode SDK's
`migration.v1.status` and `session.message.list`.
`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
