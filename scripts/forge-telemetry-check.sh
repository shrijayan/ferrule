#!/usr/bin/env bash
# Fail if the pinned Forge build gains a network-capable dependency (ADR 0006).
set -euo pipefail
dir="$(cd "$(dirname "$0")/.." && pwd)/.cache/forge"
deny='^(reqwest|hyper|ureq|isahc|curl|surf|attohttpc|sentry|opentelemetry|tokio-tungstenite)$'
found="$(cargo tree --manifest-path "$dir/Cargo.toml" --features wasm -e normal --prefix none \
  | awk '{print $1}' | sort -u | grep -E "$deny" || true)"
if [ -n "$found" ]; then
  echo "network-capable crates in forge: $found" >&2
  exit 1
fi
echo "forge: no network-capable crates"
