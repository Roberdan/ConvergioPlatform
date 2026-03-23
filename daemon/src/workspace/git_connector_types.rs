// Why: Plan 706 T3-01 — split from git_connector.rs to stay under 250 lines.
// Contains error types, data structs, and async return-type alias.
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

/// Structured error type for all GitHub API operations.
/// Replaces ad-hoc `String` errors for typed error handling downstream.
#[derive(Debug, Error)]
pub enum GitError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("GitHub API error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("Response parse failed: {0}")]
    Parse(String),
    #[error("Git command error: {0}")]
    Git(String),
}

/// Alias for boxed async futures in trait — enables dyn GitConnector without async_trait.
pub type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = Result<T, GitError>> + Send + 'a>>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MergeMethod {
    #[default]
    Squash,
    Merge,
    Rebase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrReadiness {
    pub mergeable: bool,
    pub ci_passed: bool,
    pub pending_checks: i64,
    pub unresolved_threads: i64,
    pub review_status: String,
}
