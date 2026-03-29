// Ali escalation and deterministic problem triage for Jarvis recovery.
// Known problems get auto-fixed; unknown problems are escalated to Ali via copilot-plan-runner.

use std::process::Command;
use tracing::{info, warn};

use super::recover::RecoveryConfig;

// ----- Problem classification -------------------------------------------------

/// Classifies a problem description into a known or unknown category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemClass {
    DaemonCrash,
    TelegramPollDead,
    DbLocked,
    StaleWorktrees,
    HighFdCount,
    Unknown,
}

/// Map a free-text problem string to a ProblemClass by keyword matching.
pub fn classify_problem(problem: &str) -> ProblemClass {
    let lower = problem.to_lowercase();
    if lower.contains("daemon_crash") || lower.contains("daemon_health") {
        ProblemClass::DaemonCrash
    } else if lower.contains("telegram_poll") {
        ProblemClass::TelegramPollDead
    } else if lower.contains("db_locked") || lower.contains("database is locked") {
        ProblemClass::DbLocked
    } else if lower.contains("stale_worktree") {
        ProblemClass::StaleWorktrees
    } else if lower.contains("high_fd") || lower.contains("file descriptor") {
        ProblemClass::HighFdCount
    } else {
        ProblemClass::Unknown
    }
}

// ----- Ali escalation ---------------------------------------------------------

/// Escalate an unknown problem to Ali by creating a micro-plan and launching copilot-plan-runner.
/// Returns Ok(plan_name) on success. Skips external commands when cfg.dry_run is true.
pub async fn escalate_to_ali(
    problem: &str,
    fix_instructions: &str,
    cfg: &RecoveryConfig,
) -> Result<String, String> {
    // Build plan name, capped at 80 chars total.
    let prefix = "Jarvis Alert: ";
    let max_problem_len = 80 - prefix.len();
    let truncated = if problem.len() > max_problem_len {
        &problem[..max_problem_len]
    } else {
        problem
    };
    let plan_name = format!("{prefix}{truncated}");

    let project_id =
        std::env::var("CONVERGIO_PROJECT_ID").unwrap_or_else(|_| "convergio".to_string());

    if cfg.dry_run {
        info!(
            "jarvis.recover: [dry_run] escalate_to_ali — plan_name='{}' project={} instructions='{}'",
            plan_name, project_id, fix_instructions
        );
        return Ok(plan_name);
    }

    // Create the plan via cvg CLI; expect JSON {"ok":true,"plan_id":NNN}.
    let out = Command::new("cvg")
        .args(["plan", "create", &project_id, &plan_name])
        .output()
        .map_err(|e| format!("escalate_to_ali: cvg plan create exec error: {e}"))?;

    if !out.status.success() {
        return Err(format!(
            "escalate_to_ali: cvg plan create failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let plan_id = parse_plan_id(&stdout)
        .ok_or_else(|| format!("escalate_to_ali: could not parse plan_id from: {stdout}"))?;

    info!("jarvis.recover: escalated to Ali — plan {plan_id}: {problem}");

    // Launch copilot-plan-runner.sh in background via tmux; fall back to direct spawn.
    let tmux_cmd = format!("copilot-plan-runner.sh {plan_id}");
    let tmux_ok = Command::new("tmux")
        .args(["send-keys", "-t", "convergio", &tmux_cmd, "Enter"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !tmux_ok {
        warn!("jarvis.recover: tmux unavailable — spawning copilot-plan-runner.sh directly");
        Command::new("copilot-plan-runner.sh")
            .arg(plan_id.to_string())
            .spawn()
            .map_err(|e| format!("escalate_to_ali: spawn copilot-plan-runner failed: {e}"))?;
    }

    // Notify via Telegram.
    let msg = format!("Jarvis escalated to Ali: {problem}");
    super::telegram::communicate(&msg, super::recover::Severity::Warn, cfg.dry_run)
        .await
        .unwrap_or_else(|e| warn!("jarvis.recover: telegram notify failed: {e}"));

    Ok(plan_name)
}

/// Extract numeric plan_id from JSON like {"ok":true,"plan_id":42}.
fn parse_plan_id(json: &str) -> Option<u64> {
    // Simple non-serde parse: find "plan_id": and read the number.
    let key = "\"plan_id\":";
    let pos = json.find(key)?;
    let rest = json[pos + key.len()..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    match rest[..end].parse() {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

// ----- Triage and auto-fix ---------------------------------------------------

/// Classify a problem and either auto-fix or escalate to Ali.
/// Returns Ok(action_description). Skips real commands when cfg.dry_run is true.
pub async fn triage_and_recover(
    problem: &str,
    details: &str,
    cfg: &RecoveryConfig,
) -> Result<String, String> {
    let action = match classify_problem(problem) {
        ProblemClass::DaemonCrash => fix_daemon_crash(cfg),
        ProblemClass::TelegramPollDead => fix_telegram_poll(),
        ProblemClass::DbLocked => fix_db_locked(),
        ProblemClass::StaleWorktrees => fix_stale_worktrees(cfg),
        ProblemClass::HighFdCount => fix_high_fd(problem, cfg).await,
        ProblemClass::Unknown => {
            let plan_name = escalate_to_ali(problem, details, cfg).await?;
            format!("escalated to Ali: {plan_name}")
        }
    };
    Ok(action)
}

// ----- Known fix implementations ---------------------------------------------

fn fix_daemon_crash(cfg: &RecoveryConfig) -> String {
    if cfg.dry_run {
        info!("jarvis.recover: [dry_run] daemon start.sh skipped");
        return "daemon restart skipped (dry_run) — start.sh".to_string();
    }
    info!("jarvis.recover: DaemonCrash — running daemon/start.sh");
    match Command::new("bash").args(["daemon/start.sh"]).status() {
        Ok(s) if s.success() => {
            info!("jarvis.recover: daemon/start.sh succeeded");
            "daemon restarted via start.sh".to_string()
        }
        Ok(s) => {
            warn!("jarvis.recover: daemon/start.sh exit={s}");
            format!("daemon/start.sh failed with exit={s}")
        }
        Err(e) => {
            warn!("jarvis.recover: daemon/start.sh exec error: {e}");
            format!("daemon/start.sh exec error: {e}")
        }
    }
}

fn fix_telegram_poll() -> String {
    // Why: the monitor loop handles Telegram poll restarts; no action needed here.
    info!("jarvis.recover: Telegram poll restart delegated to monitor");
    "telegram poll restart delegated to monitor".to_string()
}

fn fix_db_locked() -> String {
    // Why: PRAGMA busy_timeout=5000 handles retries transparently.
    info!("jarvis.recover: DB lock — busy_timeout handles retries");
    "db busy_timeout handles retries — no action".to_string()
}

fn fix_stale_worktrees(cfg: &RecoveryConfig) -> String {
    if cfg.dry_run {
        info!("jarvis.recover: [dry_run] git worktree prune skipped");
        return "git worktree prune skipped (dry_run)".to_string();
    }
    info!("jarvis.recover: StaleWorktrees — running git worktree prune");
    match Command::new("git").args(["worktree", "prune"]).status() {
        Ok(s) if s.success() => {
            info!("jarvis.recover: git worktree prune succeeded");
            "git worktree prune completed".to_string()
        }
        Ok(s) => {
            warn!("jarvis.recover: git worktree prune exit={s}");
            format!("git worktree prune failed exit={s}")
        }
        Err(e) => {
            warn!("jarvis.recover: git worktree prune exec error: {e}");
            format!("git worktree prune exec error: {e}")
        }
    }
}

async fn fix_high_fd(problem: &str, cfg: &RecoveryConfig) -> String {
    warn!("jarvis.recover: HighFdCount detected — {problem}");
    let msg = format!("High file descriptor count detected: {problem}. Consider restarting.");
    super::telegram::communicate(&msg, super::recover::Severity::Warn, cfg.dry_run)
        .await
        .unwrap_or_else(|e| warn!("jarvis.recover: telegram high_fd notify failed: {e}"));
    format!("high fd warning sent — restart recommended: {problem}")
}
