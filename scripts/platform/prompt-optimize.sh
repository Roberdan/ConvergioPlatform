#!/usr/bin/env bash
# prompt-optimize.sh — Scan agent definitions for token bloat, compare with previous run.
#
# Usage: ./prompt-optimize.sh [--agents-dir <dir>] [--prev-file <file>] [--save]
#   --agents-dir  Path to agent .md files (default: claude-config/agents)
#   --prev-file   JSON file with previous run data (default: prompt-audit-prev.json in repo root)
#   --save        Persist current run to prev-file for next delta comparison
#
# Output: per-agent token count, line count, delta from last run, bloat warnings.
# Exit 0 always; warnings are informational only.

set -euo pipefail

# ── defaults ────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
AGENTS_DIR="${REPO_ROOT}/claude-config/agents"
PREV_FILE="${REPO_ROOT}/prompt-audit-prev.json"
SAVE=false

LINE_LIMIT=200
WORD_LIMIT=3000

# ── arg parsing ──────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --agents-dir) AGENTS_DIR="$2"; shift 2 ;;
    --prev-file)  PREV_FILE="$2";  shift 2 ;;
    --save)       SAVE=true;       shift   ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# ── validate ─────────────────────────────────────────────────────────────────
if [[ ! -d "${AGENTS_DIR}" ]]; then
  echo "Agents directory not found: ${AGENTS_DIR}" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required but not found in PATH" >&2
  exit 1
fi

# ── delegate to Python for full logic (Bash 3.2 compat — no assoc arrays) ────
python3 - "${AGENTS_DIR}" "${PREV_FILE}" "${SAVE}" "${LINE_LIMIT}" "${WORD_LIMIT}" <<'PYEOF'
import sys, os, json, datetime

agents_dir  = sys.argv[1]
prev_file   = sys.argv[2]
do_save     = sys.argv[3] == "true"
line_limit  = int(sys.argv[4])
word_limit  = int(sys.argv[5])

RED    = "\033[0;31m"
YELLOW = "\033[1;33m"
GREEN  = "\033[0;32m"
CYAN   = "\033[0;36m"
BOLD   = "\033[1m"
NC     = "\033[0m"

# ── load previous run ────────────────────────────────────────────────────────
prev_data = {}
if os.path.isfile(prev_file):
    try:
        with open(prev_file) as f:
            prev_data = json.load(f)
    except (json.JSONDecodeError, IOError):
        pass

prev_agents = prev_data.get("agents", {})
prev_total  = prev_data.get("total_words", 0)

# ── scan md files ────────────────────────────────────────────────────────────
md_files = sorted(
    os.path.join(root, fname)
    for root, _, files in os.walk(agents_dir)
    for fname in files
    if fname.endswith(".md")
)

if not md_files:
    print(f"{CYAN}[INFO]{NC} No .md files found in {agents_dir}")
    sys.exit(0)

print()
print("══════════════════════════════════════════════════════════════")
print(f"  Prompt Optimization Report — {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
print("══════════════════════════════════════════════════════════════")
print()
print(f"{'Agent':<45} {'Lines':>6} {'Words':>6}  {'Delta':<14} Status")
print(f"{'-----':<45} {'-----':>6} {'-----':>6}  {'-----':<14} ------")

total_words  = 0
total_lines  = 0
bloat_count  = 0
current_data = {}

for fpath in md_files:
    name = os.path.splitext(os.path.basename(fpath))[0]
    with open(fpath, encoding="utf-8", errors="replace") as f:
        content = f.read()

    lines = content.count("\n")
    words = len(content.split())
    total_words += words
    total_lines += lines
    current_data[name] = {"words": words, "lines": lines}

    # delta
    prev_w = prev_agents.get(name, {}).get("words", 0)
    if prev_w == 0:
        delta_str = "(new)"
        delta_col = NC
    else:
        diff = words - prev_w
        if diff > 0:
            delta_str = f"(+{diff})"
            delta_col = YELLOW
        elif diff < 0:
            delta_str = f"({diff})"
            delta_col = GREEN
        else:
            delta_str = "(no change)"
            delta_col = NC

    # status
    both  = lines > line_limit and words > word_limit
    l_big = lines > line_limit
    w_big = words > word_limit
    if both:
        status = f"{RED}BLOAT (lines+words){NC}"
        bloat_count += 1
    elif l_big:
        status = f"{YELLOW}WARN (>{line_limit} lines){NC}"
        bloat_count += 1
    elif w_big:
        status = f"{YELLOW}WARN (>{word_limit} words){NC}"
        bloat_count += 1
    else:
        status = f"{GREEN}OK{NC}"

    delta_display = f"{delta_col}{delta_str:<14}{NC}"
    print(f"{name:<45} {lines:>6} {words:>6}  {delta_display} {status}")

# ── summary ──────────────────────────────────────────────────────────────────
print()
print("══════════════════════════════════════════════════════════════")

if prev_total > 0:
    td = total_words - prev_total
    sign = "+" if td >= 0 else ""
    delta_summary = f"{sign}{td} from last run"
else:
    delta_summary = "first run"

print(f"  Agents scanned : {len(md_files)}")
print(f"  Total lines    : {total_lines}")
print(f"  Total words    : {total_words}  ({delta_summary})")
print(f"  Bloat warnings : {bloat_count}")
print("══════════════════════════════════════════════════════════════")
print()

if bloat_count > 0:
    print(f"{YELLOW}[WARN]{NC} {bloat_count} agent(s) exceed line/word limits — consider trimming.")
else:
    print(f"{GREEN}[OK]{NC} All agents within size limits.")

# ── persist ──────────────────────────────────────────────────────────────────
if do_save:
    payload = {
        "timestamp": datetime.datetime.utcnow().isoformat() + "Z",
        "total_words": total_words,
        "agents": current_data,
    }
    with open(prev_file, "w") as f:
        json.dump(payload, f, indent=2)
    print(f"  Saved run data to: {prev_file}")

print()
PYEOF
