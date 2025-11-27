// Error types for jolt application.
// Handles GitHub API errors, cache errors, and general application errors.

#![allow(dead_code)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum JoltError {
    #[error("GitHub API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Authentication failed: invalid or expired token")]
    Unauthorized,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded, resets at {reset_at}")]
    RateLimited { reset_at: String },

    #[error("Missing GITHUB_TOKEN environment variable")]
    MissingToken,

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, JoltError>;
