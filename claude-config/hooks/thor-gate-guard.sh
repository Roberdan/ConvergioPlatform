#!/usr/bin/env bash
# Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
# thor-gate-guard.sh — Enforce correct plan execution workflow.
#
# Constitution Article VI, ADR-0001:
#   ONLY Thor can set task status=done.
#   Flow per wave:
#     1. All tasks in wave: pending → in_progress → submitted
#     2. After ALL wave tasks submitted: cvg plan validate <plan_id>
#        Thor batch-validates the entire wave → submitted tasks become done
#     3. NEVER call validate per-task. NEVER set done directly.
#
# Triggered by: PreToolUse Bash hook
# Reads: stdin (JSON with command field from Claude Code)
#
# Why: Plan 10044 — 46 tasks executed but plan couldn't close because
#      executor set done directly, bypassing Thor wave-level validation.

set -euo pipefail

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.input.command // .command // empty' 2>/dev/null)
[ -z "$CMD" ] && exit 0

# ── Guard 1: Block "cvg task update <id> done" ──
if echo "$CMD" | grep -qE 'cvg\s+task\s+update\s+\S+\s+done'; then
  echo "BLOCKED: ThorGateGuard — cannot set task status=done directly." >&2
  echo "  Constitution Art. VI / ADR-0001: ONLY Thor can promote tasks to done." >&2
  echo "  Correct flow:" >&2
  echo "    1. Complete all tasks in the wave → cvg task update <id> submitted" >&2
  echo "    2. After ALL wave tasks are submitted:" >&2
  echo "       cvg plan validate <plan_id>  ← Thor validates the entire wave" >&2
  exit 2
fi

# ── Guard 2: Block per-task validate (must be per-wave via plan validate) ──
if echo "$CMD" | grep -qE 'cvg\s+task\s+validate'; then
  echo "BLOCKED: ThorGateGuard — do not validate individual tasks." >&2
  echo "  Thor validates at wave level, not per-task." >&2
  echo "  Use: cvg plan validate <plan_id>  ← after ALL wave tasks are submitted." >&2
  exit 2
fi

# ── Guard 3: Block forced-admin validate endpoint ──
if echo "$CMD" | grep -qE '/api/plans/[0-9]+/validate'; then
  echo "BLOCKED: ThorGateGuard — cannot use forced-admin validate endpoint." >&2
  echo "  Use: cvg plan validate <plan_id>  ← proper Thor wave validation." >&2
  exit 2
fi

# ── Guard 4: Block direct SQL/API status=done ──
if echo "$CMD" | grep -qiE "UPDATE.*tasks.*SET.*status.*=.*['\"]done['\"]"; then
  echo "BLOCKED: ThorGateGuard — cannot UPDATE task status to done via SQL." >&2
  echo "  Use: cvg plan validate <plan_id>  ← Thor decides at wave level." >&2
  exit 2
fi

if echo "$CMD" | grep -qE 'curl.*status.*done.*task|curl.*task.*status.*done'; then
  echo "BLOCKED: ThorGateGuard — cannot set task status=done via API." >&2
  echo "  Use: cvg plan validate <plan_id>  ← Thor decides at wave level." >&2
  exit 2
fi

exit 0
