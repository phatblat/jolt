// Harness API endpoint implementations.
// Provides high-level methods for interacting with Harness resources.

use super::client::HarnessClient;
use super::error::Result;
use super::types::*;

impl HarnessClient {
    // ========================================
    // Organizations
    // ========================================

    /// List all organizations in the account.
    pub async fn list_organizations(&self) -> Result<Vec<Organization>> {
        let response = self
            .get_with_params(
                "ng/api/organizations",
                &[("accountIdentifier", self.account_id())],
            )
            .await?;

        let page: PageResponse<Organization> = self.parse_response(response).await?;
        Ok(page.content)
    }

    /// Get a specific organization by identifier.
    pub async fn get_organization(&self, org_id: &str) -> Result<Organization> {
        let response = self
            .get_with_params(
                &format!("ng/api/organizations/{}", org_id),
                &[("accountIdentifier", self.account_id())],
            )
            .await?;

        self.parse_response(response).await
    }

    // ========================================
    // Projects
    // ========================================

    /// List all projects in an organization.
    pub async fn list_projects(&self, org_id: &str) -> Result<Vec<Project>> {
        let response = self
            .get_with_params(
                "ng/api/projects",
                &[
                    ("accountIdentifier", self.account_id()),
                    ("orgIdentifier", org_id),
                ],
            )
            .await?;

        let page: PageResponse<Project> = self.parse_response(response).await?;
        Ok(page.content)
    }

    /// Get a specific project by identifier.
    pub async fn get_project(&self, org_id: &str, project_id: &str) -> Result<Project> {
        let response = self
            .get_with_params(
                &format!("ng/api/projects/{}", project_id),
                &[
                    ("accountIdentifier", self.account_id()),
                    ("orgIdentifier", org_id),
                ],
            )
            .await?;

        self.parse_response(response).await
    }

    // ========================================
    // Runners
    // ========================================

    /// List runners with optional filtering by organization and project.
    pub async fn list_runners(
        &self,
        org_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<Runner>> {
        let mut params = vec![("accountIdentifier", self.account_id())];

        if let Some(org) = org_id {
            params.push(("orgIdentifier", org));
        }
        if let Some(project) = project_id {
            params.push(("projectIdentifier", project));
        }

        let response = self.get_with_params("ng/api/runner/list", &params).await?;

        let page: PageResponse<Runner> = self.parse_response(response).await?;
        Ok(page.content)
    }

    /// Get details for a specific runner.
    pub async fn get_runner(&self, runner_id: &str) -> Result<Runner> {
        let response = self
            .get_with_params(
                &format!("ng/api/runner/{}", runner_id),
                &[("accountIdentifier", self.account_id())],
            )
            .await?;

        self.parse_response(response).await
    }

    // ========================================
    // Pipelines
    // ========================================

    /// List pipelines in a project.
    pub async fn list_pipelines(&self, org_id: &str, project_id: &str) -> Result<Vec<Pipeline>> {
        let response = self
            .get_with_params(
                "pipeline/api/pipelines",
                &[
                    ("accountIdentifier", self.account_id()),
                    ("orgIdentifier", org_id),
                    ("projectIdentifier", project_id),
                ],
            )
            .await?;

        let page: PageResponse<Pipeline> = self.parse_response(response).await?;
        Ok(page.content)
    }

    // ========================================
    // Executions
    // ========================================

    /// List pipeline executions with optional filtering.
    pub async fn list_executions(
        &self,
        org_id: &str,
        project_id: &str,
        filter: Option<ExecutionFilter>,
    ) -> Result<Vec<Execution>> {
        let body = filter.unwrap_or_default();

        let response = self
            .post(
                &format!(
                    "pipeline/api/pipelines/execution/v2/list?accountIdentifier={}&orgIdentifier={}&projectIdentifier={}",
                    self.account_id(),
                    org_id,
                    project_id
                ),
                &body,
            )
            .await?;

        let page: PageResponse<Execution> = self.parse_response(response).await?;
        Ok(page.content)
    }

    /// Get details for a specific execution.
    pub async fn get_execution(&self, execution_id: &str) -> Result<ExecutionDetails> {
        let response = self
            .get(&format!(
                "pipeline/api/pipelines/execution/{}",
                execution_id
            ))
            .await?;

        self.parse_response(response).await
    }

    // ========================================
    // Logs
    // ========================================

    /// Fetch logs for an execution.
    pub async fn fetch_logs(&self, execution_id: &str) -> Result<LogResponse> {
        let response = self
            .get(&format!(
                "pipeline/api/pipelines/execution/{}/logs",
                execution_id
            ))
            .await?;

        self.parse_response(response).await
    }

    /// Fetch logs for a specific step using a log key.
    ///
    /// # Arguments
    ///
    /// * `log_key` - Full log key in format: accountId/orgId/projectId/pipelineId/executionId/stageId/stepId
    pub async fn fetch_step_logs(&self, log_key: &str) -> Result<LogResponse> {
        let response = self
            .get_with_params(
                "log-service/log-stream",
                &[("accountID", self.account_id()), ("key", log_key)],
            )
            .await?;

        self.parse_response(response).await
    }

    /// Build a log key for fetching step logs.
    pub fn build_log_key(
        account_id: &str,
        org_id: &str,
        project_id: &str,
        pipeline_id: &str,
        execution_id: &str,
        stage_id: &str,
        step_id: &str,
    ) -> String {
        format!(
            "{}/{}/{}/{}/{}/{}/{}",
            account_id, org_id, project_id, pipeline_id, execution_id, stage_id, step_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_filter_default() {
        let filter = ExecutionFilter::default();
        assert!(filter.status.is_none());
        assert!(filter.pipeline_ids.is_none());
    }

    #[test]
    fn test_execution_filter_with_status() {
        let filter = ExecutionFilter {
            status: Some(vec![ExecutionStatus::Running, ExecutionStatus::Queued]),
            pipeline_ids: None,
        };
        assert!(filter.status.is_some());
        assert_eq!(filter.status.as_ref().unwrap().len(), 2);
    }
}
