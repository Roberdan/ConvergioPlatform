#!/bin/bash
# verify-content-quality.sh — Verify content quality of task output
# Checks: minimum LOC, stub detection, template/boilerplate detection
# Usage: verify-content-quality.sh <file_or_dir> [--min-loc N] [--strict]
# Exit 0 = pass, Exit 1 = fail, Exit 2 = warning
# Version: 1.0.0
set -euo pipefail

TARGET="${1:?file or directory required}"
shift

MIN_LOC=5
STRICT=false
while [[ $# -gt 0 ]]; do
	case "$1" in
	--min-loc) MIN_LOC="$2"; shift 2 ;;
	--strict) STRICT=true; shift ;;
	*) shift ;;
	esac
done

ISSUES=()
WARNINGS=()

# Check if target exists
if [[ ! -e "$TARGET" ]]; then
	echo '{"pass":false,"error":"target not found","target":"'"$TARGET"'"}'
	exit 1
fi

# Collect files to check
FILES=()
if [[ -d "$TARGET" ]]; then
	while IFS= read -r f; do
		FILES+=("$f")
	done < <(find "$TARGET" -type f \( -name "*.rs" -o -name "*.sh" -o -name "*.js" -o -name "*.ts" -o -name "*.py" \) | head -50)
else
	FILES=("$TARGET")
fi

if [[ ${#FILES[@]} -eq 0 ]]; then
	echo '{"pass":true,"warning":"no source files found","target":"'"$TARGET"'"}'
	exit 0
fi

for file in "${FILES[@]}"; do
	fname="$(basename "$file")"
	loc=$(wc -l <"$file" | tr -d ' ')

	# Check minimum lines of code
	if [[ $loc -lt $MIN_LOC ]]; then
		ISSUES+=("$fname: only $loc lines (min: $MIN_LOC)")
	fi

	# Stub detection: TODO/FIXME/unimplemented/todo!() without real logic
	stub_count=$(grep -cEi '(TODO|FIXME|unimplemented!|todo!\(\)|pass\b|raise NotImplementedError)' "$file" 2>/dev/null || echo 0)
	if [[ $stub_count -gt 0 ]]; then
		WARNINGS+=("$fname: $stub_count stub markers found")
	fi

	# Template detection: check for placeholder patterns
	template_count=$(grep -cEi '(YOUR_.*_HERE|REPLACE_ME|xxx|placeholder|example\.com|foo_bar_baz)' "$file" 2>/dev/null || echo 0)
	if [[ $template_count -gt 0 ]]; then
		ISSUES+=("$fname: $template_count template placeholders found")
	fi

	# Empty function detection (Rust/JS/TS)
	empty_fn=$(grep -cE '(fn [a-z_]+\([^)]*\)[^{]*\{\s*\}|function [a-z_]+\([^)]*\)\s*\{\s*\}|=>\s*\{\s*\})' "$file" 2>/dev/null || echo 0)
	if [[ $empty_fn -gt 0 ]]; then
		ISSUES+=("$fname: $empty_fn empty function bodies")
	fi

	# Shell: functions with only echo/true/return 0
	if [[ "$fname" == *.sh ]]; then
		noop_fn=$(grep -cE '^\s*(echo|true|return 0|:)\s*$' "$file" 2>/dev/null || echo 0)
		total_lines=$loc
		if [[ $total_lines -gt 0 ]]; then
			noop_ratio=$((noop_fn * 100 / total_lines))
			if [[ $noop_ratio -gt 50 ]]; then
				ISSUES+=("$fname: ${noop_ratio}% no-op lines (likely stub)")
			fi
		fi
	fi
done

# Build result JSON
PASS=true
EXIT_CODE=0
if [[ ${#ISSUES[@]} -gt 0 ]]; then
	PASS=false
	EXIT_CODE=1
elif [[ ${#WARNINGS[@]} -gt 0 && "$STRICT" == true ]]; then
	PASS=false
	EXIT_CODE=2
fi

ISSUES_JSON="[]"
if [[ ${#ISSUES[@]} -gt 0 ]]; then
	ISSUES_JSON=$(printf '%s\n' "${ISSUES[@]}" | jq -R . | jq -sc .)
fi
WARNINGS_JSON="[]"
if [[ ${#WARNINGS[@]} -gt 0 ]]; then
	WARNINGS_JSON=$(printf '%s\n' "${WARNINGS[@]}" | jq -R . | jq -sc .)
fi

echo "{\"pass\":$PASS,\"files\":${#FILES[@]},\"issues\":$ISSUES_JSON,\"warnings\":$WARNINGS_JSON}"
exit $EXIT_CODE
