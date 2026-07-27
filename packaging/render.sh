#!/usr/bin/env bash
#
# Renders the Scoop manifest and Homebrew formula for a release.
#
#   packaging/render.sh <version> <SHA256SUMS> <output-dir>
#
# Reads the checksums the release workflow already publishes, so the manifests
# cannot claim a hash that was never built. Run by the `packaging` job in
# release.yml, and directly to inspect the output without cutting a release.

set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <version> <SHA256SUMS> <output-dir>" >&2
  exit 2
fi

VERSION="${1#v}"
SUMS="$2"
OUT="$3"

if [ ! -f "$SUMS" ]; then
  echo "error: no SHA256SUMS at $SUMS" >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# `shasum -a 256 anesis-*` writes "<hash>  <file>", so the archive is field 2.
sum_for() {
  local archive="$1" hash
  hash="$(awk -v want="$archive" '$2 == want { print $1 }' "$SUMS")"
  if [ -z "$hash" ]; then
    echo "error: $archive is missing from $SUMS" >&2
    exit 1
  fi
  printf '%s' "$hash"
}

SHA256_LINUX_X86_64="$(sum_for anesis-linux-x86_64.tar.gz)"
SHA256_LINUX_AARCH64="$(sum_for anesis-linux-aarch64.tar.gz)"
SHA256_MACOS_X86_64="$(sum_for anesis-macos-x86_64.tar.gz)"
SHA256_MACOS_AARCH64="$(sum_for anesis-macos-aarch64.tar.gz)"
SHA256_WINDOWS_X86_64="$(sum_for anesis-windows-x86_64.zip)"

render() {
  sed \
    -e "s|{{VERSION}}|${VERSION}|g" \
    -e "s|{{SHA256_LINUX_X86_64}}|${SHA256_LINUX_X86_64}|g" \
    -e "s|{{SHA256_LINUX_AARCH64}}|${SHA256_LINUX_AARCH64}|g" \
    -e "s|{{SHA256_MACOS_X86_64}}|${SHA256_MACOS_X86_64}|g" \
    -e "s|{{SHA256_MACOS_AARCH64}}|${SHA256_MACOS_AARCH64}|g" \
    -e "s|{{SHA256_WINDOWS_X86_64}}|${SHA256_WINDOWS_X86_64}|g" \
    "$1"
}

mkdir -p "$OUT/bucket" "$OUT/Formula"
render "$script_dir/scoop/anesis.json" >"$OUT/bucket/anesis.json"
render "$script_dir/homebrew/anesis.rb" >"$OUT/Formula/anesis.rb"

# A leftover placeholder means a template gained a field the substitution above
# does not know about — better to fail here than to publish a broken manifest.
if grep -rq '{{' "$OUT/bucket/anesis.json" "$OUT/Formula/anesis.rb"; then
  echo "error: unsubstituted placeholders remain:" >&2
  grep -rn '{{' "$OUT/bucket/anesis.json" "$OUT/Formula/anesis.rb" >&2
  exit 1
fi

echo "Rendered anesis $VERSION into $OUT"
