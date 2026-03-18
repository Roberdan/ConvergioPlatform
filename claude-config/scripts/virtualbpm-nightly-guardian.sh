#!/usr/bin/env bash
set -euo pipefail
TRIGGER_SOURCE="scheduled"
PARENT_RUN_ID=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --trigger=*) TRIGGER_SOURCE="${1#*=}" ;;
    --trigger) TRIGGER_SOURCE="${2:-$TRIGGER_SOURCE}"; shift ;;
    --parent-run-id=*) PARENT_RUN_ID="${1#*=}" ;;
    --parent-run-id) PARENT_RUN_ID="${2:-$PARENT_RUN_ID}"; shift ;;
    *) ;;
  esac
  shift
done
# VirtualBPM nightly guardian: triage GitHub issues and run safe auto-remediation.
# No Sentry — VirtualBPM uses Azure Container Apps + GitHub Actions CI.
# Version: 1.0.0
CLAUDE_HOME="${CLAUDE_HOME:-$HOME/.claude}"

# Load local infra config (provides GH_MICROSOFT_ACCOUNT, GH_DEFAULT_ACCOUNT, etc.)
_VG_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../../config/load-config.sh
source "$_VG_SCRIPT_DIR/../../config/load-config.sh" 2>/dev/null || true
unset _VG_SCRIPT_DIR

CONFIG_FILE="${VIRTUALBPM_NIGHTLY_CONFIG:-$CLAUDE_HOME/config/virtualbpm-nightly.conf}"
[[ -f "$CONFIG_FILE" ]] && source "$CONFIG_FILE"

# ── gh auth switch (VirtualBPM uses a dedicated GitHub account) ──
[[ -z "${GH_MICROSOFT_ACCOUNT:-}" ]] && { echo "ERROR: GH_MICROSOFT_ACCOUNT not set. Run: cp config/local.env.example config/local.env" >&2; exit 1; }
GH_USER="${VIRTUALBPM_GH_USER:-${GH_MICROSOFT_ACCOUNT}}"
GH_RESTORE_USER="${VIRTUALBPM_GH_RESTORE_USER:-${GH_DEFAULT_ACCOUNT:-Roberdan}}"
GH_SWITCHED=false

switch_gh_user() {
  local target_user="$1"
  local current_user
  current_user="$(gh auth status 2>&1 | grep -oP 'Logged in to github.com account \K[^ ]+' || gh api user --jq .login 2>/dev/null || echo "")"
  if [[ "$current_user" == "$target_user" ]]; then
    log "gh already authenticated as $target_user"
    return 0
  fi
  log "Switching gh auth: $current_user → $target_user"
  if gh auth switch --user "$target_user" 2>/dev/null; then
    log "gh auth switched to $target_user"
    return 0
  else
    log "WARNING: gh auth switch to $target_user failed"
    return 1
  fi
}

restore_gh_user() {
  if [[ "$GH_SWITCHED" == "true" && -n "$GH_RESTORE_USER" ]]; then
    log "Restoring gh auth to $GH_RESTORE_USER"
    gh auth switch --user "$GH_RESTORE_USER" 2>/dev/null || log "WARNING: failed to restore gh to $GH_RESTORE_USER"
  fi
}

# ── CRR-aware SQLite wrapper (dashboard.db uses crsqlite triggers) ──
_CRSQLITE_EXT="${CLAUDE_HOME}/lib/crsqlite/crsqlite"
_find_capable_sqlite3() {
  for p in /opt/homebrew/opt/sqlite/bin/sqlite3 /usr/local/opt/sqlite/bin/sqlite3; do
    [[ -x "$p" ]] && echo "$p" && return
  done
  echo "$(command -v sqlite3 2>/dev/null || echo sqlite3)"
}
_REAL_SQLITE3="$(_find_capable_sqlite3)"
sqlite3() {
  if [[ -f "$_CRSQLITE_EXT.dylib" || -f "$_CRSQLITE_EXT.so" || -f "$_CRSQLITE_EXT" ]]; then
    "$_REAL_SQLITE3" -cmd ".load $_CRSQLITE_EXT" "$@" 2>/dev/null
  else
    "$_REAL_SQLITE3" "$@"
  fi
}

CONFIG_SNAPSHOT=$({ env | grep "^VIRTUALBPM_" || true; } | sort | jq -Rs 'split("\n") | map(select(length>0)) | map(split("=") | {(.[0]): (.[1:] | join("="))}) | add // {}')
REPO_PATH="${VIRTUALBPM_REPO_PATH:-$HOME/GitHub/VirtualBPM}"
DEFAULT_BRANCH="${VIRTUALBPM_DEFAULT_BRANCH:-maranelloVanilla}"
REPO_SLUG="${VIRTUALBPM_GITHUB_REPO:-}"
MODEL="${VIRTUALBPM_MODEL:-gpt-5.3-codex}"
MAX_ITEMS="${VIRTUALBPM_MAX_ITEMS:-6}"
PROJECT_AGENT_REL_PATH="${VIRTUALBPM_PROJECT_AGENT_REL_PATH:-.github/agents/night-maintenance.agent.md}"
RUN_FIXES="${VIRTUALBPM_RUN_FIXES:-true}"
AUTO_MERGE="${VIRTUALBPM_AUTO_MERGE:-false}"
FIX_TIMEOUT_SEC="${VIRTUALBPM_FIX_TIMEOUT_SEC:-5400}"

DB_FILE="${CLAUDE_DB:-$CLAUDE_HOME/data/dashboard.db}"
REPORT_DIR="$CLAUDE_HOME/data/nightly-jobs"
RUN_ID="virtualbpm-nightly-$(date -u +%Y%m%d-%H%M%S)"
STARTED_EPOCH=$(date +%s)

log() { printf '[virtualbpm-nightly] %s\n' "$*"; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || { log "Missing command: $1"; exit 1; }; }
sql_escape() { printf "%s" "$1" | sed "s/'/''/g"; }
json_or_default() {
  local default_json="$1"; shift
  local raw
  raw="$("$@" 2>/dev/null || true)"
  if [[ -n "$raw" ]] && jq -e . >/dev/null 2>&1 <<<"$raw"; then printf '%s' "$raw"; else printf '%s' "$default_json"; fi
}

insert_dashboard_notification() {
  local notif_type="$1" severity="$2" title="$3" message="$4" link="${5:-}" has_extended_schema
  has_extended_schema="$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM pragma_table_info('notifications') WHERE name IN ('severity','link','link_type','source_table','source_id');" 2>/dev/null || echo 0)"
  if [[ "$has_extended_schema" -eq 5 ]]; then
    sqlite3 "$DB_FILE" <<SQL >/dev/null 2>&1 || { log "WARNING: failed to persist dashboard notification"; return 0; }
INSERT INTO notifications (
  project_id, type, severity, title, message, link, link_type, source_table, source_id, is_read
)
SELECT
  'virtualbpm',
  '$(sql_escape "$notif_type")',
  '$(sql_escape "$severity")',
  '$(sql_escape "$title")',
  '$(sql_escape "$message")',
  '$(sql_escape "$link")',
  'url',
  'nightly_jobs',
  '$(sql_escape "$RUN_ID")',
  0
WHERE NOT EXISTS (
  SELECT 1 FROM notifications
  WHERE source_table='nightly_jobs' AND source_id='$(sql_escape "$RUN_ID")'
);
SQL
    return 0
  fi
  sqlite3 "$DB_FILE" "INSERT INTO notifications (project_id, type, title, message, is_read) VALUES ('virtualbpm','$(sql_escape "$notif_type")','$(sql_escape "$title")','$(sql_escape "$message")',0);" >/dev/null 2>&1 || log "WARNING: failed to persist dashboard notification"
}

build_report_json() {
  local exit_code="${1:-0}" error_detail="${2:-}"
  jq -n --arg run_id "$RUN_ID" --arg host "$HOST_NAME" --arg status "$STATUS" \
    --arg summary "$SUMMARY" --arg branch "$BRANCH_NAME" --arg pr_url "$PR_URL" \
    --arg trigger "$TRIGGER_SOURCE" --arg parent_run_id "$PARENT_RUN_ID" \
    --arg error_detail "$error_detail" --argjson exit_code "$exit_code" \
    --argjson github_open_issues "$GH_OPEN_COUNT" \
    --argjson actionable_github "$GH_ACTIONABLE_COUNT" \
    --argjson processed_items "$PROCESSED_ITEMS" \
    --argjson fixed_items "${FIXED_ITEMS:-0}" \
    --argjson top_github_issues "$TOP_GITHUB_ISSUES" \
    --argjson deploy "$DEPLOY_JSON" \
    '{run_id:$run_id,host:$host,status:$status,summary:$summary,branch:$branch,pr_url:$pr_url,trigger:$trigger,parent_run_id:$parent_run_id,exit_code:$exit_code,error_detail:$error_detail,github_open_issues:$github_open_issues,actionable_github:$actionable_github,processed_items:$processed_items,fixed_items:$fixed_items,top_github_issues:$top_github_issues,deploy:$deploy}'
}

write_report_files() {
  REPORT_PATH="$REPORT_DIR/${RUN_ID}.json"
  printf '%s\n' "$REPORT_JSON" > "$REPORT_PATH"
  printf '%s\n' "$REPORT_JSON" > "$REPORT_DIR/latest-virtualbpm-nightly.json"
}

ensure_nightly_jobs_column() {
  local column_def="$1" column_name="${1%% *}" present
  present="$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM pragma_table_info('nightly_jobs') WHERE name='$(sql_escape "$column_name")';" 2>/dev/null || echo 0)"
  [[ "$present" -eq 1 ]] || sqlite3 "$DB_FILE" "ALTER TABLE nightly_jobs ADD COLUMN $column_def;" >/dev/null 2>&1
}

# ── Prerequisites ──
require_cmd jq; require_cmd sqlite3; require_cmd git; require_cmd gh
[[ "$RUN_FIXES" == "true" ]] && require_cmd copilot
[[ -d "$REPO_PATH/.git" ]] || { log "Repository not found at $REPO_PATH"; exit 1; }

# ── Enabled-flag check (soft pause from dashboard) ──
if [[ -f "$DB_FILE" ]]; then
  _enabled="$(sqlite3 "$DB_FILE" "SELECT enabled FROM nightly_job_definitions WHERE project_id='virtualbpm' LIMIT 1;" 2>/dev/null || echo 1)"
  if [[ "$_enabled" == "0" ]]; then
    log "Guardian paused via dashboard (enabled=0). Exiting gracefully."
    exit 0
  fi
fi

mkdir -p "$REPORT_DIR" "$(dirname "$DB_FILE")"
LOG_DIR="$CLAUDE_HOME/data/nightly-jobs/logs"; mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/${RUN_ID}.log"
exec > >(tee "$LOG_FILE") 2>&1

# ── Switch gh to configured GH_USER ──
switch_gh_user "$GH_USER" && GH_SWITCHED=true

log "=== Startup Validation ==="
log "Host: $(hostname -f 2>/dev/null || hostname)"
log "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
log "Copilot: $(copilot --version 2>/dev/null || echo not-found)"
log "gh auth: $(gh auth status 2>&1 | head -1 || echo not-found)"
log "Python: $(python3.11 --version 2>/dev/null || python3 --version 2>/dev/null || echo not-found)"
log "Disk: $(df -h "$REPO_PATH" 2>/dev/null | tail -1)"
log "Config: $CONFIG_FILE"
log "=== End Validation ==="

if [[ -z "$REPO_SLUG" ]]; then
  ORIGIN_URL="$(git -C "$REPO_PATH" config --get remote.origin.url || true)"
  REPO_SLUG="$(printf '%s' "$ORIGIN_URL" | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')"
fi
[[ -n "$REPO_SLUG" ]] || { log "Cannot determine GitHub repo slug"; exit 1; }

PROJECT_AGENT_FILE="${REPO_PATH}/${PROJECT_AGENT_REL_PATH}"
PROJECT_AGENT_CONTENT=""; [[ -f "$PROJECT_AGENT_FILE" ]] && PROJECT_AGENT_CONTENT="$(<"$PROJECT_AGENT_FILE")"

# ── DB schema ──
sqlite3 "$DB_FILE" <<'SQL'
CREATE TABLE IF NOT EXISTS nightly_jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT, started_at DATETIME DEFAULT CURRENT_TIMESTAMP, finished_at DATETIME, host TEXT,
  status TEXT NOT NULL CHECK(status IN ('running','ok','action_required','failed')),
  sentry_unresolved INTEGER DEFAULT 0, github_open_issues INTEGER DEFAULT 0, processed_items INTEGER DEFAULT 0,
  fixed_items INTEGER DEFAULT 0, branch_name TEXT, pr_url TEXT, summary TEXT, report_json TEXT,
  log_stdout TEXT, log_file_path TEXT, duration_sec INTEGER DEFAULT 0, config_snapshot TEXT, exit_code INTEGER DEFAULT 0,
  error_detail TEXT, trigger_source TEXT, parent_run_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_nightly_jobs_started ON nightly_jobs(started_at DESC);
CREATE TABLE IF NOT EXISTS notifications (
  id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL DEFAULT 'virtualbpm', type TEXT NOT NULL,
  severity TEXT DEFAULT 'info', title TEXT NOT NULL, message TEXT, link TEXT, link_type TEXT, is_read INTEGER DEFAULT 0,
  is_dismissed INTEGER DEFAULT 0, source_table TEXT, source_id TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, read_at DATETIME
);
CREATE INDEX IF NOT EXISTS idx_notifications_unread ON notifications(is_read, created_at DESC);
SQL

for column_def in "log_stdout TEXT" "log_file_path TEXT" "duration_sec INTEGER DEFAULT 0" "config_snapshot TEXT" "exit_code INTEGER DEFAULT 0" "error_detail TEXT" "trigger_source TEXT" "parent_run_id TEXT"; do
  ensure_nightly_jobs_column "$column_def"
done

HOST_NAME="$(hostname -s 2>/dev/null || echo unknown)"
RUN_ROW_ID="$(sqlite3 "$DB_FILE" "INSERT INTO nightly_jobs(run_id,host,status,trigger_source,parent_run_id) VALUES('$(sql_escape "$RUN_ID")','$(sql_escape "$HOST_NAME")','running','$(sql_escape "$TRIGGER_SOURCE")','$(sql_escape "$PARENT_RUN_ID")'); SELECT last_insert_rowid();")"

LAST_FAILED_COMMAND=""
STATUS="failed"; SUMMARY="VirtualBPM nightly guardian failed before completion."
GH_OPEN_COUNT=0; GH_ACTIONABLE_COUNT=0; TOP_GITHUB_ISSUES='[]'
BRANCH_NAME=""; PR_URL=""; FIXED_ITEMS=0; PROCESSED_ITEMS=0
DEPLOY_JSON='{"status":"unknown"}'; REPORT_JSON=""; REPORT_PATH="$REPORT_DIR/${RUN_ID}.json"

finalize_on_exit() {
  local exit_code=$? duration_sec=$(( $(date +%s) - STARTED_EPOCH )) current_status log_content error_detail summary status_value
  # Always restore gh auth
  restore_gh_user
  [[ -n "${RUN_ROW_ID:-}" ]] || return "$exit_code"
  current_status="$(sqlite3 "$DB_FILE" "SELECT status FROM nightly_jobs WHERE id=${RUN_ROW_ID};" 2>/dev/null || echo "")"
  [[ "$current_status" == "running" ]] || return "$exit_code"
  log_content=$(head -c 65536 "$LOG_FILE" 2>/dev/null || echo "")
  error_detail=""
  status_value="${STATUS:-ok}"
  summary="${SUMMARY:-}"
  if [[ "$exit_code" -ne 0 ]]; then
    error_detail="$(tail -50 "$LOG_FILE" 2>/dev/null || echo "")"
    status_value="failed"
    summary="${SUMMARY:-VirtualBPM nightly guardian failed (exit ${exit_code}). Last command: ${LAST_FAILED_COMMAND:-unknown}}"
    STATUS="$status_value"
    SUMMARY="$summary"
  fi
  REPORT_JSON="$(build_report_json "$exit_code" "$error_detail")"
  write_report_files
  sqlite3 "$DB_FILE" <<SQL >/dev/null 2>&1 || true
UPDATE nightly_jobs
SET finished_at = datetime('now'),
    status = '$(sql_escape "$status_value")',
    sentry_unresolved = 0,
    github_open_issues = ${GH_OPEN_COUNT},
    processed_items = ${PROCESSED_ITEMS},
    fixed_items = ${FIXED_ITEMS:-0},
    branch_name = '$(sql_escape "$BRANCH_NAME")',
    pr_url = '$(sql_escape "$PR_URL")',
    summary = '$(sql_escape "$summary")',
    report_json = '$(sql_escape "$REPORT_JSON")',
    log_stdout = '$(sql_escape "$log_content")',
    log_file_path = '$(sql_escape "$LOG_FILE")',
    duration_sec = ${duration_sec},
    config_snapshot = '$(sql_escape "$CONFIG_SNAPSHOT")',
    exit_code = ${exit_code},
    error_detail = '$(sql_escape "$error_detail")',
    trigger_source = '$(sql_escape "$TRIGGER_SOURCE")',
    parent_run_id = '$(sql_escape "$PARENT_RUN_ID")'
WHERE id = ${RUN_ROW_ID} AND status = 'running';
SQL
  if [[ "$exit_code" -ne 0 ]]; then insert_dashboard_notification "error" "critical" "VirtualBPM Nightly Guardian failed" "$summary" "$PR_URL"; log "FAILED: $summary"; fi
  return "$exit_code"
}
trap 'LAST_FAILED_COMMAND="$BASH_COMMAND"' ERR
trap finalize_on_exit EXIT

# ── Phase 1: GitHub issues triage (no Sentry for VirtualBPM) ──
log "Phase 1: GitHub issues triage"
GH_ALL_ISSUES="$(json_or_default '[]' gh issue list --repo "$REPO_SLUG" --state open --limit 40 --json number,title,url,labels,updatedAt)"
GH_OPEN_COUNT="$(echo "$GH_ALL_ISSUES" | jq 'length')"
GH_ACTIONABLE="$(echo "$GH_ALL_ISSUES" | jq -c '[ .[] | select(((.labels // []) | map(.name | ascii_downcase) | any(test("bug|regression|critical|production|incident"))) or ((.title // "") | ascii_downcase | test("error|crash|500|timeout|regression|incident"))) ]')"
GH_ACTIONABLE_COUNT="$(echo "$GH_ACTIONABLE" | jq 'length')"
TOP_GITHUB_ISSUES="$(echo "$GH_ACTIONABLE" | jq -c 'map({number,title,url}) | .[:3]')"

STATUS="ok"
SUMMARY="No actionable GitHub issues."
PROCESSED_ITEMS=$GH_ACTIONABLE_COUNT

# ── Phase 2: Fix flow ──
run_fix_flow() {
  cd "$REPO_PATH"
  git fetch origin "$DEFAULT_BRANCH" --quiet
  git checkout "$DEFAULT_BRANCH" --quiet
  git pull --rebase origin "$DEFAULT_BRANCH" --quiet
  BRANCH_NAME="nightly/guardian-$(date -u +%Y%m%d-%H%M)"; git checkout -B "$BRANCH_NAME" --quiet

  local prompt
  prompt=$(cat <<EOF
You are the VirtualBPM nightly maintenance Copilot agent.
Repository: ${REPO_SLUG}
Stack: Python 3.11 / Flask / Azure Container Apps
Default branch: ${DEFAULT_BRANCH}
Actionable GitHub issues: ${GH_ACTIONABLE_COUNT}
Top GitHub issues: ${TOP_GITHUB_ISSUES}

Execute a safe remediation sweep:
1. Fix only high-confidence regressions/errors linked to these items.
2. Avoid speculative refactors.
3. Run and pass:
   - ruff check scripts/python/ webapp/ --fix
   - cd scripts/python && python3.11 -m pytest -m "not integration and not slow" -q --tb=short
4. Commit with: fix: nightly guardian remediation
5. Do not force push and do not merge ${DEFAULT_BRANCH}.
EOF
)
  [[ -n "$PROJECT_AGENT_CONTENT" ]] && prompt="${prompt}"$'\n\n'"Repository-specific NightMaintenance runbook (MUST follow exactly):"$'\n'"${PROJECT_AGENT_CONTENT}"

  timeout "$FIX_TIMEOUT_SEC" copilot --yolo --add-dir "$REPO_PATH" --model "$MODEL" -p "$prompt"

  # Post-fix verification
  cd "$REPO_PATH"
  ruff check scripts/python/ webapp/ 2>/dev/null || log "WARNING: ruff check found issues"
  (cd scripts/python && python3.11 -m pytest -m "not integration and not slow" -q --tb=short 2>/dev/null) || log "WARNING: pytest found failures"

  git add -A
  if ! git diff --cached --quiet; then
    git commit -m "fix: nightly guardian remediation" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>" >/dev/null
  fi
  [[ "$(git rev-list --count "origin/${DEFAULT_BRANCH}..HEAD")" -eq 0 ]] && return 2

  FIXED_ITEMS="$(git diff --name-only "origin/${DEFAULT_BRANCH}...HEAD" | sed '/^$/d' | wc -l | tr -d ' ')"
  git push -u origin "$BRANCH_NAME" >/dev/null 2>&1
  PR_URL="$(gh pr create --repo "$REPO_SLUG" --base "$DEFAULT_BRANCH" --head "$BRANCH_NAME" \
    --title "fix: nightly guardian remediation ($(date -u +%F))" \
    --body "Automated nightly sweep for GitHub issues.\n\n- Actionable GitHub issues: ${GH_ACTIONABLE_COUNT}\n- Processed items: ${PROCESSED_ITEMS}" 2>/dev/null || true)"
  [[ -z "$PR_URL" ]] && PR_URL="$(gh pr list --repo "$REPO_SLUG" --head "$BRANCH_NAME" --state open --json url --jq '.[0].url' 2>/dev/null || true)"
  [[ -n "$PR_URL" && "$AUTO_MERGE" == "true" ]] && gh pr merge --repo "$REPO_SLUG" --squash --auto "$PR_URL" >/dev/null 2>&1 || true
}

if (( PROCESSED_ITEMS > 0 )); then
  if [[ "$RUN_FIXES" != "true" ]]; then
    STATUS="action_required"; SUMMARY="Issues detected, but auto-fix is disabled."
  else
    set +e; run_fix_flow; FIX_EXIT=$?; set -e
    if [[ "$FIX_EXIT" -eq 0 ]]; then
      STATUS="action_required"; SUMMARY="Nightly fixes prepared in PR for review/merge."
      [[ -n "$PR_URL" ]] && SUMMARY="Nightly fixes prepared: $PR_URL"
    elif [[ "$FIX_EXIT" -eq 2 ]]; then
      STATUS="action_required"; SUMMARY="Issues detected but no deterministic patch generated."
    else
      STATUS="failed"; SUMMARY="Nightly auto-fix flow failed."
    fi
  fi
fi

# ── Phase 3: Deploy status via GitHub Actions ──
log "Phase 3: Deploy status check"
LAST_DEPLOY_RUN="$(gh run list --repo "$REPO_SLUG" --workflow=deploy.yml --limit 1 --json status,conclusion,headBranch,createdAt 2>/dev/null || echo '[]')"
DEPLOY_CONCLUSION="$(echo "$LAST_DEPLOY_RUN" | jq -r '.[0].conclusion // "unknown"')"
DEPLOY_STATUS_RAW="$(echo "$LAST_DEPLOY_RUN" | jq -r '.[0].status // "unknown"')"
if [[ "$DEPLOY_CONCLUSION" == "success" ]]; then
  DEPLOY_JSON='{"status":"ready","source":"github-actions"}'
elif [[ "$DEPLOY_STATUS_RAW" == "in_progress" ]]; then
  DEPLOY_JSON='{"status":"in_progress","source":"github-actions"}'
else
  DEPLOY_JSON="{\"status\":\"$(echo "$DEPLOY_CONCLUSION" | jq -Rs '.' | tr -d '"')\",\"source\":\"github-actions\"}"
fi
DEPLOY_STATUS="$(echo "$DEPLOY_JSON" | jq -r '.status // "unknown"')"

if [[ "$STATUS" == "ok" && "$DEPLOY_STATUS" != "ready" ]]; then
  STATUS="action_required"
  SUMMARY="No new issues, but deploy status is ${DEPLOY_STATUS}."
fi

# ── Finalize ──
FINAL_EXIT_CODE=0
FINAL_ERROR_DETAIL=""
if [[ "$STATUS" == "failed" ]]; then FINAL_EXIT_CODE=1; FINAL_ERROR_DETAIL="$(tail -50 "$LOG_FILE" 2>/dev/null || echo "")"; fi

REPORT_JSON="$(build_report_json "$FINAL_EXIT_CODE" "$FINAL_ERROR_DETAIL")"
write_report_files

if [[ "$STATUS" == "action_required" ]]; then
  insert_dashboard_notification "warning" "warning" "VirtualBPM Nightly Guardian needs review" "$SUMMARY" "$PR_URL"
fi

if [[ "$STATUS" == "failed" ]]; then exit 1; fi

log "$STATUS: $SUMMARY"
log "Report: $REPORT_PATH"
