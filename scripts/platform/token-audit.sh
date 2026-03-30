#!/usr/bin/env bash
# token-audit.sh — Count instruction tokens across all agent definitions.
#
# Usage: ./token-audit.sh [--agents-dir <dir>] [--budget <words>] [--format text|json]
#   --agents-dir  Root of agent .md files (default: claude-config/agents)
#   --budget      Word budget threshold (default: 50000)
#   --format      Output format: text (default) or json
#
# Exit codes: 0 = within budget, 1 = over budget.

set -euo pipefail

# ── defaults ────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
AGENTS_DIR="${REPO_ROOT}/claude-config/agents"
BUDGET=50000
FORMAT="text"

# ── arg parsing ──────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --agents-dir) AGENTS_DIR="$2"; shift 2 ;;
    --budget)     BUDGET="$2";     shift 2 ;;
    --format)     FORMAT="$2";     shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# ── validate inputs ──────────────────────────────────────────────────────────
if [[ ! -d "${AGENTS_DIR}" ]]; then
  echo "Agents directory not found: ${AGENTS_DIR}" >&2
  exit 1
fi

if ! [[ "${BUDGET}" =~ ^[0-9]+$ ]]; then
  echo "Budget must be a positive integer, got: ${BUDGET}" >&2
  exit 1
fi

if [[ "${FORMAT}" != "text" && "${FORMAT}" != "json" ]]; then
  echo "Format must be 'text' or 'json', got: ${FORMAT}" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required but not found in PATH" >&2
  exit 1
fi

# ── delegate to Python (Bash 3.2 compat — no assoc arrays) ──────────────────
python3 - "${AGENTS_DIR}" "${BUDGET}" "${FORMAT}" <<'PYEOF'
import sys, os, json, datetime

agents_dir = sys.argv[1]
budget     = int(sys.argv[2])
fmt        = sys.argv[3]

RED    = "\033[0;31m"
YELLOW = "\033[1;33m"
GREEN  = "\033[0;32m"
BOLD   = "\033[1m"
NC     = "\033[0m"

# ── collect per-category data ────────────────────────────────────────────────
categories  = {}  # category -> {words, files}
total_words = 0
total_files = 0

for root, dirs, files in os.walk(agents_dir):
    dirs.sort()
    for fname in sorted(files):
        if not fname.endswith(".md"):
            continue
        fpath = os.path.join(root, fname)
        rel   = os.path.relpath(fpath, agents_dir)
        parts = rel.split(os.sep)
        # Category = immediate subdirectory; files at root level -> "root"
        category = parts[0] if len(parts) > 1 else "root"

        with open(fpath, encoding="utf-8", errors="replace") as f:
            words = len(f.read().split())

        total_words += words
        total_files += 1

        if category not in categories:
            categories[category] = {"words": 0, "files": 0}
        categories[category]["words"] += words
        categories[category]["files"] += 1

over_budget = total_words > budget
usage_pct   = round(total_words / budget * 100, 1)

# ── output ────────────────────────────────────────────────────────────────────
if fmt == "json":
    payload = {
        "timestamp": datetime.datetime.utcnow().isoformat() + "Z",
        "budget": budget,
        "total_words": total_words,
        "total_files": total_files,
        "over_budget": over_budget,
        "usage_pct": usage_pct,
        "categories": categories,
    }
    print(json.dumps(payload, indent=2))
else:
    print()
    print(f"{BOLD}══════════════════════════════════════════════════════════════{NC}")
    print(f"{BOLD}  Token Audit Report — {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}{NC}")
    print(f"{BOLD}══════════════════════════════════════════════════════════════{NC}")
    print()
    print(f"{'Category':<35} {'Words':>8} {'Files':>6} {'% Budget':>8}")
    print(f"{'--------':<35} {'-----':>8} {'-----':>6} {'--------':>8}")

    # Sort by word count descending; highlight if > 20% of budget individually
    threshold_20 = budget // 5
    for cat, data in sorted(categories.items(), key=lambda x: -x[1]["words"]):
        w = data["words"]
        f = data["files"]
        p = round(w / budget * 100, 1)
        row = f"{cat:<35} {w:>8} {f:>6} {p:>7}%"
        if w > threshold_20:
            print(f"{YELLOW}{row}{NC}")
        else:
            print(row)

    print()
    print(f"{BOLD}──────────────────────────────────────────────────────────────{NC}")

    if over_budget:
        overage = total_words - budget
        print(f"{RED}{BOLD}  OVER BUDGET{NC}")
        print(f"  Total words  : {RED}{total_words}{NC} / {budget} ({usage_pct}% — {overage} over)")
    else:
        remaining = budget - total_words
        print(f"{GREEN}  WITHIN BUDGET{NC}")
        print(f"  Total words  : {GREEN}{total_words}{NC} / {budget} ({usage_pct}% — {remaining} remaining)")

    print(f"  Total files  : {total_files}")
    print(f"  Budget       : {budget} words")
    print(f"{BOLD}══════════════════════════════════════════════════════════════{NC}")
    print()

    if over_budget:
        print(f"{RED}[ALERT]{NC} Token budget exceeded. Review large categories above.")

sys.exit(1 if over_budget else 0)
PYEOF
