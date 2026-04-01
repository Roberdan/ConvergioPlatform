#!/usr/bin/env bash
set -euo pipefail

# Update Homebrew formula with real SHA256 checksums from GitHub Releases.
# Usage: ./update-homebrew.sh 20.4.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORMULA="${SCRIPT_DIR}/convergio.rb"
REPO="Roberdan/ConvergioPlatform"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  echo "Example: $0 20.4.0" >&2
  exit 1
fi

VERSION="$1"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
ARM64_FILE="convergio-v${VERSION}-aarch64-apple-darwin.tar.gz"
X86_64_FILE="convergio-v${VERSION}-x86_64-apple-darwin.tar.gz"

TMPDIR_WORK="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_WORK}"' EXIT

echo "Downloading ARM64 tarball..."
curl -fSL "${BASE_URL}/${ARM64_FILE}" -o "${TMPDIR_WORK}/${ARM64_FILE}"

echo "Downloading x86_64 tarball..."
curl -fSL "${BASE_URL}/${X86_64_FILE}" -o "${TMPDIR_WORK}/${X86_64_FILE}"

ARM64_SHA="$(shasum -a 256 "${TMPDIR_WORK}/${ARM64_FILE}" | awk '{print $1}')"
X86_64_SHA="$(shasum -a 256 "${TMPDIR_WORK}/${X86_64_FILE}" | awk '{print $1}')"

echo "ARM64  SHA256: ${ARM64_SHA}"
echo "x86_64 SHA256: ${X86_64_SHA}"

if [[ ! -f "${FORMULA}" ]]; then
  echo "Error: formula not found at ${FORMULA}" >&2
  exit 1
fi

# Update version
sed -i '' "s|version \"[^\"]*\"|version \"${VERSION}\"|" "${FORMULA}"

# Update ARM64 URL and SHA256
sed -i '' "s|/download/v[^/]*/convergio-v[^\"]*-aarch64-apple-darwin.tar.gz|/download/v${VERSION}/convergio-v${VERSION}-aarch64-apple-darwin.tar.gz|" "${FORMULA}"
sed -i '' "/aarch64-apple-darwin/{ n; s|sha256 \"[^\"]*\"|sha256 \"${ARM64_SHA}\"|; }" "${FORMULA}"

# Update x86_64 URL and SHA256
sed -i '' "s|/download/v[^/]*/convergio-v[^\"]*-x86_64-apple-darwin.tar.gz|/download/v${VERSION}/convergio-v${VERSION}-x86_64-apple-darwin.tar.gz|" "${FORMULA}"
sed -i '' "/x86_64-apple-darwin/{ n; s|sha256 \"[^\"]*\"|sha256 \"${X86_64_SHA}\"|; }" "${FORMULA}"

echo ""
echo "Formula updated: ${FORMULA}"
echo "---"
cat "${FORMULA}"
