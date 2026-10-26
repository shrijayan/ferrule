#!/bin/sh
# Install ferrule-gen from a GitHub release of shrijayan/ferrule (ADR 0021).
#
#   curl -LsSf https://ferrule.cpluz.com/install.sh | sh
#
# FERRULE_VERSION      a release tag such as v0.2.0 (default: the latest release)
# FERRULE_INSTALL_DIR  where to put the binary (default: ~/.local/bin)
set -eu

REPO="shrijayan/ferrule"

say() { printf 'ferrule: %s\n' "$*"; }
err() {
  printf 'ferrule: error: %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || err "this installer needs $1"; }

download() {
  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -fsSL "$1" -o "$2" || err "could not download $1"
  elif command -v wget >/dev/null 2>&1; then
    wget --https-only -q "$1" -O "$2" || err "could not download $1"
  else
    err "this installer needs curl or wget"
  fi
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d ' ' -f 1
  else
    err "this installer needs sha256sum or shasum to check the download"
  fi
}

main() {
  need uname
  need tar
  need mktemp

  case "$(uname -s)" in
    Linux) os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    *) err "no build for $(uname -s); on Windows, download the zip from https://github.com/$REPO/releases" ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) err "no build for $(uname -m)" ;;
  esac
  # A shell running under Rosetta reports x86_64; use the native build.
  if [ "$os" = apple-darwin ] && [ "$arch" = x86_64 ] &&
    [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = 1 ]; then
    arch="aarch64"
  fi

  name="ferrule-gen-$arch-$os"
  archive="$name.tar.gz"
  version="${FERRULE_VERSION:-latest}"
  if [ "$version" = latest ]; then
    base="https://github.com/$REPO/releases/latest/download"
  else
    base="https://github.com/$REPO/releases/download/$version"
  fi
  dir="${FERRULE_INSTALL_DIR:-$HOME/.local/bin}"

  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT INT TERM

  say "downloading $archive ($version)"
  download "$base/$archive" "$tmp/$archive"
  download "$base/$archive.sha256" "$tmp/$archive.sha256"
  expected="$(cut -d ' ' -f 1 <"$tmp/$archive.sha256")"
  actual="$(sha256 "$tmp/$archive")"
  [ "$expected" = "$actual" ] || err "checksum mismatch for $archive: expected $expected, got $actual"

  tar -xzf "$tmp/$archive" -C "$tmp"
  mkdir -p "$dir"
  cp "$tmp/$name/ferrule-gen" "$dir/ferrule-gen.tmp"
  chmod 755 "$dir/ferrule-gen.tmp"
  mv "$dir/ferrule-gen.tmp" "$dir/ferrule-gen"
  say "installed $("$dir/ferrule-gen" --version) to $dir/ferrule-gen"

  case ":$PATH:" in
    *":$dir:"*) ;;
    *) say "add $dir to your PATH, for example: export PATH=\"$dir:\$PATH\"" ;;
  esac
  if ! command -v ruff >/dev/null 2>&1; then
    say "Python SDKs are formatted with ruff: curl -LsSf https://astral.sh/ruff/install.sh | sh"
  fi
  say "next: ferrule-gen generate --spec openapi.yml --config stainless.yml --out sdk"
}

main "$@"
