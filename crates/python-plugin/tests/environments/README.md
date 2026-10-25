# environments

A client with several environments, reduced from honcho (`environments: {demo, local,
production}`, phase 4 triage honcho/03) with a nullable `api_key` read from the environment.
Stainless (lithic-python and honcho-python-core `_client.py` and `__init__.py`) writes:

- a module-level `ENVIRONMENTS: Dict[str, str]` in config order, first in `__all__`, re-exported
  from the package;
- `_environment: Literal[...] | NotGiven` on the class, `environment: Literal[...] | NotGiven =
  not_given` and `base_url: ... | NotGiven = not_given` on `__init__`;
- the base URL from an explicit `base_url`, else `environment` (an error beside
  `<PREFIX>_BASE_URL`), else that variable, else the first environment;
- `environment` on `copy()`, passed on as `environment or self._environment`.

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; the
expected files are in `../golden/environments/` and pinned by `../environments.rs`.
