// GitHub Actions platform adapter.
// Implements the Platform trait for GitHub Actions.

use async_trait::async_trait;

use crate::github::{GitHubClient, OwnerType as GHOwnerType};
use crate::platform::{Platform as PlatformTrait, PlatformError, Result};
use crate::types::*;

/// GitHub Actions platform implementation.
pub struct GitHubPlatform {
    client: GitHubClient,
}

impl GitHubPlatform {
    /// Create a new GitHub platform with the given token.
    pub fn new(token: &str) -> Result<Self> {
        let client = GitHubClient::new(token)?;
        Ok(Self { client })
    }

    /// Create a GitHub platform from the GITHUB_TOKEN environment variable.
    pub fn from_env() -> Result<Self> {
        let client = GitHubClient::from_env()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl PlatformTrait for GitHubPlatform {
    fn platform_type(&self) -> Platform {
        Platform::GitHub
    }

    fn is_authenticated(&self) -> bool {
        true // If we have a client, we're authenticated
    }

    async fn list_organizations(&mut self) -> Result<Vec<Organization>> {
        // For GitHub, we need to list both user and orgs
        // This is a simplified implementation - in reality we'd need to:
        // 1. Get the authenticated user
        // 2. List the user's organizations
        // For now, return empty to avoid API calls
        Ok(vec![])
    }

    async fn get_organization(&mut self, _org_id: &str) -> Result<Organization> {
        Err(PlatformError::NotSupported(
            "GitHub organization lookup not yet implemented".to_string(),
        ))
    }

    async fn list_projects(&mut self, _org_id: &str) -> Result<Vec<Project>> {
        // Would list repositories for the org
        Ok(vec![])
    }

    async fn get_project(&mut self, _org_id: &str, _project_id: &str) -> Result<Project> {
        Err(PlatformError::NotSupported(
            "GitHub project lookup not yet implemented".to_string(),
        ))
    }

    async fn list_workflows(&mut self, _project_id: &str) -> Result<Vec<Workflow>> {
        Ok(vec![])
    }

    async fn get_workflow(&mut self, _workflow_id: &str) -> Result<Workflow> {
        Err(PlatformError::NotSupported(
            "GitHub workflow lookup not yet implemented".to_string(),
        ))
    }

    async fn list_executions(&mut self, _workflow_id: &str) -> Result<Vec<Execution>> {
        Ok(vec![])
    }

    async fn get_execution(&mut self, _execution_id: &str) -> Result<Execution> {
        Err(PlatformError::NotSupported(
            "GitHub execution lookup not yet implemented".to_string(),
        ))
    }

    async fn list_jobs(&mut self, _execution_id: &str) -> Result<Vec<Job>> {
        Ok(vec![])
    }

    async fn get_job(&mut self, _job_id: &str) -> Result<Job> {
        Err(PlatformError::NotSupported(
            "GitHub job lookup not yet implemented".to_string(),
        ))
    }

    async fn list_steps(&mut self, _job_id: &str) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    async fn get_step(&mut self, _step_id: &str) -> Result<Step> {
        Err(PlatformError::NotSupported(
            "GitHub step lookup not yet implemented".to_string(),
        ))
    }

    async fn fetch_logs(&mut self, _step_id: &str) -> Result<Vec<LogLine>> {
        Ok(vec![])
    }

    async fn list_runners(&mut self, _scope: Option<&str>) -> Result<Vec<Runner>> {
        Ok(vec![])
    }

    async fn get_runner(&mut self, _runner_id: &str) -> Result<Runner> {
        Err(PlatformError::NotSupported(
            "GitHub runner lookup not yet implemented".to_string(),
        ))
    }
}

// ========================================
// Type Mappers: GitHub → Unified
// ========================================

/// Convert GitHub Owner to unified Organization.
pub fn map_owner_to_organization(owner: &crate::github::Owner) -> Organization {
    Organization {
        id: owner.login.clone(),
        name: owner.login.clone(),
        display_name: owner.login.clone(),
        platform: Platform::GitHub,
        description: None,
        org_type: Some(match owner.owner_type {
            GHOwnerType::User => OrgType::User,
            GHOwnerType::Organization => OrgType::Organization,
            GHOwnerType::Bot | GHOwnerType::Unknown => OrgType::Organization,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_owner_to_organization() {
        let owner = crate::github::Owner {
            id: 12345,
            login: "octocat".to_string(),
            owner_type: GHOwnerType::User,
            avatar_url: None,
        };

        let org = map_owner_to_organization(&owner);
        assert_eq!(org.id, "octocat");
        assert_eq!(org.platform, Platform::GitHub);
        assert_eq!(org.org_type, Some(OrgType::User));
    }
}
