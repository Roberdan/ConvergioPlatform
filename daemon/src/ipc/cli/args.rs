use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub(super) fn default_socket_path() -> PathBuf {
    dirs_next().join("data/ipc.sock")
}

pub(super) fn default_db_path() -> PathBuf {
    dirs_next().join("dashboard.db")
}

fn dirs_next() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".claude")
}

pub fn agent_name_from_env() -> Option<String> {
    std::env::var("AGENT_IPC_NAME").ok()
}

#[derive(Debug, Parser)]
pub struct IpcArgs {
    #[arg(long, default_value = "false")]
    pub json: bool,

    #[arg(long)]
    pub socket: Option<PathBuf>,

    #[arg(long)]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: IpcSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum IpcSubcommand {
    Register {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short = 't', long, default_value = "claude")]
        agent_type: String,
        #[arg(long)]
        pid: Option<u32>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        metadata: Option<String>,
    },
    Unregister {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(long)]
        host: Option<String>,
    },
    Who,
    Prune,
    Send {
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        message: String,
        #[arg(long, default_value = "text")]
        msg_type: String,
        #[arg(long, default_value_t = 0)]
        priority: i32,
        #[arg(long)]
        from: Option<String>,
    },
    Broadcast {
        #[arg(short, long)]
        message: String,
        #[arg(long, default_value = "text")]
        msg_type: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        from: Option<String>,
    },
    Recv {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        peek: bool,
        #[arg(long)]
        wait: bool,
        #[arg(short, long)]
        agent: Option<String>,
    },
    #[command(subcommand)]
    Channel(ChannelSub),
    #[command(subcommand)]
    Ctx(CtxSub),
    Status,
    Ping,
    History {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long)]
        since: Option<String>,
    },
    #[command(subcommand)]
    Db(DbSub),
}

#[derive(Debug, Subcommand)]
pub enum ChannelSub {
    Create {
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        created_by: Option<String>,
    },
    List,
}

#[derive(Debug, Subcommand)]
pub enum CtxSub {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
        #[arg(long)]
        set_by: Option<String>,
    },
    List,
    Delete {
        key: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum DbSub {
    Stats,
    Cleanup {
        #[arg(long, default_value_t = 30)]
        older_than_days: u32,
    },
    Vacuum,
    Reset,
}
