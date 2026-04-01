// CLI Commands enum — all top-level subcommands for claude-core / cvg.
use crate::{cli_agent, cli_bus, cli_capability, cli_channel, cli_checkpoint,
    cli_delegation, cli_domain, cli_kb, cli_kernel, cli_lock, cli_memory, cli_ops,
    cli_org, cli_plan, cli_project, cli_reap, cli_repo, cli_review, cli_run,
    cli_skill, cli_task, cli_voice, cli_wave, cli_who, cli_workspace, ipc_handler::{DaemonCommands, IpcCommands}};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Interactive first-run wizard
    #[command(next_help_heading = "User Commands")]
    Setup { #[arg(long)] defaults: bool },
    /// Platform overview (plans, agents, mesh health)
    Status {
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Plan management (list, show, create, start, validate)
    Plan {
        #[command(subcommand)]
        command: cli_plan::PlanCommands,
    },
    /// Project management (create, list, show)
    Project {
        #[command(subcommand)]
        command: cli_project::ProjectCommands,
    },
    /// Organization management (create, list, show)
    Org {
        #[command(subcommand)]
        command: cli_org::OrgCommands,
    },
    /// Chat with Ali orchestrator (cvg chat ["message"])
    Chat {
        /// Single message (non-interactive). Omit for REPL mode.
        message: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Show who is working and on what
    Who {
        #[command(subcommand)]
        command: cli_who::WhoCommands,
    },
    /// Launch terminal dashboard
    Tui {
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Print all commands grouped by domain
    #[command(alias = "commands")]
    Cheatsheet,
    /// Agent session management (start, complete, list, history)
    #[command(next_help_heading = "Operations")]
    Agent {
        #[command(subcommand)]
        command: cli_agent::AgentCommands,
    },
    /// Task management (list, update, create)
    Task {
        #[command(subcommand)]
        command: cli_task::TaskCommands,
    },
    /// Wave management (list, status)
    Wave {
        #[command(subcommand)]
        command: cli_wave::WaveCommands,
    },
    /// Mesh commands (heartbeat, status, cluster-status)
    Mesh {
        #[command(subcommand)]
        command: cli_ops::MeshCommands,
    },
    /// Delegate work to mesh peers
    Delegation {
        #[command(subcommand)]
        command: cli_delegation::DelegationCommands,
    },
    /// Channel management (Telegram, Slack, email)
    Channel {
        #[command(subcommand)]
        command: cli_channel::ChannelCommands,
    },
    /// Kernel/Jarvis commands (start, stop, status)
    Kernel {
        #[command(subcommand)]
        command: cli_kernel::KernelCommands,
    },
    /// Voice pipeline commands (start, stop, status, test)
    Voice {
        #[command(subcommand)]
        command: cli_voice::VoiceCommands,
    },
    /// Workspace management (create, delete, list, status)
    Workspace {
        #[command(subcommand)]
        command: cli_workspace::WorkspaceCommands,
    },
    /// Execution run commands (create, list, pause, resume)
    Run {
        #[command(subcommand)]
        command: cli_run::RunCommands,
    },
    /// Save/restore plan state
    Checkpoint {
        #[command(subcommand)]
        command: cli_checkpoint::CheckpointCommands,
    },
    /// Plan review management (register, check, reset)
    Review {
        #[command(subcommand)]
        command: cli_review::ReviewCommands,
    },
    /// Audit project for violations
    Audit {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        output: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    /// Metrics and cost summary
    Metrics {
        #[command(subcommand)]
        command: cli_ops::MetricsCommands,
    },
    /// Capability system (list, invoke, register, permissions)
    Capability {
        #[command(subcommand)]
        command: cli_capability::CapabilityCommands,
    },
    /// Agent memory (remember, recall, forget, share, attest)
    Memory {
        #[command(subcommand)]
        command: cli_memory::MemoryCommands,
    },
    /// Knowledge base (search, write)
    Kb {
        #[command(subcommand)]
        command: cli_kb::KbCommands,
    },
    /// Start the unified daemon (HTTP + mesh + orchestrator)
    #[command(alias = "start", next_help_heading = "Internal (agents/scripts)")]
    Serve {
        #[arg(long, default_value = "0.0.0.0:8420")]
        bind: String,
        #[arg(long)]
        static_dir: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
        #[arg(long)]
        dev_mode: bool,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        mesh: bool,
    },
    /// Database operations
    Db {
        #[arg(long)]
        db_path: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Hook management (pre/post)
    Hook {
        mode: String,
    },
    /// Daemon control
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
    /// IPC bus operations
    Ipc {
        #[command(flatten)]
        args: convergio_core::ipc::cli::IpcArgs,
    },
    /// IPC intelligence layer (auth, models, budget, routing)
    #[command(name = "ipc-intel")]
    IpcIntel {
        #[command(subcommand)]
        command: IpcCommands,
    },
    /// Message bus commands (who, send, read, broadcast)
    Bus {
        #[command(subcommand)]
        command: cli_bus::BusCommands,
    },
    /// File lock management (acquire, release, list)
    Lock {
        #[command(subcommand)]
        command: cli_lock::LockCommands,
    },
    /// Session commands (reap, recovery)
    Session {
        #[command(subcommand)]
        command: cli_ops::SessionCommands,
    },
    /// Skill transpilation (lint, transpile)
    Skill {
        #[command(subcommand)]
        command: cli_skill::SkillCommands,
    },
    /// Reaper commands (stale session cleanup)
    Reap {
        #[command(subcommand)]
        command: cli_reap::ReapCommands,
    },
    /// Repository management (add, list, show, link, sync)
    Repo {
        #[command(subcommand)]
        command: cli_repo::RepoCommands,
    },
    /// Domain-skill mapping (list, map)
    Domain {
        #[command(subcommand)]
        command: cli_domain::DomainCommands,
    },
    /// Alert / notification commands
    Alert {
        #[command(subcommand)]
        command: cli_ops::AlertCommands,
    },
    /// List all daemon HTTP API endpoints
    Api,
    /// Launch a claude session with auto-registration
    #[command(name = "claude")]
    Claude {
        name: String,
        #[arg(long)] parent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
    /// Launch a copilot session with auto-registration
    #[command(name = "copilot")]
    Copilot {
        name: String,
        #[arg(long)] parent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
}
