#!/usr/bin/env bash
# MCP coverage report: find daemon API routes not covered by MCP server.
# Run after adding/changing daemon endpoints.
set -euo pipefail

ROUTES="daemon/src/server/routes/api_routes.rs"
MCP="scripts/platform/mcp-ipc/server.mjs"

echo "=== Convergio MCP Coverage ==="
echo ""

# Extract API paths from Rust route constants
api_routes=$(grep -o '"/api/[^"]*"' "$ROUTES" | tr -d '"' | sort -u)

# Extract API paths from MCP handler calls
mcp_paths=$(grep -oE "'/api/[^']*'|/api/[^'\")\`]+" "$MCP" | tr -d "'" | sort -u)

covered=0
missing=0

echo "Not in MCP server:"
while IFS= read -r route; do
  # Normalize: remove :param segments for matching
  base=$(echo "$route" | sed 's/:[a-z_]*//g; s|//|/|g; s|/$||')
  if echo "$mcp_paths" | grep -qF "$base"; then
    covered=$((covered + 1))
  else
    echo "  $route"
    missing=$((missing + 1))
  fi
done <<< "$api_routes"

total=$((covered + missing))
echo ""
echo "Coverage: $covered/$total (${missing} missing)"
