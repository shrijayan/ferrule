#!/usr/bin/env bash
# ADR 0006: fail if ferrule-gen, as built (Forge included, at the pin in Cargo.toml), depends on a
# network-capable crate. Checks the dependency tree we compile, not Forge's optional features.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
deny='^(reqwest|hyper|ureq|isahc|curl|surf|attohttpc|sentry|opentelemetry|tokio-tungstenite)$'
found="$(cd "$root" && cargo tree --locked -p ferrule-gen -e normal --prefix none \
  | awk '{print $1}' | sort -u | grep -E "$deny" || true)"
if [ -n "$found" ]; then
  echo "network-capable crates in ferrule-gen: $found" >&2
  exit 1
fi
echo "ferrule-gen (forge included): no network-capable crates"
