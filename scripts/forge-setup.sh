#!/usr/bin/env bash
# Clone @cloudflare/forge at the pinned commit and build it from source.
# Forge is not published to npm yet (ADR 0005), so this is the pin.
set -euo pipefail

FORGE_REPO="https://github.com/cloudflare/forge.git"
FORGE_COMMIT="91f52a40c746e8a2ac0271d8be0b7c82cdf28798"

root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/.cache/forge"

if [ ! -d "$dir/.git" ]; then
  git clone --filter=blob:none "$FORGE_REPO" "$dir"
fi
git -C "$dir" fetch --quiet origin "$FORGE_COMMIT" || true
git -C "$dir" checkout --quiet --detach "$FORGE_COMMIT"
test "$(git -C "$dir" rev-parse HEAD)" = "$FORGE_COMMIT"

cargo build --manifest-path "$dir/Cargo.toml" --release --locked --features wasm
echo "forge $(git -C "$dir" rev-parse --short HEAD) built: $dir/target/release/forge"
