#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"
REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || echo '')"

if [[ -z "$REPO" ]]; then
  echo "Could not determine GitHub repo. Run from inside the repository or set REPO manually." >&2
  exit 1
fi

echo "Fetching latest release from $REPO..."
RELEASE_TAG="$(gh release view --repo "$REPO" --json tagName -q .tagName)"
echo "Latest release: $RELEASE_TAG"

mkdir -p "$BIN_DIR"

BINARIES=(
  "second-opinion-darwin-arm64"
  "second-opinion-darwin-x86_64"
  "second-opinion-linux-x86_64"
  "second-opinion-linux-aarch64"
)

for binary in "${BINARIES[@]}"; do
  echo "Downloading $binary..."
  gh release download "$RELEASE_TAG" \
    --repo "$REPO" \
    --pattern "$binary" \
    --dir "$BIN_DIR" \
    --clobber 2>/dev/null && chmod +x "$BIN_DIR/$binary" && echo "  ✓ $binary" || echo "  - $binary (not available for this release)"
done

echo "Done."
