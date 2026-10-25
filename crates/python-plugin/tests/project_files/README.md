# project_files

Inputs of `tests/project_files_oracles.rs`, which compares the project files (README.md,
`tests/test_client.py`, `tests/conftest.py`, scripts, CI) with config oracles whose credentials
OneBusAway's (an API key query parameter) do not cover. Each directory holds:

- `annotated.json`: `ferrule-gen convert <spec> <stainless.yml>` of the config oracle;
- `meta.json`: what `x-ferrule-config` does not carry yet (`ProjectMeta`): the version of the
  pinned SDK, the `organization` values, the README title and `readme.example_requests.default`,
  `readme.include_stainless_attribution` and `settings.disable_mock_tests`. The `.stats.yml`
  hashes are read from the fetched oracle.

| Oracle | Config (oracles/manifest.yaml) | SDK | Credentials | Fixture |
|--------|--------------------------------|-----|-------------|---------|
| isaacus | isaacus-dev/openapi @ ee98098 (Apache-2.0) | isaacus-python 0.22.0 | bearer `api_key` | the whole spec |
| lumaai | lumalabs/lumaai-api @ 5834633 | lumaai-python 1.21.0 | bearer `auth_token` | reduced |
| imagekit | imagekit-developer/sdk-generation @ 28534c1 | imagekit-python 5.5.0 | basic, `private_key` + `password` | reduced |

The config repositories of lumaai and imagekit carry no licence, so their fixtures keep only the
README example's endpoint (`post /generations/video`, `post /api/v1/files/upload`), the schemas it
reaches and the client settings, without descriptions or examples: the names and types their
Apache-2.0 SDKs publish. The test then leaves out the files that list every endpoint.
