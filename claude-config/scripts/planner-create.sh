#!/usr/bin/env bash
# planner-create.sh — Gated plan creation wrapper
# Enforces full planner workflow BEFORE plan-db.sh create/import.
# ONLY way to create plans. Hook blocks direct plan-db.sh create/import.
# Version: 1.0.0
set -euo pipefail

REVIEW_DIR="/tmp/plan-reviews"

usage() {
    echo "Usage: planner-create.sh <subcommand> [args]"
    echo ""
    echo "Subcommands:"
    echo "  register-review <type> <file>   Register a completed review (standard)"
    echo "  check-reviews                   Check if review is registered"
    echo "  create <project> <name> [opts]  Create plan (requires review)"
    echo "  import <plan_id> <spec>         Import spec (requires review)"
    echo "  reset                           Clear review state for new plan"
    echo ""
    echo "Workflow: register review → create → import"
    exit 1
}

ensure_review_dir() {
    mkdir -p "$REVIEW_DIR"
}

register_review() {
    local type="$1"
    local file="$2"

    if [[ "$type" != "standard" ]]; then
        echo "ERROR: Review type must be: standard"
        exit 1
    fi

    if [ ! -f "$file" ]; then
        echo "ERROR: Review file not found: $file"
        exit 1
    fi

    ensure_review_dir
    cp "$file" "$REVIEW_DIR/${type}-review.md"
    echo "OK: Registered ${type} review from ${file}"
}

check_reviews() {
    ensure_review_dir
    if [ -f "$REVIEW_DIR/standard-review.md" ]; then
        echo "  OK standard: $(wc -l < "$REVIEW_DIR/standard-review.md") lines"
        echo ""
        echo "Review registered. Plan creation allowed."
    else
        echo "  MISSING: standard review"
        echo ""
        echo "BLOCKED: Review required before plan creation."
        echo "Run review agent first, then register with:"
        echo "  planner-create.sh register-review standard <file>"
        return 1
    fi
    return 0
}

create_plan() {
    if ! check_reviews; then
        exit 1
    fi
    echo "--- Creating plan ---"
    plan-db.sh create "$@"
}

import_spec() {
    if ! check_reviews; then
        exit 1
    fi
    echo "--- Importing spec ---"
    plan-db.sh import "$@"
}

reset_reviews() {
    rm -rf "$REVIEW_DIR"
    echo "OK: Review state cleared for new plan."
}

[ $# -lt 1 ] && usage

case "$1" in
    register-review)
        [ $# -lt 3 ] && usage
        register_review "$2" "$3"
        ;;
    check-reviews)
        check_reviews
        ;;
    create)
        shift
        create_plan "$@"
        ;;
    import)
        shift
        import_spec "$@"
        ;;
    reset)
        reset_reviews
        ;;
    *)
        usage
        ;;
esac
