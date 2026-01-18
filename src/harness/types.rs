// Harness API response types.
// Defines structures for deserializing Harness API responses.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Standard Harness API response envelope.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: Option<T>,
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
}

/// Error response from Harness API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub status: String,
    pub code: String,
    pub message: String,
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
}

/// Organization in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Organization {
    pub identifier: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Project in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Project {
    pub identifier: String,
    pub name: String,
    #[serde(rename = "orgIdentifier")]
    pub org_identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Runner status in Harness.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RunnerStatus {
    Active,
    Inactive,
    Unhealthy,
    Connected,
    Disconnected,
}

/// Pipeline execution status in Harness.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Aborted,
    Expired,
    Queued,
    Paused,
}
