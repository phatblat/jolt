// GitHub API response types.
// Defines structs for deserializing GitHub REST API responses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Owner type discriminator (user or organization).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OwnerType {
    User,
    #[default]
    Organization,
    Bot,
    #[serde(other)]
    Unknown,
}

/// GitHub user or organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub id: u64,
    pub login: String,
    #[serde(rename = "type", default)]
    pub owner_type: OwnerType,
    pub avatar_url: Option<String>,
}

/// GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: Owner,
    pub private: bool,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: Option<DateTime<Utc>>,
}

/// GitHub Actions workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: WorkflowState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    Active,
    Deleted,
    DisabledFork,
    DisabledInactivity,
    DisabledManually,
    #[serde(other)]
    Unknown,
}

/// GitHub Actions workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: Option<String>,
    pub run_number: u64,
    pub run_attempt: Option<u64>,
    pub status: RunStatus,
    pub conclusion: Option<RunConclusion>,
    pub workflow_id: u64,
    pub head_branch: Option<String>,
    pub head_sha: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub html_url: String,
    #[serde(default)]
    pub pull_requests: Vec<PullRequestRef>,
}

/// Workflow run status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    InProgress,
    Completed,
    Waiting,
    Requested,
    Pending,
    #[serde(other)]
    Unknown,
}

/// Workflow run conclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunConclusion {
    Success,
    Failure,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    Neutral,
    Stale,
    StartupFailure,
    #[serde(other)]
    Unknown,
}

/// Reference to a pull request in a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestRef {
    pub number: u64,
    pub head: GitRef,
    pub base: GitRef,
}

/// Git reference (branch/commit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// GitHub Actions job within a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub run_id: u64,
    pub name: String,
    pub status: RunStatus,
    pub conclusion: Option<RunConclusion>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub html_url: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    pub runner_name: Option<String>,
}

/// Step within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    pub status: RunStatus,
    pub conclusion: Option<RunConclusion>,
    pub number: u64,
}

/// Self-hosted runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: u64,
    pub name: String,
    pub os: String,
    pub status: RunnerStatus,
    pub busy: bool,
    #[serde(default)]
    pub labels: Vec<RunnerLabel>,
}

/// Enriched runner info with current job details.
#[derive(Debug, Clone)]
pub struct EnrichedRunner {
    pub runner: Runner,
    pub current_job: Option<RunnerJobInfo>,
}

/// Current job information for a busy runner.
#[derive(Debug, Clone)]
pub struct RunnerJobInfo {
    pub pr_number: Option<u64>,
    pub branch: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub job_name: String,
}

/// Runner status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerStatus {
    Online,
    Offline,
    #[serde(other)]
    Unknown,
}

/// Runner label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerLabel {
    pub id: Option<u64>,
    pub name: String,
    #[serde(rename = "type")]
    pub label_type: Option<String>,
}

/// Paginated list response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub total_count: u64,
    #[serde(
        alias = "workflows",
        alias = "workflow_runs",
        alias = "jobs",
        alias = "runners"
    )]
    pub items: Vec<T>,
}

/// Rate limit information from response headers.
#[derive(Debug, Clone, Default)]
pub struct RateLimit {
    pub limit: u64,
    pub remaining: u64,
    pub reset: u64,
}
