// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// CLI Commands enum — all top-level subcommands for claude-core / cvg.
// Kept in a separate module so main.rs stays under 250 lines.

use crate::cli_agent;
use crate::cli_kb;
use crate::cli_ops;
use crate::cli_plan;
use crate::cli_run;
use crate::cli_skill;
use crate::cli_support;
use crate::cli_task;
use crate::cli_wave;
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
        command: cli_support::CheckpointCommands,
    },
    /// File lock commands (acquire/release/list)
    Lock {
        #[command(subcommand)]
        command: cli_support::LockCommands,
    },
    /// Plan review commands (register/check/reset)
    Review {
        #[command(subcommand)]
        command: cli_support::ReviewCommands,
    },
    /// Audit project for violations: file sizes, token budget, copyright, constitution files
    Audit {
        /// Project root to audit (defaults to current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Skill commands (lint, transpile) — replaces skill-lint.sh and skill-transpile-*.sh
    Skill {
        #[command(subcommand)]
        command: cli_skill::SkillCommands,
    },
}
