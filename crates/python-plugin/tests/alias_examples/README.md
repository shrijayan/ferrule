# alias_examples

A path parameter typed by a named string alias (`$shared.models.session_id: Session.ID`), as the
opencode SDK's IDs are. The example call in the API tests and the agent recipe passes a string
(the wire name), not `{}`: the recipe test runs it, and a dict fails the client's path check.
`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`.
