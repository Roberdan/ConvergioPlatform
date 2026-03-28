// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// `cvg cheatsheet` — print all available cvg commands grouped by domain.

pub fn cheatsheet_text() -> &'static str {
    r#"cvg cheatsheet — all available commands

PLAN MANAGEMENT
  cvg plan list [--human]              List active plans
  cvg plan show <id> [--human]         Show plan JSON
  cvg plan tree <id> [--human]         Execution tree (waves + tasks)
  cvg plan create <proj> "name"        Create a new plan
  cvg plan import <id> spec.yaml       Import spec YAML into plan
  cvg plan template                    Print example spec YAML
  cvg plan start <id>                  Begin execution
  cvg plan complete <id>               Mark plan as complete
  cvg plan cancel <id> "reason"        Cancel with reason
  cvg plan approve <id>                Approve for execution
  cvg plan readiness <id>              Pre-execution checks
  cvg plan validate <id>               Thor wave-level validation
  cvg plan drift <id>                  Check plan staleness

TASK MANAGEMENT
  cvg task update <id> <status>        Update task status
  cvg task validate <id> <plan>        Validate individual task
  cvg task approve <id>                Approve task output

WAVE MANAGEMENT
  cvg wave create <plan> <wave>        Create wave worktree
  cvg wave merge <plan> <wave>         Merge wave PR

AGENT MANAGEMENT
  cvg agent start "<name>"             Register agent session
  cvg agent complete "<name>"          Mark agent session done
  cvg agent list                       List registered agents
  cvg who agents                       Active agents across mesh

CHECKPOINT & RECOVERY
  cvg checkpoint save <plan_id>        Snapshot plan state
  cvg checkpoint restore <plan_id>     Restore from snapshot
  cvg reap worktrees [--dry-run]       Clean stale worktrees
  cvg reap branches [--dry-run]        Clean merged branches
  cvg reap locks [--dry-run]           Clean expired locks

MESH & DELEGATION
  cvg mesh status                      Peer topology
  cvg mesh heartbeat                   Send heartbeat
  cvg delegation start <plan> <peer>   Delegate plan to peer
  cvg delegation status <plan>         Check delegation progress
  cvg delegation cancel <plan>         Cancel delegation
  cvg delegation list                  List active delegations

KERNEL
  cvg kernel start                     Start kernel health monitor
  cvg kernel stop                      Stop kernel
  cvg kernel status                    Kernel status
  cvg kernel here                      Set this machine as audio node
  cvg kernel say "<text>"              TTS on active audio node

PROJECT & REPO
  cvg project create <name>            Create project
  cvg project list                     List projects
  cvg project show <id>                Show project details
  cvg repo add <name> --path <p>       Register repository
  cvg repo list                        List registered repos
  cvg repo show <name>                 Repository details
  cvg repo link <name> <project-id>    Link repo to project
  cvg repo sync                        Health-check all repos

KNOWLEDGE BASE
  cvg kb search "<query>"              Search knowledge base
  cvg kb write "<entry>"               Write KB entry

WORKSPACE
  cvg workspace create <name>          Create workspace
  cvg workspace list                   List workspaces
  cvg workspace status <id>            Workspace status
  cvg workspace events <id>            Workspace events
  cvg workspace delete <id>            Delete workspace

REVIEW & AUDIT
  cvg review register                  Register plan review
  cvg review check <plan_id>           Check review status
  cvg audit --path .                   Audit file sizes, tokens
  cvg audit --project <id>             Full project audit

OTHER
  cvg status                           Platform overview
  cvg chat ["message"]                 Chat with Ali
  cvg bus who                          Active IPC agents
  cvg bus send <msg>                   Send IPC message
  cvg channel list                     List channels
  cvg skill lint <file>                Lint skill file
  cvg memory recall "<query>"          Recall from memory
  cvg voice start                      Start voice pipeline
  cvg lock acquire <path>              Acquire file lock
  cvg metrics summary                  Metrics overview
  cvg domain list                      Domain-skill mappings
  cvg run create <name>                Create execution run
  cvg cheatsheet                       This help text
"#
}

pub fn print_cheatsheet() {
    print!("{}", cheatsheet_text());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheatsheet_covers_major_domains() {
        let text = cheatsheet_text();
        let domains = [
            "PLAN MANAGEMENT", "TASK MANAGEMENT", "WAVE MANAGEMENT",
            "AGENT MANAGEMENT", "CHECKPOINT & RECOVERY", "MESH & DELEGATION",
            "KERNEL", "PROJECT & REPO", "KNOWLEDGE BASE", "WORKSPACE",
            "REVIEW & AUDIT", "OTHER",
        ];
        for domain in domains {
            assert!(text.contains(domain), "must cover domain '{domain}'");
        }
    }

    #[test]
    fn cheatsheet_includes_new_commands() {
        let text = cheatsheet_text();
        assert!(text.contains("cvg plan template"), "must include plan template");
        assert!(text.contains("cvg cheatsheet"), "must include self-reference");
    }
}
