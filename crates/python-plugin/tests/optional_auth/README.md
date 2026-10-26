# optional_auth

Credentials that are all optional (`nullable: true`), as the opencode SDK's HTTP basic auth and
ogx-client-python's `api_key` are. The client builds without them, so `tests/test_client.py`
has no `test_validate_headers`, which expects the constructor to raise (ogx-client-python's
tests have none; imagekit-python's, whose `private_key` is required, do).
`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
