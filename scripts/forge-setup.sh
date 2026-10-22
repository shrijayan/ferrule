#!/usr/bin/env bash
# Clone @cloudflare/forge at the pinned commit and build it from source.
# Forge is not published to npm yet (ADR 0005), so this is the pin.
set -euo pipefail

FORGE_REPO="https://github.com/cloudflare/forge.git"
FORGE_PIN="91f52a40c746e8a2ac0271d8be0b7c82cdf28798"
# FORGE_REF=latest (forge-canary.yml) builds Forge's main branch tip instead of the pin.
FORGE_REF="${FORGE_REF:-$FORGE_PIN}"

root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/.cache/forge"

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

cargo build --manifest-path "$dir/Cargo.toml" --release --locked --features wasm
# The TypeScript plugin library (@cloudflare/forge/plugin, /ts) and its WIT.
(cd "$dir" && pnpm install --frozen-lockfile)
(cd "$dir/packages/forge" && node scripts/build.mjs)
echo "forge $(git -C "$dir" rev-parse --short HEAD) built: $dir/target/release/forge"
