#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Print the comma-separated labels that may carry the ferrule:allow-breaking override.
# Only the PR that triggered this run counts, and only when its head is in this
# repository (never a fork). No other PR is consulted. Read-only, no network.
set -euo pipefail

if [[ -z "${GITHUB_EVENT_PATH:-}" || ! -f "$GITHUB_EVENT_PATH" ]]; then
  echo ""
  exit 0
fi
jq -r '
  .pull_request // empty
  | select(.number != null)
  | select(.head.repo.full_name != null and .head.repo.full_name == .base.repo.full_name)
  | [.labels[]?.name] | join(",")
' "$GITHUB_EVENT_PATH"
