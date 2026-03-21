use super::core::IpcEngine;
use super::super::protocol::{IpcRequest, IpcResponse};

impl IpcEngine {
    pub async fn dispatch(&self, req: IpcRequest) -> rusqlite::Result<IpcResponse> {
        match req {
            IpcRequest::Register {
                name,
                agent_type,
                pid,
                host,
                metadata,
            } => self.register(&name, &agent_type, pid, &host, metadata.as_deref()),
            IpcRequest::Unregister { name, host } => self.unregister(&name, &host),
            IpcRequest::Who => self.who(),
            IpcRequest::Prune => self.prune(),
            IpcRequest::Send {
                from,
                to,
                content,
                msg_type,
                priority,
            } => self.send_message(&from, &to, &content, &msg_type, priority),
            IpcRequest::Broadcast {
                from,
                content,
                msg_type,
                channel,
            } => self.broadcast(&from, &content, &msg_type, channel.as_deref()),
            IpcRequest::Receive {
                agent,
                from_filter,
                channel_filter,
                limit,
                peek,
                wait,
            } => {
                if wait {
                    self.receive_wait(
                        &agent,
                        from_filter.as_deref(),
                        channel_filter.as_deref(),
                        limit,
                        30,
                    )
                    .await
                } else {
                    self.receive(
                        &agent,
                        from_filter.as_deref(),
                        channel_filter.as_deref(),
                        limit,
                        peek,
                    )
                }
            }
            IpcRequest::ChannelCreate {
                name,
                description,
                created_by,
            } => self.channel_create(&name, description.as_deref(), &created_by),
            IpcRequest::ChannelList => self.channel_list(),
            IpcRequest::ContextGet { key } => self.context_get(&key),
            IpcRequest::ContextSet { key, value, set_by } => {
                self.context_set(&key, &value, &set_by)
            }
            IpcRequest::ContextList => self.context_list(),
            IpcRequest::ContextDelete { key } => self.context_delete(&key),
            IpcRequest::History {
                agent,
                channel,
                limit,
                since,
            } => self.history(
                agent.as_deref(),
                channel.as_deref(),
                limit,
                since.as_deref(),
            ),
            IpcRequest::DbStats => self.db_stats(),
            IpcRequest::DbCleanup { older_than_days } => self.db_cleanup(older_than_days),
            IpcRequest::DbVacuum => self.db_vacuum(),
            IpcRequest::DbReset => self.db_reset(),
            IpcRequest::Ping => Ok(self.ping()),
            IpcRequest::Status => self.status(),
        }
    }
}
