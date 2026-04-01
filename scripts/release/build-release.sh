#!/usr/bin/env bash
# build-release.sh — Local release build for the native target.
#
# Usage:
#   scripts/release/build-release.sh [TAG]
#
# Builds a release binary with --features kernel, strips it,
# and creates a tarball matching the CI naming convention.

set -euo pipefail
trap 'echo "ERROR: build-release.sh failed at line $LINENO" >&2' EXIT

BINARY_NAME="convergio-platform-daemon"
TAG="${1:-$(git describe --tags --abbrev=0 2>/dev/null || echo "dev")}"

# Detect native target triple
TARGET="$(rustc -vV | awk '/^host:/ { print $2 }')"
if [ -z "$TARGET" ]; then
  echo "ERROR: could not detect Rust host target" >&2
  exit 1
fi

echo "Building release: tag=${TAG} target=${TARGET}"

# Build
cd "$(git rev-parse --show-toplevel)/daemon"
cargo build --release --features kernel --target "$TARGET"

# Strip
BINARY="target/${TARGET}/release/${BINARY_NAME}"
if [ ! -f "$BINARY" ]; then
  echo "ERROR: binary not found at ${BINARY}" >&2
  exit 1
fi
strip "$BINARY"

# Package
ARCHIVE_DIR="$(git rev-parse --show-toplevel)/dist"
mkdir -p "$ARCHIVE_DIR"
ARCHIVE="${ARCHIVE_DIR}/convergio-${TAG}-${TARGET}.tar.gz"
tar -czf "$ARCHIVE" -C "target/${TARGET}/release" "$BINARY_NAME"

SIZE=$(du -h "$ARCHIVE" | cut -f1)
echo "Done: ${ARCHIVE} (${SIZE})"

trap - EXIT
