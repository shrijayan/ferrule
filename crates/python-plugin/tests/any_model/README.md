# any_model

A named schema with no type (`Payload: {not: {type: null}}`, the opencode SDK's
`Session.Inbox.CompactionPayload`) holds any JSON value: an `object` alias, not an empty class.
A named `type: object` without properties stays `Dict[str, object]`.
`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
