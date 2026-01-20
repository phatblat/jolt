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

/// Grouped job with previous attempts.
/// Used for displaying job history with latest attempt and previous re-runs.
#[derive(Debug, Clone)]
pub struct JobGroup {
    /// Latest attempt (most recent)
    pub latest: Job,
    /// Previous attempts in reverse chronological order
    pub previous: Vec<Job>,
}

impl JobGroup {
    /// Create a new job group from a list of jobs with the same name.
    pub fn from_jobs(mut jobs: Vec<Job>) -> Self {
        // Sort by started_at (or completed_at if no start) in reverse chronological order
        jobs.sort_by(|a, b| {
            let a_time = a
                .started_at
                .or(a.completed_at)
                .unwrap_or_else(chrono::Utc::now);
            let b_time = b
                .started_at
                .or(b.completed_at)
                .unwrap_or_else(chrono::Utc::now);
            b_time.cmp(&a_time)
        });

        let mut iter = jobs.into_iter();
        let latest = iter.next().expect("JobGroup requires at least one job");
        let previous: Vec<Job> = iter.collect();

        Self { latest, previous }
    }

    /// Total number of attempts (including latest)
    pub fn total_attempts(&self) -> usize {
        1 + self.previous.len()
    }

    /// Group a list of jobs by name, returning job groups in alphabetical order by name.
    pub fn group_by_name(jobs: Vec<Job>) -> Vec<JobGroup> {
        use std::collections::HashMap;

        let mut groups: HashMap<String, Vec<Job>> = HashMap::new();
        for job in jobs {
            groups.entry(job.name.clone()).or_default().push(job);
        }

        let mut result: Vec<JobGroup> = groups
            .into_iter()
            .map(|(_, jobs)| JobGroup::from_jobs(jobs))
            .collect();

        // Sort by job name
        result.sort_by(|a, b| a.latest.name.cmp(&b.latest.name));
        result
    }
}

/// Flattened job list item for display.
/// Represents either a main job or a previous attempt sub-item.
#[derive(Debug, Clone)]
pub enum JobListItem {
    /// Main job entry (latest attempt)
    Main { group_index: usize },
    /// Previous attempt sub-item
    SubItem {
        group_index: usize,
        attempt_index: usize,
    },
}

impl JobListItem {
    /// Create a flattened list from job groups for rendering.
    pub fn flatten(groups: &[JobGroup]) -> Vec<JobListItem> {
        let mut items = Vec::new();
        for (group_idx, group) in groups.iter().enumerate() {
            // Add main item
            items.push(JobListItem::Main {
                group_index: group_idx,
            });
            // Add sub-items for previous attempts
            for attempt_idx in 0..group.previous.len() {
                items.push(JobListItem::SubItem {
                    group_index: group_idx,
                    attempt_index: attempt_idx,
                });
            }
        }
        items
    }

    /// Get the job for this list item.
    pub fn get_job<'a>(&self, groups: &'a [JobGroup]) -> &'a Job {
        match self {
            JobListItem::Main { group_index } => &groups[*group_index].latest,
            JobListItem::SubItem {
                group_index,
                attempt_index,
            } => &groups[*group_index].previous[*attempt_index],
        }
    }
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
