// Cache path utilities.
// Constructs filesystem paths for the cache hierarchy matching the GitHub object model.

use std::path::PathBuf;

use directories::ProjectDirs;

/// Get the base cache directory (~/.cache/jolt on macOS/Linux).
pub fn cache_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "jolt").map(|dirs| dirs.cache_dir().to_path_buf())
}

/// Path to the application state file.
pub fn state_path() -> Option<PathBuf> {
    cache_dir().map(|dir| dir.join("state.json"))
}

/// Path to the cached runners repositories list.
pub fn runners_repos_path() -> Option<PathBuf> {
    cache_dir().map(|dir| dir.join("runners_repos.json"))
}

/// Path to an owner's directory.
pub fn owner_dir(owner: &str) -> Option<PathBuf> {
    cache_dir().map(|dir| dir.join("owners").join(sanitize_name(owner)))
}

/// Path to an owner's metadata file.
pub fn owner_path(owner: &str) -> Option<PathBuf> {
    owner_dir(owner).map(|dir| dir.join("owner.json"))
}

/// Path to a repository's directory.
pub fn repo_dir(owner: &str, repo: &str) -> Option<PathBuf> {
    owner_dir(owner).map(|dir| dir.join("repos").join(sanitize_name(repo)))
}

/// Path to a repository's metadata file.
pub fn repo_path(owner: &str, repo: &str) -> Option<PathBuf> {
    repo_dir(owner, repo).map(|dir| dir.join("repo.json"))
}

/// Path to a repository's runners directory.
pub fn runners_dir(owner: &str, repo: &str) -> Option<PathBuf> {
    repo_dir(owner, repo).map(|dir| dir.join("runners"))
}

/// Path to a runner's metadata file.
pub fn runner_path(owner: &str, repo: &str, runner_id: u64) -> Option<PathBuf> {
    runners_dir(owner, repo).map(|dir| dir.join(format!("{}.json", runner_id)))
}

/// Path to a repository's workflows directory.
pub fn workflows_dir(owner: &str, repo: &str) -> Option<PathBuf> {
    repo_dir(owner, repo).map(|dir| dir.join("workflows"))
}

/// Path to a workflow's directory.
pub fn workflow_dir(owner: &str, repo: &str, workflow_id: u64) -> Option<PathBuf> {
    workflows_dir(owner, repo).map(|dir| dir.join(workflow_id.to_string()))
}

/// Path to a workflow's metadata file.
pub fn workflow_path(owner: &str, repo: &str, workflow_id: u64) -> Option<PathBuf> {
    workflow_dir(owner, repo, workflow_id).map(|dir| dir.join("workflow.json"))
}

/// Path to a workflow's runs directory.
pub fn runs_dir(owner: &str, repo: &str, workflow_id: u64) -> Option<PathBuf> {
    workflow_dir(owner, repo, workflow_id).map(|dir| dir.join("runs"))
}

/// Path to a workflow run's directory.
pub fn run_dir(owner: &str, repo: &str, workflow_id: u64, run_id: u64) -> Option<PathBuf> {
    runs_dir(owner, repo, workflow_id).map(|dir| dir.join(run_id.to_string()))
}

/// Path to a workflow run's metadata file.
pub fn run_path(owner: &str, repo: &str, workflow_id: u64, run_id: u64) -> Option<PathBuf> {
    run_dir(owner, repo, workflow_id, run_id).map(|dir| dir.join("run.json"))
}

/// Path to a workflow run's jobs directory.
pub fn jobs_dir(owner: &str, repo: &str, workflow_id: u64, run_id: u64) -> Option<PathBuf> {
    run_dir(owner, repo, workflow_id, run_id).map(|dir| dir.join("jobs"))
}

/// Path to a job's directory.
pub fn job_dir(
    owner: &str,
    repo: &str,
    workflow_id: u64,
    run_id: u64,
    job_id: u64,
) -> Option<PathBuf> {
    jobs_dir(owner, repo, workflow_id, run_id).map(|dir| dir.join(job_id.to_string()))
}

/// Path to a job's metadata file.
pub fn job_path(
    owner: &str,
    repo: &str,
    workflow_id: u64,
    run_id: u64,
    job_id: u64,
) -> Option<PathBuf> {
    job_dir(owner, repo, workflow_id, run_id, job_id).map(|dir| dir.join("job.json"))
}

/// Path to a job's log file.
pub fn job_log_path(
    owner: &str,
    repo: &str,
    workflow_id: u64,
    run_id: u64,
    job_id: u64,
) -> Option<PathBuf> {
    job_dir(owner, repo, workflow_id, run_id, job_id).map(|dir| dir.join("log.txt"))
}

/// Sanitize a name for use in filesystem paths.
/// Replaces problematic characters with underscores.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("simple"), "simple");
        assert_eq!(sanitize_name("with/slash"), "with_slash");
        assert_eq!(sanitize_name("owner:name"), "owner_name");
    }

    #[test]
    fn test_cache_paths() {
        // These tests verify path construction, not actual filesystem
        let owner = "phatblat";
        let repo = "jolt";
        let workflow_id = 12345u64;
        let run_id = 67890u64;
        let job_id = 11111u64;

        let owner_p = owner_path(owner).unwrap();
        assert!(owner_p.ends_with("owners/phatblat/owner.json"));

        let repo_p = repo_path(owner, repo).unwrap();
        assert!(repo_p.ends_with("owners/phatblat/repos/jolt/repo.json"));

        let workflow_p = workflow_path(owner, repo, workflow_id).unwrap();
        assert!(workflow_p.ends_with("workflows/12345/workflow.json"));

        let run_p = run_path(owner, repo, workflow_id, run_id).unwrap();
        assert!(run_p.ends_with("runs/67890/run.json"));

        let job_p = job_path(owner, repo, workflow_id, run_id, job_id).unwrap();
        assert!(job_p.ends_with("jobs/11111/job.json"));

        let log_p = job_log_path(owner, repo, workflow_id, run_id, job_id).unwrap();
        assert!(log_p.ends_with("jobs/11111/log.txt"));
    }
}
