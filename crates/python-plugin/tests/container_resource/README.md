# container_resource

A resource with subresources and no methods of its own (groq-python `resources/chat/chat.py`):
its module imports neither `httpx` nor the request-option types, which only methods use.
`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
