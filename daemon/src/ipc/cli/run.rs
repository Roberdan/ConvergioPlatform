use super::args::{
    agent_name_from_env, default_db_path, default_socket_path, ChannelSub, CtxSub, DbSub,
    IpcArgs, IpcSubcommand,
};
use super::output::format_response;
use super::super::client::ipc_request_with_fallback;
use super::super::engine::core::IpcEngine;
use super::super::protocol::IpcRequest;

pub async fn run_ipc(args: IpcArgs) -> Result<(), String> {
    let socket = args.socket.unwrap_or_else(default_socket_path);
    let db = args.db.unwrap_or_else(default_db_path);
    let json = args.json;

    let agent_name = || -> String {
        agent_name_from_env().unwrap_or_else(|| format!("cli-{}", std::process::id()))
    };

    let hostname = || -> String { IpcEngine::hostname() };

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
