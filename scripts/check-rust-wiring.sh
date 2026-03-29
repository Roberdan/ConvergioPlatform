#!/usr/bin/env bash
set -euo pipefail

# Check that every .rs file in daemon/src/ directories is wired in its
# parent mod.rs, lib.rs, or main.rs. Handles feature-gated modules,
# #[path] refs, nested dirs, and test files.
#
# Override scan root: DAEMON_SRC=/path/to/src ./check-rust-wiring.sh
# Exit 0 = clean, Exit 1 = unwired files found.

ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo '.')
SRC="${DAEMON_SRC:-$ROOT/daemon/src}"

if [ ! -d "$SRC" ]; then
  echo "WARN: $SRC not found, skipping wiring check" >&2
  exit 0
fi

UNWIRED=()

# Extract mod names from one or more parent files.
collect_declared_mods() {
  for pf in "$@"; do
    sed -E -n 's/.*mod[[:space:]]+([a-zA-Z_][a-zA-Z0-9_]*).*/\1/p' \
      "$pf" 2>/dev/null
  done | sort -u
}

# Collect #[path = "..."] referenced basenames from .rs files in a dir
# and from subdirectory files using ../ to reference back.
collect_path_refs() {
  local dir="$1"
  {
    local -a files=()
    for f in "$dir"/*.rs; do
      [ -f "$f" ] && files+=("$f")
    done
    if [ ${#files[@]} -gt 0 ]; then
      { grep -h '#\[path.*=.*"[^"]*"' "${files[@]}" 2>/dev/null || true; } \
        | sed -E -n 's/.*#\[path[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' \
        | while read -r p; do basename "$p"; done
    fi
    { find "$dir" -mindepth 2 -name '*.rs' -print0 2>/dev/null \
      | xargs -0 grep -h '#\[path.*=.*"\.\./[^"]*"' 2>/dev/null || true; } \
      | sed -E -n 's/.*#\[path[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' \
      | while read -r p; do
          case "$p" in ../*) basename "$p" ;; esac
        done
  } | sort -u
}

# Check one directory, merging declarations from all parent files
check_dir() {
  local dir="$1"
  shift
  local -a parent_files=("$@")

  local -a declared=()
  while IFS= read -r m; do
    [ -n "$m" ] && declared+=("$m")
  done < <(collect_declared_mods "${parent_files[@]}")

  local -a path_refs=()
  while IFS= read -r p; do
    [ -n "$p" ] && path_refs+=("$p")
  done < <(collect_path_refs "$dir")

  for rs_file in "$dir"/*.rs; do
    [ -f "$rs_file" ] || continue
    local base
    base=$(basename "$rs_file")

    case "$base" in mod.rs|lib.rs|main.rs) continue ;; esac
    case "$base" in *_tests.rs|*_test.rs|tests.rs) continue ;; esac

    local stem="${base%.rs}"
    local found=false

    for d in "${declared[@]+"${declared[@]}"}"; do
      [ "$d" = "$stem" ] && { found=true; break; }
    done
    $found && continue

    for p in "${path_refs[@]+"${path_refs[@]}"}"; do
      [ "$p" = "$base" ] && { found=true; break; }
    done
    $found && continue

    UNWIRED+=("$rs_file")
  done
}

# Find unique directories, collect their parent files, check each once
CHECKED_DIRS=""
while IFS= read -r parent_file; do
  dir=$(dirname "$parent_file")
  # Skip if already checked
  case ",$CHECKED_DIRS," in *",$dir,"*) continue ;; esac
  CHECKED_DIRS="${CHECKED_DIRS:+$CHECKED_DIRS,}$dir"

  # Collect all parent files in this directory
  local_parents=()
  for candidate in "$dir/mod.rs" "$dir/lib.rs" "$dir/main.rs"; do
    [ -f "$candidate" ] && local_parents+=("$candidate")
  done
  [ ${#local_parents[@]} -eq 0 ] && continue

  check_dir "$dir" "${local_parents[@]}"
done < <(find "$SRC" \( -name "mod.rs" -o -name "lib.rs" -o -name "main.rs" \) | sort)

if [ ${#UNWIRED[@]} -gt 0 ]; then
  echo "ERROR: ${#UNWIRED[@]} unwired .rs file(s) found:" >&2
  for f in "${UNWIRED[@]}"; do
    echo "  $f" >&2
  done
  echo "Add 'mod <name>;' or 'pub mod <name>;' to the parent mod.rs/lib.rs" >&2
  exit 1
fi

exit 0
