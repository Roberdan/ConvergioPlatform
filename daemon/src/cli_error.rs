// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Structured error type for CLI subcommands — replaces process::exit calls.

#[derive(Debug, thiserror::Error)]
pub(crate) enum CliError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    ApiCallFailed(String),
    #[error("{0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl CliError {
    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            CliError::NotFound(_) => 1,
            _ => 2,
        }
    }
}
