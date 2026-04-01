// CLI Commands enum — all top-level subcommands for claude-core / cvg.
use crate::{cli_agent, cli_bus, cli_capability, cli_channel, cli_checkpoint,
    cli_delegation, cli_domain, cli_kb, cli_kernel, cli_lock, cli_memory, cli_ops,
    cli_org, cli_plan, cli_project, cli_reap, cli_repo, cli_review, cli_run,
    cli_skill, cli_task, cli_voice, cli_wave, cli_who, cli_workspace, ipc_handler::{DaemonCommands, IpcCommands}};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(next_help_heading = "User Commands")]
    Setup { #[arg(long)] defaults: bool },
    Status {
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Plan {
        #[command(subcommand)]
        command: cli_plan::PlanCommands,
    },
    Project {
        #[command(subcommand)]
        command: cli_project::ProjectCommands,
    },
    Org {
        #[command(subcommand)]
        command: cli_org::OrgCommands,
    },
    Chat {
        /// Single message (non-interactive). Omit for REPL mode.
        message: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Who {
        #[command(subcommand)]
        command: cli_who::WhoCommands,
    },
    Tui {
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    #[command(alias = "commands")]
    Cheatsheet,
    #[command(next_help_heading = "Operations")]
    Agent {
        #[command(subcommand)]
        command: cli_agent::AgentCommands,
    },
    Task {
        #[command(subcommand)]
        command: cli_task::TaskCommands,
    },
    Wave {
        #[command(subcommand)]
        command: cli_wave::WaveCommands,
    },
    Mesh {
        #[command(subcommand)]
        command: cli_ops::MeshCommands,
    },
    Delegation {
        #[command(subcommand)]
        command: cli_delegation::DelegationCommands,
    },
    Channel {
        #[command(subcommand)]
        command: cli_channel::ChannelCommands,
    },
    Kernel {
        #[command(subcommand)]
        command: cli_kernel::KernelCommands,
    },
    Voice {
        #[command(subcommand)]
        command: cli_voice::VoiceCommands,
    },
    Workspace {
        #[command(subcommand)]
        command: cli_workspace::WorkspaceCommands,
    },
    Run {
        #[command(subcommand)]
        command: cli_run::RunCommands,
    },
    Checkpoint {
        #[command(subcommand)]
        command: cli_checkpoint::CheckpointCommands,
    },
    Review {
        #[command(subcommand)]
        command: cli_review::ReviewCommands,
    },
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
    Metrics {
        #[command(subcommand)]
        command: cli_ops::MetricsCommands,
    },
    Capability {
        #[command(subcommand)]
        command: cli_capability::CapabilityCommands,
    },
    Memory {
        #[command(subcommand)]
        command: cli_memory::MemoryCommands,
    },
    Kb {
        #[command(subcommand)]
        command: cli_kb::KbCommands,
    },
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
    Db {
        #[arg(long)]
        db_path: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    Hook {
        mode: String,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
    Ipc {
        #[command(flatten)]
        args: convergio_core::ipc::cli::IpcArgs,
    },
    #[command(name = "ipc-intel")]
    IpcIntel {
        #[command(subcommand)]
        command: IpcCommands,
    },
    Bus {
        #[command(subcommand)]
        command: cli_bus::BusCommands,
    },
    Lock {
        #[command(subcommand)]
        command: cli_lock::LockCommands,
    },
    Session {
        #[command(subcommand)]
        command: cli_ops::SessionCommands,
    },
    Skill {
        #[command(subcommand)]
        command: cli_skill::SkillCommands,
    },
    Reap {
        #[command(subcommand)]
        command: cli_reap::ReapCommands,
    },
    Repo {
        #[command(subcommand)]
        command: cli_repo::RepoCommands,
    },
    Domain {
        #[command(subcommand)]
        command: cli_domain::DomainCommands,
    },
    Alert {
        #[command(subcommand)]
        command: cli_ops::AlertCommands,
    },
    Api,
    #[command(name = "claude")]
    Claude {
        name: String,
        #[arg(long)] parent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
    #[command(name = "copilot")]
    Copilot {
        name: String,
        #[arg(long)] parent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
    /// Ask an agent a question using alias resolution
    #[command(name = "ask")]
    Ask {
        /// Agent alias or full name
        alias: String,
        /// Message to send
        message: Option<String>,
        #[arg(long)] list: bool,
        #[arg(long)] set: Option<String>,
        #[arg(long)] agent: Option<String>,
        #[arg(long, default_value = "http://localhost:8420")] api_url: String,
    },
}
