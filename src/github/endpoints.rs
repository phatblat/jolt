// GitHub API endpoint functions.
// Provides typed methods for fetching data from the GitHub REST API.

use reqwest::Response;
use serde::{Deserialize, de::DeserializeOwned};

use crate::error::{JoltError, Result};

use super::client::GitHubClient;
use super::types::{
    EnrichedRunner, Job, Owner, Repository, RunStatus, Runner, RunnerJobInfo, Workflow, WorkflowRun,
};

/// Parse JSON response with better error messages.
async fn parse_json<T: DeserializeOwned>(response: Response) -> Result<T> {
    let text = response.text().await.map_err(JoltError::Api)?;
    serde_json::from_str(&text).map_err(|e| {
        // Include the first 500 chars of response for debugging
        let preview = if text.len() > 500 {
            format!("{}...", &text[..500])
        } else {
            text.clone()
        };
        JoltError::Other(format!("JSON parse error: {}. Response: {}", e, preview))
    })
}

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
        parse_json(response).await
    }

    /// Get organizations for the authenticated user.
    pub async fn get_user_orgs(&mut self) -> Result<Vec<Owner>> {
        let response = self.get("/user/orgs").await?;
        parse_json(response).await
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
        parse_json(response).await
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
        parse_json(response).await
    }

    /// Get a specific repository.
    pub async fn get_repo(&mut self, owner: &str, repo: &str) -> Result<Repository> {
        let response = self.get(&format!("/repos/{}/{}", owner, repo)).await?;
        parse_json(response).await
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
        let wrapper: WorkflowsResponse = parse_json(response).await?;
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
        let wrapper: WorkflowRunsResponse = parse_json(response).await?;
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
        let wrapper: WorkflowRunsResponse = parse_json(response).await?;
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
        parse_json(response).await
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
        let wrapper: JobsResponse = parse_json(response).await?;
        Ok((wrapper.jobs, wrapper.total_count))
    }

    /// Get logs for a job (returns raw text).
    /// Returns a user-friendly error if logs are not available.
    pub async fn get_job_logs(&mut self, owner: &str, repo: &str, job_id: u64) -> Result<String> {
        let result = self
            .get(&format!(
                "/repos/{}/{}/actions/jobs/{}/logs",
                owner, repo, job_id
            ))
            .await;

        match result {
            Ok(response) => {
                let logs = response.text().await.map_err(JoltError::Api)?;
                Ok(logs)
            }
            Err(JoltError::NotFound(_)) => Err(JoltError::Other(
                "Logs not available (may have expired or job is still running)".to_string(),
            )),
            Err(e) => Err(e),
        }
    }

    /// Get in-progress workflow runs for a repository.
    pub async fn get_in_progress_runs(
        &mut self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<WorkflowRun>> {
        let params = [("status", "in_progress"), ("per_page", "100")];
        let response = self
            .get_with_params(&format!("/repos/{}/{}/actions/runs", owner, repo), &params)
            .await?;
        let wrapper: WorkflowRunsResponse = parse_json(response).await?;
        Ok(wrapper.workflow_runs)
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
        let wrapper: RunnersResponse = parse_json(response).await?;
        Ok((wrapper.runners, wrapper.total_count))
    }

    /// Fetch runner enrichment data (job info) without fetching runners list.
    /// Returns a map of runner names to their current job info.
    pub async fn fetch_runner_enrichment_data(
        &mut self,
        owner: &str,
        repo: &str,
    ) -> std::collections::HashMap<String, RunnerJobInfo> {
        let mut enrichment_map = std::collections::HashMap::new();

        // Fetch in-progress runs (limit to first 10 runs)
        let in_progress_runs = match self.get_in_progress_runs(owner, repo).await {
            Ok(runs) => runs.into_iter().take(10).collect::<Vec<_>>(),
            Err(_) => return enrichment_map,
        };

        // Collect all jobs from in-progress runs (limit total jobs fetched)
        let mut all_jobs = Vec::new();
        for run in &in_progress_runs {
            if all_jobs.len() >= 50 {
                break;
            }
            if let Ok((jobs, _)) = self.get_jobs(owner, repo, run.id, 1, 50).await {
                for job in jobs {
                    all_jobs.push((job, run.clone()));
                    if all_jobs.len() >= 50 {
                        break;
                    }
                }
            }
        }

        // Build map of runner name to job info for in-progress jobs
        for (job, run) in all_jobs {
            if matches!(job.status, RunStatus::InProgress) {
                if let Some(runner_name) = job.runner_name {
                    enrichment_map.insert(
                        runner_name,
                        RunnerJobInfo {
                            pr_number: run.pull_requests.first().map(|pr| pr.number),
                            branch: run.head_branch.clone(),
                            started_at: job.started_at,
                            job_name: job.name.clone(),
                        },
                    );
                }
            }
        }

        enrichment_map
    }

    /// Get enriched runners - returns runners immediately without enrichment data.
    /// Enrichment data should be loaded separately using fetch_runner_enrichment_data.
    pub async fn get_enriched_runners(
        &mut self,
        owner: &str,
        repo: &str,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<EnrichedRunner>, u64)> {
        // Fetch runners and return immediately without enrichment
        let (runners, total_count) = self.get_runners(owner, repo, page, per_page).await?;

        let enriched_runners = runners
            .into_iter()
            .map(|runner| EnrichedRunner {
                runner,
                current_job: None,
            })
            .collect();

        Ok((enriched_runners, total_count))
    }
}
