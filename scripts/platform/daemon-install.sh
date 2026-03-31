#!/usr/bin/env bash
# daemon-install.sh — Copy release binary to ~/.convergio/bin/
# WHY: Isolates the running daemon from cargo check/build locks on
#      daemon/target/. Copilot workers and hooks can build freely
#      without crashing the live daemon process.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SRC="$REPO_ROOT/daemon/target/release/convergio-platform-daemon"
DEST_DIR="$HOME/.convergio/bin"
DEST="$DEST_DIR/convergio-platform-daemon"

if [[ ! -f "$SRC" ]]; then
  echo "ERROR: release binary not found at $SRC" >&2
  echo "Run: cd daemon && cargo build --release --features kernel" >&2
  exit 1
fi

mkdir -p "$DEST_DIR"
cp -f "$SRC" "$DEST"
chmod +x "$DEST"

SIZE=$(du -h "$DEST" | cut -f1)
VERSION=$("$DEST" --version 2>/dev/null || echo "unknown")

echo "Installed: $DEST"
echo "Version:   $VERSION"
echo "Size:      $SIZE"
