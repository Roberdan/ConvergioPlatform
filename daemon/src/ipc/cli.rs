use std::path::PathBuf;

use clap::{Parser, Subcommand};

use super::client::ipc_request_with_fallback;
use super::protocol::{IpcRequest, IpcResponse};

fn default_socket_path() -> PathBuf {
    dirs_next().join("data/ipc.sock")
}

fn default_db_path() -> PathBuf {
    dirs_next().join("dashboard.db")
}

fn dirs_next() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".claude")
}

fn agent_name_from_env() -> Option<String> {
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

// ── Colored output ───────────────────────────────────

fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err()
}

fn green(s: &str) -> String {
    if use_color() {
        format!("\x1b[32m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn cyan(s: &str) -> String {
    if use_color() {
        format!("\x1b[36m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn yellow(s: &str) -> String {
    if use_color() {
        format!("\x1b[33m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn red(s: &str) -> String {
    if use_color() {
        format!("\x1b[31m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn dim(s: &str) -> String {
    if use_color() {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn format_response(resp: &IpcResponse, json: bool) -> String {
    if json {
        return serde_json::to_string_pretty(resp).unwrap_or_else(|_| format!("{resp:?}"));
    }

    match resp {
        IpcResponse::Ok { message } => format!("✓ {}", green(message)),
        IpcResponse::Error { code, message } => format!("{} [{}]", red(message), code),
        IpcResponse::AgentList { agents } => {
            if agents.is_empty() {
                return dim("(no agents registered)").to_string();
            }
            agents
                .iter()
                .map(|a| {
                    format!(
                        "  {} {} {} {}",
                        green(&a.name),
                        dim(&format!("@{}", a.host)),
                        cyan(&a.agent_type),
                        yellow(&a.last_seen),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        IpcResponse::MessageList { messages } => {
            if messages.is_empty() {
                return dim("(no messages)").to_string();
            }
            messages
                .iter()
                .map(|m| {
                    let to = m.to_agent.as_deref().unwrap_or("*");
                    format!(
                        "  {} {} → {} {}  {}",
                        yellow(&m.created_at),
                        green(&m.from_agent),
                        cyan(to),
                        dim(&format!("[{}]", m.msg_type)),
                        m.content,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        IpcResponse::ChannelList { channels } => {
            if channels.is_empty() {
                return dim("(no channels)").to_string();
            }
            channels
                .iter()
                .map(|c| {
                    let desc = c.description.as_deref().unwrap_or("");
                    format!(
                        "  {} {} {}",
                        cyan(&c.name),
                        dim(desc),
                        dim(&format!("by {}", c.created_by))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        IpcResponse::ContextList { entries } => {
            if entries.is_empty() {
                return dim("(no context entries)").to_string();
            }
            entries
                .iter()
                .map(|e| {
                    format!(
                        "  {} = {} {}",
                        green(&e.key),
                        e.value,
                        dim(&format!("v{} by {}", e.version, e.set_by))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        IpcResponse::Context {
            key,
            value,
            version,
            set_by,
            ..
        } => {
            format!(
                "{} = {} {}",
                green(key),
                value,
                dim(&format!("v{version} by {set_by}"))
            )
        }
        IpcResponse::Stats {
            agents,
            messages,
            channels,
            context_keys,
            db_size_bytes,
        } => {
            format!(
                "agents: {}  messages: {}  channels: {}  context: {}  db: {}KB",
                agents,
                messages,
                channels,
                context_keys,
                db_size_bytes / 1024
            )
        }
        IpcResponse::Pong { uptime_secs } => format!("pong (uptime: {}s)", uptime_secs),
        _ => format!("{resp:?}"),
    }
}

// ── Run CLI ──────────────────────────────────────────

pub async fn run_ipc(args: IpcArgs) -> Result<(), String> {
    let socket = args.socket.unwrap_or_else(default_socket_path);
    let db = args.db.unwrap_or_else(default_db_path);
    let json = args.json;

    let agent_name = || -> String {
        agent_name_from_env().unwrap_or_else(|| format!("cli-{}", std::process::id()))
    };

    let hostname = || -> String { super::engine::IpcEngine::hostname() };

    let request = match args.command {
        IpcSubcommand::Register {
            name,
            agent_type,
            pid,
            host,
            metadata,
        } => IpcRequest::Register {
            name: name.or_else(agent_name_from_env).unwrap_or_else(agent_name),
            agent_type,
            pid: pid.or_else(|| Some(std::process::id())),
            host: host.unwrap_or_else(hostname),
            metadata,
        },
        IpcSubcommand::Unregister { name, host } => IpcRequest::Unregister {
            name: name.unwrap_or_else(agent_name),
            host: host.unwrap_or_else(hostname),
        },
        IpcSubcommand::Who => IpcRequest::Who,
        IpcSubcommand::Prune => IpcRequest::Prune,
        IpcSubcommand::Send {
            to,
            message,
            msg_type,
            priority,
            from,
        } => IpcRequest::Send {
            from: from.unwrap_or_else(agent_name),
            to,
            content: message,
            msg_type,
            priority,
        },
        IpcSubcommand::Broadcast {
            message,
            msg_type,
            channel,
            from,
        } => IpcRequest::Broadcast {
            from: from.unwrap_or_else(agent_name),
            content: message,
            msg_type,
            channel,
        },
        IpcSubcommand::Recv {
            from,
            channel,
            limit,
            peek,
            wait,
            agent,
        } => IpcRequest::Receive {
            agent: agent.unwrap_or_else(agent_name),
            from_filter: from,
            channel_filter: channel,
            limit,
            peek,
            wait,
        },
        IpcSubcommand::Channel(ChannelSub::Create {
            name,
            description,
            created_by,
        }) => IpcRequest::ChannelCreate {
            name,
            description,
            created_by: created_by.unwrap_or_else(agent_name),
        },
        IpcSubcommand::Channel(ChannelSub::List) => IpcRequest::ChannelList,
        IpcSubcommand::Ctx(CtxSub::Get { key }) => IpcRequest::ContextGet { key },
        IpcSubcommand::Ctx(CtxSub::Set { key, value, set_by }) => IpcRequest::ContextSet {
            key,
            value,
            set_by: set_by.unwrap_or_else(agent_name),
        },
        IpcSubcommand::Ctx(CtxSub::List) => IpcRequest::ContextList,
        IpcSubcommand::Ctx(CtxSub::Delete { key }) => IpcRequest::ContextDelete { key },
        IpcSubcommand::Status => IpcRequest::Status,
        IpcSubcommand::Ping => IpcRequest::Ping,
        IpcSubcommand::History {
            agent,
            channel,
            limit,
            since,
        } => IpcRequest::History {
            agent,
            channel,
            limit,
            since,
        },
        IpcSubcommand::Db(DbSub::Stats) => IpcRequest::DbStats,
        IpcSubcommand::Db(DbSub::Cleanup { older_than_days }) => {
            IpcRequest::DbCleanup { older_than_days }
        }
        IpcSubcommand::Db(DbSub::Vacuum) => IpcRequest::DbVacuum,
        IpcSubcommand::Db(DbSub::Reset) => IpcRequest::DbReset,
    };

    let response = ipc_request_with_fallback(&socket, &db, &request).await?;
    println!("{}", format_response(&response, json));
    Ok(())
}
