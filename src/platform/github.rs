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
        // Get the authenticated user as the first "organization"
        let user = self.client.get_current_user().await?;
        let mut orgs = vec![map_owner_to_organization(&user)];

        // Get the user's organizations
        let user_orgs = self.client.get_user_orgs().await?;
        for org in &user_orgs {
            orgs.push(map_owner_to_organization(org));
        }

        Ok(orgs)
    }

    async fn get_organization(&mut self, _org_id: &str) -> Result<Organization> {
        Err(PlatformError::NotSupported(
            "GitHub organization lookup not yet implemented".to_string(),
        ))
    }

    async fn list_projects(&mut self, org_id: &str) -> Result<Vec<Project>> {
        // Determine if this is a user or org by checking against the authenticated user
        let user = self.client.get_current_user().await?;
        let repos = if user.login == org_id {
            self.client.get_user_repos(1, 30).await?
        } else {
            self.client.get_org_repos(org_id, 1, 30).await?
        };

        Ok(repos.iter().map(map_repository_to_project).collect())
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

    async fn list_runners(&mut self, scope: Option<&str>) -> Result<Vec<Runner>> {
        // scope format: "owner/repo"
        let scope = scope.ok_or_else(|| {
            PlatformError::Other("GitHub runners require owner/repo scope".to_string())
        })?;
        let parts: Vec<&str> = scope.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(PlatformError::Other(format!(
                "Invalid scope format '{}', expected 'owner/repo'",
                scope
            )));
        }
        let (owner, repo) = (parts[0], parts[1]);

        let (enriched, _count) = self.client.get_enriched_runners(owner, repo).await?;

        Ok(enriched
            .iter()
            .map(|e| Runner::from_github_enriched(e, owner, repo))
            .collect())
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

/// Convert GitHub Repository to unified Project.
pub fn map_repository_to_project(repo: &crate::github::Repository) -> Project {
    Project {
        id: repo.name.clone(),
        name: repo.name.clone(),
        display_name: repo.full_name.clone(),
        platform: Platform::GitHub,
        org_id: repo.owner.login.clone(),
        description: repo.description.clone(),
        visibility: Some(repo.private),
        updated_at: Some(repo.updated_at),
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

    #[test]
    fn test_map_owner_org_type() {
        let owner = crate::github::Owner {
            id: 67890,
            login: "my-org".to_string(),
            owner_type: GHOwnerType::Organization,
            avatar_url: None,
        };

        let org = map_owner_to_organization(&owner);
        assert_eq!(org.id, "my-org");
        assert_eq!(org.org_type, Some(OrgType::Organization));
    }

    #[test]
    fn test_map_repository_to_project() {
        use chrono::Utc;

        let repo = crate::github::Repository {
            id: 12345,
            name: "my-repo".to_string(),
            full_name: "octocat/my-repo".to_string(),
            owner: crate::github::Owner {
                id: 1,
                login: "octocat".to_string(),
                owner_type: GHOwnerType::User,
                avatar_url: None,
            },
            private: false,
            description: Some("A test repo".to_string()),
            updated_at: Utc::now(),
            pushed_at: None,
        };

        let project = map_repository_to_project(&repo);
        assert_eq!(project.id, "my-repo");
        assert_eq!(project.name, "my-repo");
        assert_eq!(project.display_name, "octocat/my-repo");
        assert_eq!(project.platform, Platform::GitHub);
        assert_eq!(project.org_id, "octocat");
        assert_eq!(project.description, Some("A test repo".to_string()));
        assert_eq!(project.visibility, Some(false));
        assert!(project.updated_at.is_some());
    }
}
