#!/usr/bin/env bash
# setup-models.test.sh — Test suite for scripts/kernel/setup-models.sh
# Run: bash scripts/kernel/setup-models.test.sh
# Verifies: syntax, model declarations, Voxtral TTS required, mlx-audio prereq
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETUP_SCRIPT="$SCRIPT_DIR/setup-models.sh"
PASS=0
FAIL=0

pass() { echo "  [PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== setup-models.sh test suite ==="
echo ""

# T1: Script exists
if [[ -f "$SETUP_SCRIPT" ]]; then
  pass "T1: scripts/kernel/setup-models.sh exists"
else
  fail "T1: scripts/kernel/setup-models.sh does not exist"
fi

# T2: Syntax check (bash -n)
if bash -n "$SETUP_SCRIPT" 2>/dev/null; then
  pass "T2: bash -n syntax check passes"
else
  fail "T2: bash -n syntax check FAILED"
fi

# T3: Voxtral-Mini bf16 is in REQUIRED_MODELS (primary TTS backend)
if grep -q 'Voxtral-Mini-3B-2507-bf16' "$SETUP_SCRIPT"; then
  pass "T3: Voxtral-Mini bf16 model referenced"
else
  fail "T3: Voxtral-Mini bf16 model NOT found — required for primary TTS"
fi

# T4: Voxtral bf16 is REQUIRED, not optional
# Extract REQUIRED_MODELS block (from declaration to closing paren)
REQ_SECTION=$(sed -n '/declare -A REQUIRED_MODELS/,/^)/p' "$SETUP_SCRIPT")
if echo "$REQ_SECTION" | grep -q 'Voxtral-Mini-3B-2507-bf16'; then
  pass "T4: Voxtral-Mini bf16 is in REQUIRED_MODELS section"
else
  fail "T4: Voxtral-Mini bf16 is NOT in REQUIRED_MODELS section"
fi

# T5: Qwen3-TTS model is declared (secondary TTS backend)
if grep -q 'Qwen3-TTS' "$SETUP_SCRIPT"; then
  pass "T5: Qwen3-TTS model referenced"
else
  fail "T5: Qwen3-TTS model NOT found — needed as secondary TTS backend"
fi

# T6: mlx-audio pip package check is present (TTS dependency)
if grep -q 'mlx.audio\|mlx_audio\|mlx-audio' "$SETUP_SCRIPT"; then
  pass "T6: mlx-audio dependency check present"
else
  fail "T6: mlx-audio dependency check NOT found — required for neural TTS"
fi

# T7: Old 4bit Voxtral should NOT be in required or optional
SCRIPT_CONTENT=$(cat "$SETUP_SCRIPT")
if echo "$SCRIPT_CONTENT" | grep -q 'Voxtral-Mini-3B-2507-4bit'; then
  fail "T7: Old Voxtral 4bit variant still present — should be replaced by bf16"
else
  pass "T7: Old Voxtral 4bit variant removed"
fi

# T8: Line count does not exceed 250
LINE_COUNT=$(wc -l < "$SETUP_SCRIPT")
if [[ "$LINE_COUNT" -le 250 ]]; then
  pass "T8: line count ${LINE_COUNT} <= 250"
else
  fail "T8: line count ${LINE_COUNT} EXCEEDS 250"
fi

# T9: Model repo matches tts.rs exactly (mlx-community/Voxtral-Mini-3B-2507-bf16)
if grep -q 'mlx-community/Voxtral-Mini-3B-2507-bf16' "$SETUP_SCRIPT"; then
  pass "T9: Voxtral repo matches tts.rs (mlx-community/Voxtral-Mini-3B-2507-bf16)"
else
  fail "T9: Voxtral repo does NOT match tts.rs"
fi

# T10: Qwen3-TTS repo matches tts.rs exactly
if grep -q 'mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16' "$SETUP_SCRIPT"; then
  pass "T10: Qwen3-TTS repo matches tts.rs"
else
  fail "T10: Qwen3-TTS repo does NOT match tts.rs"
fi

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
[[ $FAIL -eq 0 ]]
