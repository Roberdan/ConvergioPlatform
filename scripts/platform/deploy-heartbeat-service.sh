#!/usr/bin/env bash
set -euo pipefail
# DEPRECATED: mesh-heartbeat shell service has been consolidated into the Rust daemon.

cat <<'MSG'
[deprecated] deploy-heartbeat-service.sh
Heartbeat is now handled natively by claude-core daemon.

Use one of these daemon-based paths instead:
- scripts/mesh-provision-node.sh           (provisions claude-mesh-daemon service)
- config/com.claude.mesh-heartbeat.plist   (compat plist now starts claude-core daemon)

No action performed by this deprecated script.
MSG
