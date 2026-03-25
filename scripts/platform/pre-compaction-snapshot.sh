#!/bin/bash
# pre-compaction-snapshot.sh — PreCompact hook
# Saves minimal plan state before context compaction
# Why: coordinator loses plan execution state on compaction (feedback_context_bloat_prevention.md)
set -euo pipefail

# Auto-save checkpoint for active plan
if command -v cvg &>/dev/null; then
  cvg checkpoint save-auto 2>/dev/null || true
fi

exit 0
