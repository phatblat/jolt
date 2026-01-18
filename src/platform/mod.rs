// Platform abstraction module.
// Defines trait for CI/CD platform operations and implementations.

#![allow(dead_code)]

use async_trait::async_trait;

use crate::types::*;

pub mod github;
pub mod harness;

/// Result type for platform operations.
pub type Result<T> = std::result::Result<T, PlatformError>;

/// Unified error type for platform operations.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("GitHub error: {0}")]
    GitHub(#[from] crate::error::JoltError),

    #[error("Harness error: {0}")]
    Harness(#[from] crate::harness::HarnessError),

    #[error("Platform not authenticated")]
    NotAuthenticated,

    #[error("Operation not supported by platform: {0}")]
    NotSupported(String),

    #[error("{0}")]
    Other(String),
}

/// Trait defining CI/CD platform operations.
///
/// This trait provides a unified interface for interacting with different
/// CI/CD platforms (GitHub Actions, Harness, etc.) using common types.
#[async_trait]
pub trait Platform: Send + Sync {
    /// Get the platform identifier.
    fn platform_type(&self) -> crate::types::Platform;

    /// Check if the platform is authenticated.
    fn is_authenticated(&self) -> bool;

    // ========================================
    // Organizations
    // ========================================

    /// List all organizations/owners accessible to the authenticated user.
    async fn list_organizations(&mut self) -> Result<Vec<Organization>>;

    /// Get details for a specific organization.
    async fn get_organization(&mut self, org_id: &str) -> Result<Organization>;

    // ========================================
    // Projects
    // ========================================

    /// List projects/repositories in an organization.
    async fn list_projects(&mut self, org_id: &str) -> Result<Vec<Project>>;

    /// Get details for a specific project.
    async fn get_project(&mut self, org_id: &str, project_id: &str) -> Result<Project>;

    // ========================================
    // Workflows
    // ========================================

    /// List workflows/pipelines in a project.
    async fn list_workflows(&mut self, project_id: &str) -> Result<Vec<Workflow>>;

    /// Get details for a specific workflow.
    async fn get_workflow(&mut self, workflow_id: &str) -> Result<Workflow>;

    // ========================================
    // Executions
    // ========================================

    /// List executions/runs for a workflow.
    async fn list_executions(&mut self, workflow_id: &str) -> Result<Vec<Execution>>;

    /// Get details for a specific execution.
    async fn get_execution(&mut self, execution_id: &str) -> Result<Execution>;

    // ========================================
    // Jobs
    // ========================================

    /// List jobs/stages for an execution.
    async fn list_jobs(&mut self, execution_id: &str) -> Result<Vec<Job>>;

    /// Get details for a specific job.
    async fn get_job(&mut self, job_id: &str) -> Result<Job>;

    // ========================================
    // Steps
    // ========================================

    /// List steps for a job.
    async fn list_steps(&mut self, job_id: &str) -> Result<Vec<Step>>;

    /// Get details for a specific step.
    async fn get_step(&mut self, step_id: &str) -> Result<Step>;

    // ========================================
    // Logs
    // ========================================

    /// Fetch logs for a step.
    async fn fetch_logs(&mut self, step_id: &str) -> Result<Vec<LogLine>>;

    // ========================================
    // Runners
    // ========================================

    /// List runners with optional scope filtering.
    async fn list_runners(&mut self, scope: Option<&str>) -> Result<Vec<Runner>>;

    /// Get details for a specific runner.
    async fn get_runner(&mut self, runner_id: &str) -> Result<Runner>;
}
