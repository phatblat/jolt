// Harness API error types.
// Handles Harness-specific API errors, authentication, and response parsing.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("API returned error: {code} - {message}")]
    Api { code: String, message: String },

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid response format: {0}")]
    ParseError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, HarnessError>;
