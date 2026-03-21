use super::super::protocol::IpcResponse;

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
