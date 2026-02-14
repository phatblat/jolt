// Navigation state management.
// Handles the navigation stack and breadcrumb trail for drill-down views.

use serde::{Deserialize, Serialize};

use crate::github::{RunConclusion, RunStatus};
use crate::types::Platform;

/// A node in the navigation breadcrumb trail.
#[derive(Debug, Clone)]
pub struct BreadcrumbNode {
    /// Display label for the breadcrumb.
    pub label: String,
    /// The view level this node represents.
    pub level: ViewLevel,
}

/// The current view level in the navigation hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewLevel {
    /// Top level: list of organizations (users/orgs)
    #[serde(alias = "Owners")]
    Organizations,
    /// Projects (repositories) for a specific organization
    #[serde(alias = "Repositories")]
    Projects {
        #[serde(alias = "owner")]
        org_id: String,
    },
    /// Workflows for a specific project
    Workflows {
        #[serde(alias = "owner")]
        org_id: String,
        #[serde(alias = "repo")]
        project_id: String,
    },
    /// Runs for a specific workflow
    Runs {
        #[serde(alias = "owner")]
        org_id: String,
        #[serde(alias = "repo")]
        project_id: String,
        workflow_id: u64,
        workflow_name: String,
    },
    /// Jobs for a specific run
    Jobs {
        #[serde(alias = "owner")]
        org_id: String,
        #[serde(alias = "repo")]
        project_id: String,
        workflow_id: u64,
        run_id: u64,
        run_number: u64,
    },
    /// Log viewer for a specific job
    Logs {
        #[serde(alias = "owner")]
        org_id: String,
        #[serde(alias = "repo")]
        project_id: String,
        workflow_id: u64,
        run_id: u64,
        job_id: u64,
        job_name: String,
        job_status: RunStatus,
        job_conclusion: Option<RunConclusion>,
    },
}

impl ViewLevel {
    /// Get the display title for this view level.
    pub fn title(&self) -> String {
        self.title_for_platform(None)
    }

    /// Get the display title for this view level with platform-aware labels.
    pub fn title_for_platform(&self, platform: Option<Platform>) -> String {
        match self {
            ViewLevel::Organizations => match platform {
                Some(Platform::GitHub) => "Owners".to_string(),
                _ => "Organizations".to_string(),
            },
            ViewLevel::Projects { org_id } => {
                let label = match platform {
                    Some(Platform::Harness) => "Projects",
                    Some(Platform::GitHub) => "Repositories",
                    None => "Projects",
                };
                format!("{org_id} / {label}")
            }
            ViewLevel::Workflows {
                org_id, project_id, ..
            } => format!("{org_id}/{project_id} / Workflows"),
            ViewLevel::Runs { workflow_name, .. } => format!("{workflow_name} / Runs"),
            ViewLevel::Jobs { run_number, .. } => format!("Run #{run_number} / Jobs"),
            ViewLevel::Logs { job_name, .. } => format!("{job_name} / Logs"),
        }
    }

    /// Create a breadcrumb node for this view level.
    pub fn to_breadcrumb(&self) -> BreadcrumbNode {
        self.to_breadcrumb_for_platform(None)
    }

    /// Create a breadcrumb node for this view level with platform-aware labels.
    pub fn to_breadcrumb_for_platform(&self, platform: Option<Platform>) -> BreadcrumbNode {
        let label = match self {
            ViewLevel::Organizations => match platform {
                Some(Platform::GitHub) => "Owners".to_string(),
                _ => "Organizations".to_string(),
            },
            ViewLevel::Projects { org_id } => org_id.clone(),
            ViewLevel::Workflows { project_id, .. } => project_id.clone(),
            ViewLevel::Runs { workflow_name, .. } => workflow_name.clone(),
            ViewLevel::Jobs { run_number, .. } => format!("#{run_number}"),
            ViewLevel::Logs { job_name, .. } => job_name.clone(),
        };
        BreadcrumbNode {
            label,
            level: self.clone(),
        }
    }
}

/// Navigation stack for a tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationStack {
    /// Stack of view levels (bottom = root, top = current)
    stack: Vec<ViewLevel>,
}

impl NavigationStack {
    /// Create a new navigation stack starting at the given level.
    pub fn new(root: ViewLevel) -> Self {
        Self { stack: vec![root] }
    }

    /// Get the current view level.
    pub fn current(&self) -> &ViewLevel {
        self.stack.last().expect("Stack should never be empty")
    }

    /// Push a new view level onto the stack (drill down).
    pub fn push(&mut self, level: ViewLevel) {
        self.stack.push(level);
    }

    /// Pop the current view level (go back). Returns false if at root.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    /// Check if we can go back (not at root).
    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    /// Get the breadcrumb trail.
    pub fn breadcrumbs(&self) -> Vec<BreadcrumbNode> {
        self.stack
            .iter()
            .map(|level| level.to_breadcrumb())
            .collect()
    }

    /// Get the breadcrumb trail with platform-aware labels.
    pub fn breadcrumbs_for_platform(&self, platform: Option<Platform>) -> Vec<BreadcrumbNode> {
        self.stack
            .iter()
            .map(|level| level.to_breadcrumb_for_platform(platform))
            .collect()
    }

    /// Reset to root level.
    pub fn reset(&mut self) {
        self.stack.truncate(1);
    }

    /// Get the depth of the navigation stack.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl Default for NavigationStack {
    fn default() -> Self {
        Self::new(ViewLevel::Organizations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_stack() {
        let mut nav = NavigationStack::default();

        assert_eq!(nav.depth(), 1);
        assert!(!nav.can_go_back());

        // Push projects level
        nav.push(ViewLevel::Projects {
            org_id: "phatblat".to_string(),
        });
        assert_eq!(nav.depth(), 2);
        assert!(nav.can_go_back());

        // Push workflows level
        nav.push(ViewLevel::Workflows {
            org_id: "phatblat".to_string(),
            project_id: "jolt".to_string(),
        });
        assert_eq!(nav.depth(), 3);

        // Pop back
        assert!(nav.pop());
        assert_eq!(nav.depth(), 2);

        // Pop again
        assert!(nav.pop());
        assert_eq!(nav.depth(), 1);

        // Can't pop past root
        assert!(!nav.pop());
        assert_eq!(nav.depth(), 1);
    }

    #[test]
    fn test_breadcrumbs() {
        let mut nav = NavigationStack::default();
        nav.push(ViewLevel::Projects {
            org_id: "phatblat".to_string(),
        });
        nav.push(ViewLevel::Workflows {
            org_id: "phatblat".to_string(),
            project_id: "jolt".to_string(),
        });

        let breadcrumbs = nav.breadcrumbs();
        assert_eq!(breadcrumbs.len(), 3);
        assert_eq!(breadcrumbs[0].label, "Organizations");
        assert_eq!(breadcrumbs[1].label, "phatblat");
        assert_eq!(breadcrumbs[2].label, "jolt");
    }

    #[test]
    fn test_breadcrumbs_github_platform() {
        let mut nav = NavigationStack::default();
        nav.push(ViewLevel::Projects {
            org_id: "phatblat".to_string(),
        });

        let breadcrumbs = nav.breadcrumbs_for_platform(Some(Platform::GitHub));
        assert_eq!(breadcrumbs[0].label, "Owners");
        assert_eq!(breadcrumbs[1].label, "phatblat");
    }
}
