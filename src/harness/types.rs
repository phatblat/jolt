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

/// Paginated response wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageResponse<T> {
    pub content: Vec<T>,
    #[serde(rename = "totalElements")]
    pub total_elements: Option<i64>,
    #[serde(rename = "totalPages")]
    pub total_pages: Option<i64>,
}

/// Runner in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Runner {
    pub identifier: String,
    pub name: String,
    pub status: RunnerStatus,
    #[serde(rename = "lastHeartbeat")]
    pub last_heartbeat: Option<i64>,
    #[serde(rename = "ipAddress")]
    pub ip_address: Option<String>,
    pub capacity: Option<i32>,
    #[serde(rename = "runningBuilds")]
    pub running_builds: Option<i32>,
}

/// Pipeline in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pipeline {
    pub identifier: String,
    pub name: String,
    #[serde(rename = "projectIdentifier")]
    pub project_identifier: String,
    #[serde(rename = "orgIdentifier")]
    pub org_identifier: String,
}

/// Pipeline execution summary.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Execution {
    #[serde(rename = "planExecutionId")]
    pub plan_execution_id: String,
    #[serde(rename = "pipelineIdentifier")]
    pub pipeline_identifier: String,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
    pub name: Option<String>,
}

/// Pipeline execution details with stages and steps.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionDetails {
    #[serde(rename = "planExecutionId")]
    pub plan_execution_id: String,
    #[serde(rename = "pipelineIdentifier")]
    pub pipeline_identifier: String,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
    #[serde(rename = "stageExecutions")]
    pub stage_executions: Vec<StageExecution>,
}

/// Stage execution within a pipeline execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageExecution {
    #[serde(rename = "stageIdentifier")]
    pub stage_identifier: String,
    #[serde(rename = "stageName")]
    pub stage_name: Option<String>,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
    #[serde(rename = "stepExecutions")]
    pub step_executions: Option<Vec<StepExecution>>,
}

/// Step execution within a stage.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepExecution {
    #[serde(rename = "stepIdentifier")]
    pub step_identifier: String,
    #[serde(rename = "stepName")]
    pub step_name: Option<String>,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
}

/// Log line from execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogLine {
    pub level: Option<String>,
    pub time: Option<String>,
    pub message: String,
    pub pos: Option<i64>,
}

/// Log response with pagination.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogResponse {
    #[serde(rename = "logLines")]
    pub log_lines: Vec<LogLine>,
    pub more: Option<bool>,
    #[serde(rename = "nextToken")]
    pub next_token: Option<String>,
}

/// Execution filter for listing executions.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExecutionFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<ExecutionStatus>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "pipelineIdentifiers"
    )]
    pub pipeline_ids: Option<Vec<String>>,
}
