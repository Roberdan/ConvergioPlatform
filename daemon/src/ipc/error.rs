use thiserror::Error;

/// Typed error for all IPC operations, replacing stringly-typed results.
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("channel error: {0}")]
    Channel(String),

    #[error("engine dispatch error: {0}")]
    EngineDispatch(String),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("skill error: {0}")]
    Skill(String),

    #[error("lock error: {0}")]
    Lock(String),

    #[error("worktree error: {0}")]
    Worktree(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("http error: {0}")]
    Http(String),

    #[error("timeout")]
    Timeout,

    #[error("{0}")]
    Other(String),
}

impl From<String> for IpcError {
    fn from(s: String) -> Self {
        IpcError::Other(s)
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(e: serde_json::Error) -> Self {
        IpcError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::encode::Error> for IpcError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        IpcError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for IpcError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        IpcError::Serialization(e.to_string())
    }
}
