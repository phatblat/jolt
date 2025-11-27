// GitHub API endpoint functions.
// Provides typed methods for fetching data from the GitHub REST API.

use serde::Deserialize;

use crate::error::Result;

use super::client::GitHubClient;
use super::types::{Job, Owner, Repository, Runner, Workflow, WorkflowRun};

/// Response wrapper for workflows list.
#[derive(Debug, Deserialize)]
struct WorkflowsResponse {
    total_count: u64,
    workflows: Vec<Workflow>,
}

/// Response wrapper for workflow runs list.
#[derive(Debug, Deserialize)]
struct WorkflowRunsResponse {
    total_count: u64,
    workflow_runs: Vec<WorkflowRun>,
}

/// Response wrapper for jobs list.
#[derive(Debug, Deserialize)]
struct JobsResponse {
    total_count: u64,
    jobs: Vec<Job>,
}

/// Response wrapper for runners list.
#[derive(Debug, Deserialize)]
struct RunnersResponse {
    total_count: u64,
    runners: Vec<Runner>,
}

impl GitHubClient {
    /// Get the authenticated user.
    pub async fn get_current_user(&mut self) -> Result<Owner> {
        let response = self.get("/user").await?;
        let user: Owner = response.json().await?;
        Ok(user)
    }

    /// Get organizations for the authenticated user.
    pub async fn get_user_orgs(&mut self) -> Result<Vec<Owner>> {
        let response = self.get("/user/orgs").await?;
        let orgs: Vec<Owner> = response.json().await?;
        Ok(orgs)
    }

    /// Get repositories accessible to the authenticated user.
    pub async fn get_user_repos(&mut self, page: u32, per_page: u32) -> Result<Vec<Repository>> {
        let params = [
            ("sort", "updated"),
            ("direction", "desc"),
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self.get_with_params("/user/repos", &params).await?;
        let repos: Vec<Repository> = response.json().await?;
        Ok(repos)
    }

    /// Get repositories for an organization.
    pub async fn get_org_repos(
        &mut self,
        org: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<Repository>> {
        let params = [
            ("sort", "updated"),
            ("direction", "desc"),
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self
            .get_with_params(&format!("/orgs/{}/repos", org), &params)
            .await?;
        let repos: Vec<Repository> = response.json().await?;
        Ok(repos)
    }

    /// Get a specific repository.
    pub async fn get_repo(&mut self, owner: &str, repo: &str) -> Result<Repository> {
        let response = self.get(&format!("/repos/{}/{}", owner, repo)).await?;
        let repository: Repository = response.json().await?;
        Ok(repository)
    }

    /// Get workflows for a repository.
    pub async fn get_workflows(
        &mut self,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<Workflow>, u64)> {
        let params = [
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self
            .get_with_params(
                &format!("/repos/{}/{}/actions/workflows", owner, repo),
                &params,
            )
            .await?;
        let wrapper: WorkflowsResponse = response.json().await?;
        Ok((wrapper.workflows, wrapper.total_count))
    }

    /// Get workflow runs for a repository.
    pub async fn get_workflow_runs(
        &mut self,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<WorkflowRun>, u64)> {
        let params = [
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self
            .get_with_params(&format!("/repos/{}/{}/actions/runs", owner, repo), &params)
            .await?;
        let wrapper: WorkflowRunsResponse = response.json().await?;
        Ok((wrapper.workflow_runs, wrapper.total_count))
    }

    /// Get workflow runs for a specific workflow.
    pub async fn get_workflow_runs_for_workflow(
        &mut self,
        owner: &str,
        repo: &str,
        workflow_id: u64,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<WorkflowRun>, u64)> {
        let params = [
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self
            .get_with_params(
                &format!(
                    "/repos/{}/{}/actions/workflows/{}/runs",
                    owner, repo, workflow_id
                ),
                &params,
            )
            .await?;
        let wrapper: WorkflowRunsResponse = response.json().await?;
        Ok((wrapper.workflow_runs, wrapper.total_count))
    }

    /// Get a specific workflow run.
    pub async fn get_workflow_run(
        &mut self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRun> {
        let response = self
            .get(&format!(
                "/repos/{}/{}/actions/runs/{}",
                owner, repo, run_id
            ))
            .await?;
        let run: WorkflowRun = response.json().await?;
        Ok(run)
    }

    /// Get jobs for a workflow run.
    pub async fn get_jobs(
        &mut self,
        owner: &str,
        repo: &str,
        run_id: u64,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<Job>, u64)> {
        let params = [
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self
            .get_with_params(
                &format!("/repos/{}/{}/actions/runs/{}/jobs", owner, repo, run_id),
                &params,
            )
            .await?;
        let wrapper: JobsResponse = response.json().await?;
        Ok((wrapper.jobs, wrapper.total_count))
    }

    /// Get logs for a job (returns raw text).
    pub async fn get_job_logs(&mut self, owner: &str, repo: &str, job_id: u64) -> Result<String> {
        let response = self
            .get(&format!(
                "/repos/{}/{}/actions/jobs/{}/logs",
                owner, repo, job_id
            ))
            .await?;
        let logs = response.text().await?;
        Ok(logs)
    }

    /// Get runners for a repository (requires admin access).
    pub async fn get_runners(
        &mut self,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<Runner>, u64)> {
        let params = [
            ("page", &page.to_string()),
            ("per_page", &per_page.to_string()),
        ];
        let response = self
            .get_with_params(
                &format!("/repos/{}/{}/actions/runners", owner, repo),
                &params,
            )
            .await?;
        let wrapper: RunnersResponse = response.json().await?;
        Ok((wrapper.runners, wrapper.total_count))
    }
}
