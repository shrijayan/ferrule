# location_clash

Path or query parameters with the name of a body field, reduced from github
`actions.update_environment_variable` (`name`), kubernetes `deleteNamespacedPod` (`dryRun`) and
cloudflare `resource_sharing.recipients.create` (`account_id`) (Tier 1 corpus run, 2026-10-26).
Stainless prefixes every clashing parameter with its location and keeps the wire name as an
alias: gcore-python `dns.zones.replace(path_name, *, body_name, ...)` with
`body_name: Required[Annotated[str, PropertyInfo(alias="name")]]` and the path template
`/dns/v2/zones/{path_name}`; cloudflare-python `recipients.create(share_id, *, path_account_id,
body_account_id, ...)` (ADR 0020).

`annotated.json` is `ferrule-gen convert openapi.yml stainless.yml -o annotated.json`; pinned by
`../param_clashes.rs`.
