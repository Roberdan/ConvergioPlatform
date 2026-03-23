// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// CLI Commands enum — all top-level subcommands for claude-core / cvg.
// Kept in a separate module so main.rs stays under 250 lines.

use crate::cli_agent;
use crate::cli_bus;
use crate::cli_checkpoint;
use crate::cli_delegation;
use crate::cli_domain;
use crate::cli_kb;
use crate::cli_lock;
use crate::cli_ops;
use crate::cli_plan;
use crate::cli_project;
use crate::cli_review;
use crate::cli_run;
use crate::cli_skill;
use crate::cli_task;
use crate::cli_wave;
use crate::cli_who;
use crate::cli_workspace;
use crate::ipc_handler::{DaemonCommands, IpcCommands};
use clap::Subcommand;
use std::path::PathBuf;

// cli_audit is referenced via Commands::Audit below; imported in main.rs dispatch.

#[derive(Debug, Subcommand)]
pub enum Commands {
    Db {
        #[arg(long)]
        db_path: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    Hook {
        /// pre or post
        mode: String,
    },
    Serve {
        #[arg(long, default_value = "0.0.0.0:8420")]
        bind: String,
        #[arg(long)]
        static_dir: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        /// Override DB path (default: $DASHBOARD_DB or ~/.claude/data/dashboard.db)
        #[arg(long)]
        db_path: Option<PathBuf>,
        /// Disable auth and bind to 127.0.0.1 only (development use only)
        #[arg(long)]
        dev_mode: bool,
        /// Enable mesh CRDT sync (Tailscale P2P on :9420). Default: enabled.
        /// Use --no-mesh to disable.
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        mesh: bool,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
    Ipc {
        #[command(flatten)]
        args: claude_core::ipc::cli::IpcArgs,
    },
    /// IPC Intelligence Layer commands (auth, models, budget, routing, skills)
    #[command(name = "ipc-intel")]
    IpcIntel {
        #[command(subcommand)]
        command: IpcCommands,
    },
    /// Launch the TUI dashboard (connects to daemon HTTP API)
    Tui {
        /// Daemon API base URL (default: http://localhost:8420)
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Plan management commands (cvg plan <subcommand>)
    Plan {
        #[command(subcommand)]
        command: cli_plan::PlanCommands,
    },
    /// Task management commands (cvg task <subcommand>)
    Task {
        #[command(subcommand)]
        command: cli_task::TaskCommands,
    },
    /// Wave management commands (cvg wave <subcommand>)
    Wave {
        #[command(subcommand)]
        command: cli_wave::WaveCommands,
    },
    /// Agent management commands (cvg agent start/complete/list)
    Agent {
        #[command(subcommand)]
        command: cli_agent::AgentCommands,
    },
    /// Knowledge base commands (cvg kb search/write)
    Kb {
        #[command(subcommand)]
        command: cli_kb::KbCommands,
    },
    /// Execution run commands (cvg run create/list/pause/resume)
    Run {
        #[command(subcommand)]
        command: cli_run::RunCommands,
    },
    /// Mesh commands (heartbeat, status, cluster-status)
    Mesh {
        #[command(subcommand)]
        command: cli_ops::MeshCommands,
    },
    /// Session commands (reap, recovery)
    Session {
        #[command(subcommand)]
        command: cli_ops::SessionCommands,
    },
    /// Checkpoint commands (save/restore plan state)
    Checkpoint {
        #[command(subcommand)]
        command: cli_checkpoint::CheckpointCommands,
    },
    /// File lock commands (acquire/release/list)
    Lock {
        #[command(subcommand)]
        command: cli_lock::LockCommands,
    },
    /// Plan review commands (register/check/reset)
    Review {
        #[command(subcommand)]
        command: cli_review::ReviewCommands,
    },
    /// Audit project for violations: file sizes, token budget, copyright, constitution files.
    /// With --project, runs a full project audit via daemon API instead.
    Audit {
        /// Project root to audit (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Run project-level audit (solve sessions, plans, tasks, runs, KB learnings)
        #[arg(long)]
        project: Option<String>,
        /// Write report to project output directory
        #[arg(long)]
        output: bool,
        /// Skip interactive confirmation (for --output)
        #[arg(long)]
        yes: bool,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Skill commands (lint, transpile) — replaces skill-lint.sh and skill-transpile-*.sh
    Skill {
        #[command(subcommand)]
        command: cli_skill::SkillCommands,
    },
    /// IPC bus commands (who/send/read/broadcast)
    Bus {
        #[command(subcommand)]
        command: cli_bus::BusCommands,
    },
    /// Project management commands (cvg project create/list/show)
    Project {
        #[command(subcommand)]
        command: cli_project::ProjectCommands,
    },
    /// Metrics commands (summary, collect)
    Metrics {
        #[command(subcommand)]
        command: cli_ops::MetricsCommands,
    },
    /// Alert / notification commands (list)
    Alert {
        #[command(subcommand)]
        command: cli_ops::AlertCommands,
    },
    /// Domain→skill mapping commands (cvg domain list/map)
    Domain {
        #[command(subcommand)]
        command: cli_domain::DomainCommands,
    },
    /// Workspace management commands (cvg workspace create/delete/list/status/events)
    Workspace {
        #[command(subcommand)]
        command: cli_workspace::WorkspaceCommands,
    },
    /// Who is working on what? Show active agents, delegations, per-plan workers.
    Who {
        #[command(subcommand)]
        command: cli_who::WhoCommands,
    },
    /// Delegation management (delegate plans to mesh workers, track status)
    Delegation {
        #[command(subcommand)]
        command: cli_delegation::DelegationCommands,
    },
}
