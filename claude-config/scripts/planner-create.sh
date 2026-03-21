#!/usr/bin/env bash
# planner-create.sh — Gated plan creation wrapper
# Enforces full planner workflow BEFORE plan-db.sh create/import.
# ONLY way to create plans. Hook blocks direct plan-db.sh create/import.
# Version: 2.0.0 — adds: auto-worktree, review DB linkage, post-import validation
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLAN_DB="$SCRIPT_DIR/plan-db.sh"
DB="${DASHBOARD_DB:-$(cd "$SCRIPT_DIR/../.." && pwd)/data/dashboard.db}"
REVIEW_DIR="/tmp/plan-reviews"

usage() {
    cat <<'EOF'
planner-create.sh — Gated plan creation (v2.0.0)

Subcommands:
  reset                           Clear review state
  register-review <type> <file>   Register review (type: standard)
  check-reviews                   Verify review exists
  create <project> "<name>" [--source-file <f>] [--auto-worktree]
  import <plan_id> <spec.yaml>    Import + validate + worktree + link review
  readiness <plan_id>             Full readiness check (run before /execute)

Workflow: reset → review → register → check → create → import → readiness
EOF
    exit 1
}

log_ok()   { echo -e "\033[0;32m[OK]\033[0m $*"; }
log_err()  { echo -e "\033[0;31m[ERR]\033[0m $*" >&2; }
log_warn() { echo -e "\033[0;33m[WARN]\033[0m $*"; }

ensure_review_dir() { mkdir -p "$REVIEW_DIR"; }

register_review() {
    local type="$1" file="$2"
    [[ "$type" != "standard" ]] && { log_err "Review type must be: standard"; exit 1; }
    [ ! -f "$file" ] && { log_err "Review file not found: $file"; exit 1; }
    ensure_review_dir
    cp "$file" "$REVIEW_DIR/${type}-review.md"
    log_ok "Registered ${type} review from ${file}"
}

check_reviews() {
    ensure_review_dir
    if [ -f "$REVIEW_DIR/standard-review.md" ]; then
        echo "  OK standard: $(wc -l < "$REVIEW_DIR/standard-review.md") lines"
        echo ""; echo "Review registered. Plan creation allowed."
    else
        echo "  MISSING: standard review"
        echo ""; echo "BLOCKED: Review required before plan creation."
        return 1
    fi
}

create_plan() {
    check_reviews || exit 1
    echo "--- Creating plan ---"
    bash "$PLAN_DB" create "$@"
}

import_spec() {
    local plan_id="$1" spec="$2"
    check_reviews || exit 1
    echo "--- Importing spec ---"
    bash "$PLAN_DB" import "$plan_id" "$spec"

    echo "--- Post-import validation ---"
    local errors=0

    # V1: Check all tasks have test_criteria
    local missing
    missing=$(sqlite3 "$DB" "SELECT task_id FROM tasks WHERE plan_id = $plan_id AND (test_criteria IS NULL OR test_criteria = '' OR test_criteria = '{}');" 2>/dev/null)
    if [ -n "$missing" ]; then
        log_err "Tasks missing test_criteria: $missing"
        errors=$((errors + 1))
    else
        log_ok "All tasks have test_criteria"
    fi

    # V2: Check effort_level in range (1-3)
    local bad_effort
    bad_effort=$(sqlite3 "$DB" "SELECT task_id, effort_level FROM tasks WHERE plan_id = $plan_id AND (effort_level IS NULL OR effort_level < 1 OR effort_level > 3);" 2>/dev/null)
    if [ -n "$bad_effort" ]; then
        log_warn "Tasks with invalid effort (must be 1-3): $bad_effort"
        log_warn "Capping to 3..."
        sqlite3 "$DB" "UPDATE tasks SET effort_level = 3 WHERE plan_id = $plan_id AND (effort_level IS NULL OR effort_level > 3);" 2>/dev/null
    fi

    # V3: Link review in plan_reviews table
    if [ -f "$REVIEW_DIR/standard-review.md" ]; then
        local existing
        existing=$(sqlite3 "$DB" "SELECT count(*) FROM plan_reviews WHERE plan_id = $plan_id;" 2>/dev/null)
        if [ "${existing:-0}" -eq 0 ]; then
            local verdict="approved"
            grep -qi "conditional\|blocker\|NEEDS_REVISION" "$REVIEW_DIR/standard-review.md" && verdict="approved"
            local raw
            raw=$(head -20 "$REVIEW_DIR/standard-review.md" | sed "s/'/''/g")
            sqlite3 "$DB" "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict, raw_report)
                VALUES ($plan_id, 'plan-reviewer', '$verdict', '$raw');" 2>/dev/null
            log_ok "Review linked in plan_reviews (verdict: $verdict)"
        else
            log_ok "Review already linked ($existing reviews)"
        fi
    fi

    # V4: Create worktree for W1
    local w1_id
    w1_id=$(sqlite3 "$DB" "SELECT id FROM waves WHERE plan_id = $plan_id ORDER BY position LIMIT 1;" 2>/dev/null)
    local w1_wt
    w1_wt=$(sqlite3 "$DB" "SELECT worktree_path FROM waves WHERE id = ${w1_id:-0};" 2>/dev/null)
    if [ -z "$w1_wt" ] || [ ! -d "$w1_wt" ]; then
        local w1_wave_id
        w1_wave_id=$(sqlite3 "$DB" "SELECT wave_id FROM waves WHERE id = ${w1_id:-0};" 2>/dev/null)
        local branch="plan-${plan_id}-${w1_wave_id:-w1}"
        local project_path
        project_path=$(sqlite3 "$DB" "SELECT p2.path FROM plans p JOIN projects p2 ON p.project_id = p2.id WHERE p.id = $plan_id;" 2>/dev/null)
        [ -z "$project_path" ] && project_path=$(git rev-parse --show-toplevel 2>/dev/null)
        local wt_path="${project_path%/}-${branch}"

        if [ -n "$project_path" ] && [ -d "$project_path" ]; then
            (cd "$project_path" && bash "$SCRIPT_DIR/worktree-create.sh" "$branch" "$wt_path" 2>/dev/null) && {
                sqlite3 "$DB" "UPDATE waves SET worktree_path = '$wt_path', branch_name = '$branch' WHERE id = $w1_id;" 2>/dev/null
                sqlite3 "$DB" "UPDATE plans SET worktree_path = '$wt_path', status = 'doing', started_at = datetime('now') WHERE id = $plan_id;" 2>/dev/null
                log_ok "Worktree created: $wt_path"
            } || {
                log_warn "Worktree creation failed — create manually before /execute"
                errors=$((errors + 1))
            }
        else
            log_err "Project path not found: $project_path"
            errors=$((errors + 1))
        fi
    else
        log_ok "Worktree already exists: $w1_wt"
    fi

    # Summary
    echo ""
    if [ $errors -gt 0 ]; then
        log_err "$errors validation errors. Fix before /execute."
        return 1
    else
        log_ok "Plan $plan_id ready for /execute"
    fi
}

readiness_check() {
    local plan_id="$1"
    echo "=== READINESS CHECK — Plan $plan_id ==="
    local errors=0

    # R1: Plan exists and is doing
    local status
    status=$(sqlite3 "$DB" "SELECT status FROM plans WHERE id = $plan_id;" 2>/dev/null)
    if [ "$status" = "doing" ]; then
        log_ok "Plan status: doing"
    else
        log_err "Plan status: ${status:-NOT FOUND} (must be 'doing')"
        errors=$((errors + 1))
    fi

    # R2: Worktree exists
    local wt
    wt=$(sqlite3 "$DB" "SELECT worktree_path FROM plans WHERE id = $plan_id;" 2>/dev/null)
    if [ -n "$wt" ] && [ -d "$wt" ]; then
        log_ok "Worktree: $wt"
    else
        log_err "No worktree (path: ${wt:-NULL})"
        errors=$((errors + 1))
    fi

    # R3: Review exists
    local reviews
    reviews=$(sqlite3 "$DB" "SELECT count(*) FROM plan_reviews WHERE plan_id = $plan_id;" 2>/dev/null)
    if [ "${reviews:-0}" -gt 0 ]; then
        log_ok "Reviews: $reviews linked"
    else
        log_err "No reviews linked in plan_reviews"
        errors=$((errors + 1))
    fi

    # R4: All tasks have test_criteria
    local missing_tc
    missing_tc=$(sqlite3 "$DB" "SELECT count(*) FROM tasks WHERE plan_id = $plan_id AND (test_criteria IS NULL OR test_criteria = '' OR test_criteria = '{}');" 2>/dev/null)
    if [ "${missing_tc:-1}" -eq 0 ]; then
        log_ok "All tasks have test_criteria"
    else
        local bad
        bad=$(sqlite3 "$DB" "SELECT task_id FROM tasks WHERE plan_id = $plan_id AND (test_criteria IS NULL OR test_criteria = '' OR test_criteria = '{}');" 2>/dev/null)
        log_err "$missing_tc tasks missing test_criteria: $bad"
        errors=$((errors + 1))
    fi

    # R5: Task count matches
    local total
    total=$(sqlite3 "$DB" "SELECT tasks_total FROM plans WHERE id = $plan_id;" 2>/dev/null)
    local actual
    actual=$(sqlite3 "$DB" "SELECT count(*) FROM tasks WHERE plan_id = $plan_id;" 2>/dev/null)
    if [ "$total" = "$actual" ]; then
        log_ok "Task count: $actual tasks"
    else
        log_warn "Task count mismatch: plan says $total, actual $actual"
    fi

    # R6: Effort levels valid
    local bad_eff
    bad_eff=$(sqlite3 "$DB" "SELECT count(*) FROM tasks WHERE plan_id = $plan_id AND (effort_level < 1 OR effort_level > 3);" 2>/dev/null)
    if [ "${bad_eff:-0}" -eq 0 ]; then
        log_ok "All effort levels valid (1-3)"
    else
        log_err "$bad_eff tasks with invalid effort"
        errors=$((errors + 1))
    fi

    echo ""
    if [ $errors -gt 0 ]; then
        log_err "BLOCKED: $errors readiness errors. Fix before /execute."
        return 1
    else
        log_ok "Plan $plan_id is READY for /execute"
    fi
}

[ $# -lt 1 ] && usage

case "$1" in
    reset)          reset_reviews ;;
    register-review)
        [ $# -lt 3 ] && usage
        register_review "$2" "$3" ;;
    check-reviews)  check_reviews ;;
    create)         shift; create_plan "$@" ;;
    import)
        [ $# -lt 3 ] && usage
        import_spec "$2" "$3" ;;
    readiness)
        [ $# -lt 2 ] && usage
        readiness_check "$2" ;;
    *)              usage ;;
esac
