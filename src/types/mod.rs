// Unified types module.
// Platform-agnostic types that work for both GitHub Actions and Harness CI/CD.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Platform identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    GitHub,
    Harness,
}

impl Platform {
    /// Get a short display string for the platform.
    pub fn short_name(&self) -> &'static str {
        match self {
            Platform::GitHub => "GH",
            Platform::Harness => "HR",
        }
    }

    /// Get the full name of the platform.
    pub fn full_name(&self) -> &'static str {
        match self {
            Platform::GitHub => "GitHub Actions",
            Platform::Harness => "Harness CI/CD",
        }
    }
}

/// Organization or Owner (top-level grouping).
/// Maps to GitHub Owner (user/org) or Harness Organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub platform: Platform,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_type: Option<OrgType>,
}

/// Type of organization (GitHub-specific).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrgType {
    User,
    Organization,
}

/// Project or Repository.
/// Maps to GitHub Repository or Harness Project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub display_name: String, // "owner/repo" for GitHub, "org/project" for Harness
    pub platform: Platform,
    pub org_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the project is private (true = private, false = public).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<bool>,
    /// When the project was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Workflow or Pipeline.
/// Maps to GitHub Workflow or Harness Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Execution or Run.
/// Maps to GitHub Workflow Run or Harness Pipeline Execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub id: String,
    pub number: i64,
    pub platform: Platform,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// Job or Stage.
/// Maps to GitHub Job or Harness Stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub execution_id: String,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// Step (same concept in both platforms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub job_id: String,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// Unified execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
    Paused, // Harness-only
}

impl ExecutionStatus {
    /// Get a color for the status.
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            ExecutionStatus::Queued => Color::Blue,
            ExecutionStatus::Running => Color::Yellow,
            ExecutionStatus::Success => Color::Green,
            ExecutionStatus::Failed => Color::Red,
            ExecutionStatus::Cancelled => Color::Gray,
            ExecutionStatus::Paused => Color::Magenta,
        }
    }
}

/// Runner (same concept in both platforms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub status: RunnerStatus,
    pub scope: RunnerScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_job: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

/// Unified runner status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunnerStatus {
    Online,
    Offline,
    Busy,
    Unhealthy,
}

impl RunnerStatus {
    /// Get a color for the status.
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            RunnerStatus::Online => Color::Green,
            RunnerStatus::Offline => Color::Gray,
            RunnerStatus::Busy => Color::Yellow,
            RunnerStatus::Unhealthy => Color::Red,
        }
    }
}

/// Runner scope (where the runner is defined).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunnerScope {
    Repository { org: String, repo: String },
    Organization { org: String },
    Project { org: String, project: String },
    Enterprise { enterprise: String },
}

impl RunnerScope {
    /// Get a display string for the scope.
    pub fn display(&self) -> String {
        match self {
            RunnerScope::Repository { org, repo } => format!("{}/{}", org, repo),
            RunnerScope::Organization { org } => org.clone(),
            RunnerScope::Project { org, project } => format!("{}/{}", org, project),
            RunnerScope::Enterprise { enterprise } => enterprise.clone(),
        }
    }
}

/// Log line (unified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: Option<String>,
    pub message: String,
}

// ========================================
// Type Conversions: GitHub → Unified
// ========================================

impl Runner {
    /// Create a unified Runner from a GitHub EnrichedRunner.
    pub fn from_github_enriched(
        enriched: &crate::github::EnrichedRunner,
        owner: &str,
        repo: &str,
    ) -> Self {
        let runner = &enriched.runner;

        // Determine status: busy takes precedence
        let status = if runner.busy {
            RunnerStatus::Busy
        } else {
            match runner.status {
                crate::github::RunnerStatus::Online => RunnerStatus::Online,
                crate::github::RunnerStatus::Offline => RunnerStatus::Offline,
                crate::github::RunnerStatus::Unknown => RunnerStatus::Offline,
            }
        };

        // Build current job description if busy
        let current_job = if runner.busy {
            enriched.current_job.as_ref().map(|job| {
                let mut parts = Vec::new();
                if let Some(pr) = job.pr_number {
                    parts.push(format!("PR #{}", pr));
                }
                if let Some(branch) = &job.branch {
                    let branch_display = if branch.len() > 20 {
                        format!("{}...", &branch[..17])
                    } else {
                        branch.clone()
                    };
                    parts.push(branch_display);
                }
                if parts.is_empty() {
                    "Running".to_string()
                } else {
                    parts.join(" ")
                }
            })
        } else {
            None
        };

        // Extract label names
        let labels = if runner.labels.is_empty() {
            None
        } else {
            Some(runner.labels.iter().map(|l| l.name.clone()).collect())
        };

        Runner {
            id: runner.id.to_string(),
            name: runner.name.clone(),
            platform: Platform::GitHub,
            status,
            scope: RunnerScope::Repository {
                org: owner.to_string(),
                repo: repo.to_string(),
            },
            current_job,
            labels,
            os: Some(runner.os.clone()),
        }
    }
}
