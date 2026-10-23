#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Print the comma-separated labels that can carry the ferrule:allow-breaking override:
# the triggering PR's labels (from the event payload) plus those on the open
# ferrule/regenerate PR. Read-only.
set -euo pipefail

labels=()
if [[ -n "${GITHUB_EVENT_PATH:-}" && -f "$GITHUB_EVENT_PATH" ]]; then
  labels+=("$(jq -r '[.pull_request.labels[]?.name] | join(",")' "$GITHUB_EVENT_PATH")")
fi
if [[ -n "${GH_TOKEN:-}" ]]; then
  labels+=("$(gh pr list --head ferrule/regenerate --state open --json labels \
    --jq '[.[0].labels[]?.name] | join(",")' 2>/dev/null || true)")
fi
(IFS=,; echo "${labels[*]:-}")
