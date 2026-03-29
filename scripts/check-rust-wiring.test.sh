#!/usr/bin/env bash
set -euo pipefail

# Tests for check-rust-wiring.sh
# Creates a temporary Rust project structure and validates the checker

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CHECKER="$SCRIPT_DIR/check-rust-wiring.sh"
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

PASS=0
FAIL=0

assert_exit() {
  local desc="$1" expected="$2" dir="$3"
  if DAEMON_SRC="$dir" "$CHECKER" >/dev/null 2>&1; then actual=0; else actual=$?; fi
  if [ "$actual" -eq "$expected" ]; then
    PASS=$((PASS + 1))
    echo "  PASS: $desc"
  else
    FAIL=$((FAIL + 1))
    echo "  FAIL: $desc (expected exit $expected, got $actual)"
    DAEMON_SRC="$dir" "$CHECKER" 2>&1 || true
  fi
}

# --- Test 1: All wired → exit 0 ---
T1="$TMPDIR_BASE/t1"
mkdir -p "$T1/mymod"
cat > "$T1/mymod/mod.rs" <<'RS'
pub mod alpha;
mod beta;
RS
touch "$T1/mymod/alpha.rs" "$T1/mymod/beta.rs"
assert_exit "all files wired" 0 "$T1"

# --- Test 2: Unwired file → exit 1 ---
T2="$TMPDIR_BASE/t2"
mkdir -p "$T2/mymod"
cat > "$T2/mymod/mod.rs" <<'RS'
pub mod alpha;
RS
touch "$T2/mymod/alpha.rs" "$T2/mymod/orphan.rs"
assert_exit "unwired file detected" 1 "$T2"

# --- Test 3: Feature-gated module → exit 0 ---
T3="$TMPDIR_BASE/t3"
mkdir -p "$T3/mymod"
cat > "$T3/mymod/mod.rs" <<'RS'
pub mod alpha;
#[cfg(feature = "voice")]
pub mod voice;
RS
touch "$T3/mymod/alpha.rs" "$T3/mymod/voice.rs"
assert_exit "feature-gated module accepted" 0 "$T3"

# --- Test 4: #[path = "file.rs"] reference → exit 0 ---
T4="$TMPDIR_BASE/t4"
mkdir -p "$T4/mymod"
cat > "$T4/mymod/mod.rs" <<'RS'
pub mod base;
RS
cat > "$T4/mymod/base.rs" <<'RS'
#[path = "base_impl.rs"]
mod base_impl;
RS
touch "$T4/mymod/base_impl.rs"
assert_exit "path-referenced file accepted" 0 "$T4"

# --- Test 5: Test files (*_tests.rs) excluded → exit 0 ---
T5="$TMPDIR_BASE/t5"
mkdir -p "$T5/mymod"
cat > "$T5/mymod/mod.rs" <<'RS'
pub mod core;
#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;
RS
touch "$T5/mymod/core.rs" "$T5/mymod/core_tests.rs"
assert_exit "test files excluded from check" 0 "$T5"

# --- Test 6: lib.rs works like mod.rs ---
T6="$TMPDIR_BASE/t6"
mkdir -p "$T6"
cat > "$T6/lib.rs" <<'RS'
pub mod utils;
RS
touch "$T6/utils.rs"
assert_exit "lib.rs as parent module" 0 "$T6"

# --- Test 7: Nested directories ---
T7="$TMPDIR_BASE/t7"
mkdir -p "$T7/outer/inner"
cat > "$T7/outer/mod.rs" <<'RS'
pub mod inner;
RS
cat > "$T7/outer/inner/mod.rs" <<'RS'
pub mod deep;
RS
touch "$T7/outer/inner/deep.rs"
assert_exit "nested directories" 0 "$T7"

# --- Test 8: pub(crate) mod → accepted ---
T8="$TMPDIR_BASE/t8"
mkdir -p "$T8/mymod"
cat > "$T8/mymod/mod.rs" <<'RS'
pub(crate) mod secret;
RS
touch "$T8/mymod/secret.rs"
assert_exit "pub(crate) mod accepted" 0 "$T8"

# --- Test 9: #[path] with ../ relative path ---
T9="$TMPDIR_BASE/t9"
mkdir -p "$T9/parent/child"
cat > "$T9/parent/mod.rs" <<'RS'
pub mod child;
pub mod sibling;
RS
cat > "$T9/parent/child/mod.rs" <<'RS'
#[path = "../shared.rs"]
mod shared;
RS
touch "$T9/parent/sibling.rs" "$T9/parent/shared.rs"
assert_exit "relative #[path] reference" 0 "$T9"

# --- Test 10: main.rs as crate root ---
T10="$TMPDIR_BASE/t10"
mkdir -p "$T10"
cat > "$T10/main.rs" <<'RS'
mod cli_commands;
mod daemon_logging;
RS
touch "$T10/cli_commands.rs" "$T10/daemon_logging.rs"
assert_exit "main.rs as crate root" 0 "$T10"

# --- Test 11: main.rs unwired → exit 1 ---
T11="$TMPDIR_BASE/t11"
mkdir -p "$T11"
cat > "$T11/main.rs" <<'RS'
mod cli_commands;
RS
touch "$T11/cli_commands.rs" "$T11/orphan.rs"
assert_exit "main.rs detects unwired" 1 "$T11"

# --- Test 12: Empty directory (no .rs files) → exit 0 ---
T12="$TMPDIR_BASE/t12"
mkdir -p "$T12/empty"
cat > "$T12/empty/mod.rs" <<'RS'
// empty module
RS
assert_exit "empty directory is clean" 0 "$T12"

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
