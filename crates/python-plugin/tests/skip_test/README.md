# skip_test

`skip_test_reason` on a method (Stainless; knock-python's `users.bulk.set_preferences`): each of
its generated API tests is marked `@pytest.mark.skip(reason=...)`, wrapped the way ruff wraps
it. Reduced from the opencode SDK, whose integer query parameters behind a `$ref` the mock
server rejects. `annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
