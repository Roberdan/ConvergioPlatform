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

DAEMON_API="http://localhost:8420"
command -v jq &>/dev/null || { echo "ERROR: jq required" >&2; exit 1; }

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

REPORT_DIR="$CLAUDE_HOME/data/nightly-jobs"
RUN_ID="virtualbpm-nightly-$(date -u +%Y%m%d-%H%M%S)"
STARTED_EPOCH=$(date +%s)

log() { printf '[virtualbpm-nightly] %s\n' "$*"; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || { log "Missing command: $1"; exit 1; }; }
json_or_default() {
  local default_json="$1"; shift
  local raw
  raw="$("$@" 2>/dev/null || true)"
  if [[ -n "$raw" ]] && jq -e . >/dev/null 2>&1 <<<"$raw"; then printf '%s' "$raw"; else printf '%s' "$default_json"; fi
}

insert_dashboard_notification() {
  local notif_type="$1" severity="$2" title="$3" message="$4" link="${5:-}"
  curl -sf -X POST "${DAEMON_API}/api/notify" \
    -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg pid "virtualbpm" --arg t "$notif_type" --arg sev "$severity" \
      --arg title "$title" --arg msg "$message" --arg link "$link" --arg rid "$RUN_ID" \
      '{project_id:$pid, type:$t, severity:$sev, title:$title, message:$msg, link:$link, source_table:"nightly_jobs", source_id:$rid}')" 2>/dev/null || log "WARNING: failed to persist dashboard notification"
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

# ── Prerequisites ──
require_cmd jq; require_cmd git; require_cmd gh
[[ "$RUN_FIXES" == "true" ]] && require_cmd copilot
[[ -d "$REPO_PATH/.git" ]] || { log "Repository not found at $REPO_PATH"; exit 1; }

# ── Enabled-flag check (soft pause from dashboard) ──
_enabled="$(curl -sf "${DAEMON_API}/api/overview" 2>/dev/null | jq -r '.nightly_jobs_definitions // [] | .[] | select(.project_id == "virtualbpm") | .enabled // 1' 2>/dev/null || echo 1)"
if [[ "$_enabled" == "0" ]]; then
  log "Guardian paused via dashboard (enabled=0). Exiting gracefully."
  exit 0
fi

mkdir -p "$REPORT_DIR"
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

# Schema is managed by the daemon — tables are auto-created
HOST_NAME="$(hostname -s 2>/dev/null || echo unknown)"
# Register this nightly job run via daemon API
_run_result="$(curl -sf -X POST "${DAEMON_API}/api/tracking/session-state" \
  -H 'Content-Type: application/json' \
  -d "$(jq -nc --arg rid "$RUN_ID" --arg host "$HOST_NAME" --arg trigger "$TRIGGER_SOURCE" --arg parent "$PARENT_RUN_ID" \
    '{run_id:$rid, host:$host, status:"running", trigger_source:$trigger, parent_run_id:$parent, type:"nightly_job"}')" 2>/dev/null || echo '{}')"
RUN_ROW_ID="$(echo "$_run_result" | jq -r '.id // "0"' 2>/dev/null || echo "0")"

LAST_FAILED_COMMAND=""
STATUS="failed"; SUMMARY="VirtualBPM nightly guardian failed before completion."
GH_OPEN_COUNT=0; GH_ACTIONABLE_COUNT=0; TOP_GITHUB_ISSUES='[]'
BRANCH_NAME=""; PR_URL=""; FIXED_ITEMS=0; PROCESSED_ITEMS=0
DEPLOY_JSON='{"status":"unknown"}'; REPORT_JSON=""; REPORT_PATH="$REPORT_DIR/${RUN_ID}.json"

finalize_on_exit() {
  local exit_code=$? duration_sec=$(( $(date +%s) - STARTED_EPOCH )) error_detail summary status_value
  # Always restore gh auth
  restore_gh_user
  [[ -n "${RUN_ROW_ID:-}" && "$RUN_ROW_ID" != "0" ]] || return "$exit_code"
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
  # Update nightly job status via daemon API
  curl -sf -X POST "${DAEMON_API}/api/tracking/session-state" \
    -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg rid "$RUN_ID" --arg status "$status_value" --arg summary "$summary" \
      --argjson exit_code "$exit_code" --argjson duration "$duration_sec" \
      --arg branch "$BRANCH_NAME" --arg pr "$PR_URL" --arg log_path "$LOG_FILE" \
      --argjson gh "${GH_OPEN_COUNT}" \
      --argjson processed "${PROCESSED_ITEMS}" --argjson fixed "${FIXED_ITEMS:-0}" \
      '{run_id:$rid, status:$status, summary:$summary, exit_code:$exit_code,
       duration_sec:$duration, branch_name:$branch, pr_url:$pr, log_file_path:$log_path,
       sentry_unresolved:0, github_open_issues:$gh,
       processed_items:$processed, fixed_items:$fixed, type:"nightly_job_update"}')" 2>/dev/null || true
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

  timeout "$FIX_TIMEOUT_SEC" copilot --dangerously-skip-permissions --add-dir "$REPO_PATH" --model "$MODEL" -p "$prompt"

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
