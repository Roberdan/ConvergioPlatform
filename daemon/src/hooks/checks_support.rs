use crate::hooks::checks::{CheckContext, CheckOutcome, DispatchState, HookCommand};

use super::checks_support_helpers::{
    contains_any, extract_base_cmd, extract_plan_id, select_account, GhAccountsConfig,
    INFRA_DRIFT_QUERY,
};
use super::checks_support_helpers::{GhMapping, PreflightSnapshot};

pub fn check_prefer_ci_summary(
    command: &HookCommand,
    _context: &CheckContext,
    state: &mut DispatchState,
) -> Result<CheckOutcome, String> {
    let base = extract_base_cmd(&command.command);
    if base.starts_with("gh run view") && command.command.contains("--log") {
        return Ok(CheckOutcome::Block(
            "Use: service-digest.sh ci <run-id>".to_string(),
        ));
    }
    if base.starts_with("gh pr merge") {
        return Ok(CheckOutcome::Block("Use: pr-ops.sh merge <pr>".to_string()));
    }
    if base.starts_with("gh pr view") && !command.command.contains("--json") {
        return Ok(CheckOutcome::Block(
            "Use: pr-ops.sh status <pr>".to_string(),
        ));
    }
    if base.starts_with("git diff") && !base.starts_with("git diff --stat") {
        return Ok(CheckOutcome::Block(
            "Use: git-digest.sh --full or diff-digest.sh".to_string(),
        ));
    }
    if base == "git status" {
        return Ok(CheckOutcome::Block("Use: git-digest.sh".to_string()));
    }
    if command.command.contains("wc -l")
        && !base.starts_with("git commit")
        && !base.starts_with("git tag")
    {
        state.notices.push("Hint: grep -c . <file>".to_string());
    }
    Ok(CheckOutcome::Continue)
}

pub fn check_gh_auto_token(
    command: &HookCommand,
    context: &CheckContext,
    state: &mut DispatchState,
) -> Result<CheckOutcome, String> {
    if command.command.is_empty()
        || !contains_any(
            &command.command,
            &["gh ", "git push", "git pull", "git fetch"],
        )
    {
        return Ok(CheckOutcome::Continue);
    }
    let path = context.home_dir.join(".claude/config/gh-accounts.json");
    if !path.exists() {
        return Ok(CheckOutcome::Continue);
    }
    let config: GhAccountsConfig =
        serde_json::from_str(&std::fs::read_to_string(path).map_err(|err| err.to_string())?)
            .map_err(|err| err.to_string())?;
    let Some(account) = select_account(&config, &context.cwd, &context.home_dir) else {
        return Ok(CheckOutcome::Continue);
    };
    if let Some(token) = context.gh_tokens.get(&account) {
        state.gh_token = Some(token.clone());
    }
    Ok(CheckOutcome::Continue)
}

pub fn check_warn_infra_plan_drift(
    command: &HookCommand,
    context: &CheckContext,
    state: &mut DispatchState,
) -> Result<CheckOutcome, String> {
    if !contains_any(
        &command.command,
        &[
            "az containerapp",
            "az acr ",
            "az postgres",
            "az redis ",
            "az keyvault",
            "az storage ",
            "az deployment group",
            "az webapp create",
            "az webapp update",
        ],
    ) {
        return Ok(CheckOutcome::Continue);
    }
    if let Some((pending, tasks)) = context.with_db(|conn| {
        conn.query_row(INFRA_DRIFT_QUERY, [], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
    })? {
        if pending > 0 {
            state.notices.push(format!(
                "[ADR-054] INFRA PLAN DRIFT WARNING\nMatching tasks:\n{tasks}"
            ));
        }
    }
    Ok(CheckOutcome::Continue)
}

pub fn check_enforce_execution_preflight(
    command: &HookCommand,
    context: &CheckContext,
    _state: &mut DispatchState,
) -> Result<CheckOutcome, String> {
    if !contains_any(
        &command.command,
        &[
            "execute-plan.sh",
            "copilot-worker.sh",
            "plan-db.sh start",
            "plan-db.sh validate-task",
            "plan-db.sh validate-wave",
            "wave-worktree.sh merge",
            "wave-worktree.sh batch",
        ],
    ) || command.command.contains("execution-preflight.sh")
    {
        return Ok(CheckOutcome::Continue);
    }
    let Some(plan_id) = extract_plan_id(&command.command).or(context.active_plan_id) else {
        return Ok(CheckOutcome::Continue);
    };
    let snapshot_path = context.preflight_dir.join(format!("plan-{plan_id}.json"));
    if !snapshot_path.exists() {
        return Ok(CheckOutcome::Deny(format!(
            "BLOCKED: missing execution preflight snapshot for plan {plan_id}. Run execution-preflight.sh --plan-id {plan_id} <worktree> before risky plan commands."
        )));
    }
    let snapshot: PreflightSnapshot = serde_json::from_str(
        &std::fs::read_to_string(snapshot_path).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())?;
    if context.now_epoch - snapshot.generated_epoch > 1800 {
        return Ok(CheckOutcome::Deny(format!(
            "BLOCKED: execution preflight for plan {plan_id} is stale. Refresh preflight before continuing."
        )));
    }
    if snapshot
        .warnings
        .iter()
        .any(|value| value == "dirty_worktree")
    {
        return Ok(CheckOutcome::Deny(format!(
            "BLOCKED: plan {plan_id} has dirty_worktree in the latest execution preflight snapshot."
        )));
    }
    if snapshot
        .warnings
        .iter()
        .any(|value| value == "gh_auth_not_ready")
    {
        return Ok(CheckOutcome::Deny(format!(
            "BLOCKED: plan {plan_id} has gh_auth_not_ready in the latest execution preflight snapshot."
        )));
    }
    Ok(CheckOutcome::Continue)
}

pub fn extract_worktree_add_path(command: &str) -> Option<String> {
    let parts: Vec<_> = command.split_whitespace().collect();
    let add_pos = parts.iter().position(|value| *value == "add")?;
    let mut index = add_pos + 1;
    if parts.get(index) == Some(&"-b") {
        index += 2;
    }
    parts.get(index).map(|value| (*value).to_string())
}

// Suppress unused import warnings — GhMapping and PreflightSnapshot are used
// transitively via the re-exported types in checks_support_helpers.
const _: () = {
    let _ = std::mem::size_of::<GhMapping>();
    let _ = std::mem::size_of::<PreflightSnapshot>();
};
