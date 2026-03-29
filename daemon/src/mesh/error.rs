// General mesh error type for modules that lack a domain-specific thiserror enum.
// Covers: auth, config, db, io, network, serialization, and internal errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("database error: {0}")]
    Db(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for MeshError {
    fn from(e: std::io::Error) -> Self {
        MeshError::Io(e.to_string())
    }
}

impl From<rusqlite::Error> for MeshError {
    fn from(e: rusqlite::Error) -> Self {
        MeshError::Db(e.to_string())
    }
}

impl From<serde_json::Error> for MeshError {
    fn from(e: serde_json::Error) -> Self {
        MeshError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::encode::Error> for MeshError {
    fn from(e: rmp_serde::encode::Error) -> Self {
        MeshError::Serialization(e.to_string())
    }
}

impl From<rmp_serde::decode::Error> for MeshError {
    fn from(e: rmp_serde::decode::Error) -> Self {
        MeshError::Serialization(e.to_string())
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
