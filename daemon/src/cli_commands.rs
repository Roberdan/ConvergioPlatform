// CLI Commands enum — all top-level subcommands for claude-core / cvg.
use crate::{
    cli_agent, cli_bus, cli_capability, cli_channel, cli_checkpoint, cli_delegation, cli_domain,
    cli_kb, cli_kernel, cli_launch, cli_lock, cli_memory, cli_ops, cli_plan, cli_project, cli_reap, cli_repo,
    cli_review, cli_run, cli_skill, cli_task, cli_voice, cli_wave, cli_who, cli_workspace,
};
use crate::ipc_handler::{DaemonCommands, IpcCommands};
use clap::Subcommand;
use std::path::PathBuf;

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
    /// Start the unified daemon (HTTP + mesh + Ali orchestrator + reaper)
    #[command(alias = "start")]
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
        args: convergio_core::ipc::cli::IpcArgs,
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
    /// Platform status overview (cvg status)
    Status {
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Chat with Ali orchestrator (cvg chat ["message"])
    Chat {
        /// Single message (non-interactive). Omit for REPL mode.
        message: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Channel commands (cvg channel list/status/test/send)
    Channel {
        #[command(subcommand)]
        command: cli_channel::ChannelCommands,
    },
    /// Capability system commands (cvg capability list/invoke/register/permissions)
    Capability {
        #[command(subcommand)]
        command: cli_capability::CapabilityCommands,
    },
    /// Agent memory commands (cvg memory remember/recall/forget/share/attest)
    Memory {
        #[command(subcommand)]
        command: cli_memory::MemoryCommands,
    },
    /// Voice pipeline commands (cvg voice start/stop/status/test)
    Voice {
        #[command(subcommand)]
        command: cli_voice::VoiceCommands,
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
    /// Reap stale worktrees, merged branches, and expired lock files (zero-zombie enforcement)
    Reap {
        #[command(subcommand)]
        command: cli_reap::ReapCommands,
    },
    /// Repository management commands (cvg repo add/list/show/link/sync)
    Repo {
        #[command(subcommand)]
        command: cli_repo::RepoCommands,
    },
    /// Kernel health monitor commands (cvg kernel start/stop/status)
    Kernel {
        #[command(subcommand)]
        command: cli_kernel::KernelCommands,
    },
    /// Print all available cvg commands grouped by domain
    #[command(alias = "commands")]
    Cheatsheet,
    /// List all daemon HTTP API endpoints
    Api,
    /// Launch a claude session with auto-registration (cvg claude <name>)
    #[command(name = "claude")]
    Claude {
        name: String,
        #[arg(long)] parent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
    /// Launch a copilot session with auto-registration (cvg copilot <name>)
    #[command(name = "copilot")]
    Copilot {
        name: String,
        #[arg(long)] parent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
}
