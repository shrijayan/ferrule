#!/usr/bin/env bash
# Open (or comment on) a tracking issue when a scheduled workflow fails.
# Usage: open-issue.sh <title> <label> <run-url>
set -euo pipefail
title="$1"
label="$2"
run_url="$3"
body="Scheduled run failed: ${run_url}

Triage: find the FER code in the job log, then follow docs/phases for the owning area."
existing=$(gh issue list --state open --label "$label" --search "in:title \"$title\"" --json number --jq '.[0].number // empty')
if [ -n "$existing" ]; then
  gh issue comment "$existing" --body "$body"
else
  gh label create "$label" --force >/dev/null 2>&1 || true
  gh issue create --title "$title" --label "$label" --body "$body"
fi
