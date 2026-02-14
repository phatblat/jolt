// Harness CI/CD platform adapter.
// Implements the Platform trait for Harness.

use async_trait::async_trait;

use crate::harness::{
    ExecutionStatus as HarnessStatus, HarnessClient, RunnerStatus as HarnessRunnerStatus,
};
use crate::platform::{Platform as PlatformTrait, PlatformError, Result};
use crate::types::*;

/// Harness CI/CD platform implementation.
pub struct HarnessPlatform {
    client: HarnessClient,
    // Cache for storing context
    current_org: Option<String>,
    current_project: Option<String>,
}

impl HarnessPlatform {
    /// Create a new Harness platform with the given credentials.
    pub fn new(base_url: String, api_key: String, account_id: String) -> Result<Self> {
        let client = HarnessClient::new(base_url, api_key, account_id)?;
        Ok(Self {
            client,
            current_org: None,
            current_project: None,
        })
    }

    /// Create a Harness platform from environment variables.
    pub fn from_env() -> Result<Self> {
        let client = HarnessClient::from_env()?;
        Ok(Self {
            client,
            current_org: None,
            current_project: None,
        })
    }

    /// Set the current organization context.
    pub fn set_organization(&mut self, org_id: String) {
        self.current_org = Some(org_id);
    }

    /// Set the current project context.
    pub fn set_project(&mut self, project_id: String) {
        self.current_project = Some(project_id);
    }
}

#[async_trait]
impl PlatformTrait for HarnessPlatform {
    fn platform_type(&self) -> Platform {
        Platform::Harness
    }

    fn is_authenticated(&self) -> bool {
        true // If we have a client, we're authenticated
    }

    async fn list_organizations(&mut self) -> Result<Vec<Organization>> {
        let orgs = self.client.list_organizations().await?;
        Ok(orgs.iter().map(map_organization).collect())
    }

    async fn get_organization(&mut self, org_id: &str) -> Result<Organization> {
        let org = self.client.get_organization(org_id).await?;
        Ok(map_organization(&org))
    }

    async fn list_projects(&mut self, org_id: &str) -> Result<Vec<Project>> {
        let projects = self.client.list_projects(org_id).await?;
        Ok(projects.iter().map(map_project).collect())
    }

    async fn get_project(&mut self, org_id: &str, project_id: &str) -> Result<Project> {
        let project = self.client.get_project(org_id, project_id).await?;
        Ok(map_project(&project))
    }

    async fn list_workflows(&mut self, _project_id: &str) -> Result<Vec<Workflow>> {
        // Need org and project context
        if let (Some(org), Some(project)) = (&self.current_org, &self.current_project) {
            let pipelines = self.client.list_pipelines(org, project).await?;
            Ok(pipelines.iter().map(map_pipeline).collect())
        } else {
            Err(PlatformError::Other(
                "Organization and project context required".to_string(),
            ))
        }
    }

    async fn get_workflow(&mut self, _workflow_id: &str) -> Result<Workflow> {
        Err(PlatformError::NotSupported(
            "Harness workflow lookup not yet implemented".to_string(),
        ))
    }

    async fn list_executions(&mut self, _workflow_id: &str) -> Result<Vec<Execution>> {
        // Need org and project context
        if let (Some(org), Some(project)) = (&self.current_org, &self.current_project) {
            let executions = self.client.list_executions(org, project, None).await?;
            Ok(executions.iter().map(map_execution).collect())
        } else {
            Err(PlatformError::Other(
                "Organization and project context required".to_string(),
            ))
        }
    }

    async fn get_execution(&mut self, execution_id: &str) -> Result<Execution> {
        let details = self.client.get_execution(execution_id).await?;
        Ok(map_execution_details(&details))
    }

    async fn list_jobs(&mut self, execution_id: &str) -> Result<Vec<Job>> {
        let details = self.client.get_execution(execution_id).await?;
        Ok(details
            .stage_executions
            .iter()
            .map(|stage| map_stage_to_job(stage, execution_id))
            .collect())
    }

    async fn get_job(&mut self, _job_id: &str) -> Result<Job> {
        Err(PlatformError::NotSupported(
            "Harness job lookup not yet implemented".to_string(),
        ))
    }

    async fn list_steps(&mut self, _job_id: &str) -> Result<Vec<Step>> {
        // Would need to parse job_id to get execution and stage, then fetch steps
        Err(PlatformError::NotSupported(
            "Harness step listing not yet implemented".to_string(),
        ))
    }

    async fn get_step(&mut self, _step_id: &str) -> Result<Step> {
        Err(PlatformError::NotSupported(
            "Harness step lookup not yet implemented".to_string(),
        ))
    }

    async fn fetch_logs(&mut self, _step_id: &str) -> Result<Vec<LogLine>> {
        // Would need to parse step_id to build log key
        Ok(vec![])
    }

    async fn list_runners(&mut self, _scope: Option<&str>) -> Result<Vec<Runner>> {
        // List all runners (could filter by org/project if scope provided)
        let runners = self.client.list_runners(None, None).await?;
        Ok(runners.iter().map(map_runner).collect())
    }

    async fn get_runner(&mut self, runner_id: &str) -> Result<Runner> {
        let runner = self.client.get_runner(runner_id).await?;
        Ok(map_runner(&runner))
    }
}

// ========================================
// Type Mappers: Harness → Unified
// ========================================

/// Convert Harness Organization to unified Organization.
pub fn map_organization(org: &crate::harness::Organization) -> Organization {
    Organization {
        id: org.identifier.clone(),
        name: org.name.clone(),
        display_name: org.name.clone(),
        platform: Platform::Harness,
        description: org.description.clone(),
        org_type: None, // Harness doesn't distinguish user vs org
    }
}

/// Convert Harness Project to unified Project.
pub fn map_project(project: &crate::harness::Project) -> Project {
    Project {
        id: project.identifier.clone(),
        name: project.name.clone(),
        display_name: format!("{}/{}", project.org_identifier, project.identifier),
        platform: Platform::Harness,
        org_id: project.org_identifier.clone(),
        description: project.description.clone(),
        visibility: None,
        updated_at: None,
    }
}

/// Convert Harness Pipeline to unified Workflow.
pub fn map_pipeline(pipeline: &crate::harness::Pipeline) -> Workflow {
    Workflow {
        id: pipeline.identifier.clone(),
        name: pipeline.name.clone(),
        platform: Platform::Harness,
        project_id: pipeline.project_identifier.clone(),
        path: None,
    }
}

/// Convert Harness Execution to unified Execution.
pub fn map_execution(execution: &crate::harness::Execution) -> Execution {
    let duration_ms = execution
        .end_ts
        .map(|end| (end - execution.start_ts) * 1000);

    Execution {
        id: execution.plan_execution_id.clone(),
        number: 0, // Harness doesn't have run numbers
        platform: Platform::Harness,
        workflow_id: execution.pipeline_identifier.clone(),
        status: map_execution_status(&execution.status),
        started_at: execution.start_ts,
        ended_at: execution.end_ts,
        duration_ms,
    }
}

/// Convert Harness ExecutionDetails to unified Execution.
pub fn map_execution_details(details: &crate::harness::ExecutionDetails) -> Execution {
    let duration_ms = details.end_ts.map(|end| (end - details.start_ts) * 1000);

    Execution {
        id: details.plan_execution_id.clone(),
        number: 0,
        platform: Platform::Harness,
        workflow_id: details.pipeline_identifier.clone(),
        status: map_execution_status(&details.status),
        started_at: details.start_ts,
        ended_at: details.end_ts,
        duration_ms,
    }
}

/// Convert Harness StageExecution to unified Job.
pub fn map_stage_to_job(stage: &crate::harness::StageExecution, execution_id: &str) -> Job {
    let duration_ms = stage.end_ts.map(|end| (end - stage.start_ts) * 1000);

    Job {
        id: stage.stage_identifier.clone(),
        name: stage
            .stage_name
            .clone()
            .unwrap_or_else(|| stage.stage_identifier.clone()),
        platform: Platform::Harness,
        execution_id: execution_id.to_string(),
        status: map_execution_status(&stage.status),
        started_at: Some(stage.start_ts),
        ended_at: stage.end_ts,
        duration_ms,
    }
}

/// Convert Harness StepExecution to unified Step.
pub fn map_step(step: &crate::harness::StepExecution, job_id: &str) -> Step {
    let duration_ms = step.end_ts.map(|end| (end - step.start_ts) * 1000);

    Step {
        id: step.step_identifier.clone(),
        name: step
            .step_name
            .clone()
            .unwrap_or_else(|| step.step_identifier.clone()),
        platform: Platform::Harness,
        job_id: job_id.to_string(),
        status: map_execution_status(&step.status),
        started_at: Some(step.start_ts),
        ended_at: step.end_ts,
        duration_ms,
    }
}

/// Convert Harness Runner to unified Runner.
pub fn map_runner(runner: &crate::harness::Runner) -> Runner {
    Runner {
        id: runner.identifier.clone(),
        name: runner.name.clone(),
        platform: Platform::Harness,
        status: map_runner_status(&runner.status),
        scope: RunnerScope::Organization {
            org: "unknown".to_string(), // Would need context to determine actual scope
        },
        current_job: runner.running_builds.and_then(|count| {
            if count > 0 {
                Some(format!("{} running", count))
            } else {
                None
            }
        }),
        labels: None,
        os: None,
    }
}

/// Map Harness execution status to unified status.
pub fn map_execution_status(status: &HarnessStatus) -> ExecutionStatus {
    match status {
        HarnessStatus::Queued => ExecutionStatus::Queued,
        HarnessStatus::Running => ExecutionStatus::Running,
        HarnessStatus::Success => ExecutionStatus::Success,
        HarnessStatus::Failed => ExecutionStatus::Failed,
        HarnessStatus::Aborted => ExecutionStatus::Cancelled,
        HarnessStatus::Expired => ExecutionStatus::Failed,
        HarnessStatus::Paused => ExecutionStatus::Paused,
    }
}

/// Map Harness runner status to unified status.
pub fn map_runner_status(status: &HarnessRunnerStatus) -> RunnerStatus {
    match status {
        HarnessRunnerStatus::Active | HarnessRunnerStatus::Connected => RunnerStatus::Online,
        HarnessRunnerStatus::Inactive | HarnessRunnerStatus::Disconnected => RunnerStatus::Offline,
        HarnessRunnerStatus::Unhealthy => RunnerStatus::Unhealthy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_execution_status() {
        assert_eq!(
            map_execution_status(&HarnessStatus::Running),
            ExecutionStatus::Running
        );
        assert_eq!(
            map_execution_status(&HarnessStatus::Success),
            ExecutionStatus::Success
        );
        assert_eq!(
            map_execution_status(&HarnessStatus::Paused),
            ExecutionStatus::Paused
        );
    }

    #[test]
    fn test_map_runner_status() {
        assert_eq!(
            map_runner_status(&HarnessRunnerStatus::Active),
            RunnerStatus::Online
        );
        assert_eq!(
            map_runner_status(&HarnessRunnerStatus::Unhealthy),
            RunnerStatus::Unhealthy
        );
    }
}
