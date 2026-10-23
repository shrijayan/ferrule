#!/usr/bin/env bash
# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# Validate the action inputs, passed as INPUT_* environment variables (never interpolated).
set -euo pipefail

fail() { echo "::error::ferrule action: $*" >&2; exit 1; }

is_bool() { [[ "$1" == "true" || "$1" == "false" ]]; }

# A repository-relative path: no absolute paths, no `..`, no option-looking values.
is_rel_path() {
  [[ -n "$1" && "$1" != /* && "$1" != -* && "$1" != *..* && "$1" =~ ^[A-Za-z0-9._/@+-]+$ ]]
}

is_rel_path "${INPUT_SPEC:-}" || fail "spec must be a relative path inside the repository"
is_rel_path "${INPUT_CONFIG:-}" || fail "config must be a relative path inside the repository"
is_bool "${INPUT_VERIFY:-}" || fail "verify must be true or false"
is_bool "${INPUT_OPEN_PR:-}" || fail "open-pr must be true or false"
[[ "${INPUT_PYTHON_VERSION:-}" =~ ^3\.[0-9]+$ ]] || fail "python-version must look like 3.12"
[[ "${INPUT_FERRULE_VERSION:-}" == "source" || "${INPUT_FERRULE_VERSION:-}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([a-z0-9.+-]*)$ ]] \
  || fail "ferrule-version must be an exact version (1.2.3) or 'source'"
if [[ -n "${INPUT_AGAINST:-}" ]]; then
  [[ "$INPUT_AGAINST" != -* && "$INPUT_AGAINST" =~ ^[A-Za-z0-9._/~^@+-]+$ ]] \
    || fail "against must be a git ref"
fi
if [[ "${INPUT_OPEN_PR}" == "true" && -z "${GH_TOKEN:-}" ]]; then
  fail "open-pr: true needs the github-token input"
fi
echo "inputs ok"
