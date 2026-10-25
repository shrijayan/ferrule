#!/usr/bin/env bash
# Check out Forge at the pinned commit into .cache/forge. ferrule-gen gets Forge through Cargo
# (Cargo.toml, the same commit); this checkout is for the weekly canary, which builds ferrule-gen
# against Forge's latest commit (FORGE_REF=latest).
set -euo pipefail

FORGE_REPO="https://github.com/cloudflare/forge.git"
# Keep in step with the `rev` of the forge dependency in Cargo.toml (ADR 0005, ADR 0014).
FORGE_PIN="91f52a40c746e8a2ac0271d8be0b7c82cdf28798"
# FORGE_REF=latest (forge-canary.yml) checks out Forge's main branch tip instead of the pin.
FORGE_REF="${FORGE_REF:-$FORGE_PIN}"

root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/.cache/forge"

grep -q "rev = \"$FORGE_PIN\"" "$root/Cargo.toml" || {
  echo "FORGE_PIN differs from the forge rev in Cargo.toml" >&2
  exit 1
}
if [ ! -d "$dir/.git" ]; then
  git clone --filter=blob:none "$FORGE_REPO" "$dir"
fi
if [ "$FORGE_REF" = "latest" ]; then
  git -C "$dir" fetch --quiet origin main
  git -C "$dir" checkout --quiet --detach FETCH_HEAD
else
  git -C "$dir" fetch --quiet origin "$FORGE_REF" || true
  git -C "$dir" checkout --quiet --detach "$FORGE_REF"
  test "$(git -C "$dir" rev-parse HEAD)" = "$FORGE_REF"
fi
echo "forge $(git -C "$dir" rev-parse --short HEAD) checked out: $dir"
