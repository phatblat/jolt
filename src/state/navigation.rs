// Navigation state management.
// Handles the navigation stack and breadcrumb trail for drill-down views.

use serde::{Deserialize, Serialize};

use crate::github::{RunConclusion, RunStatus};

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
    /// Top level: list of owners (users/orgs)
    Owners,
    /// Repositories for a specific owner
    Repositories { owner: String },
    /// Workflows for a specific repository
    Workflows { owner: String, repo: String },
    /// Runs for a specific workflow
    Runs {
        owner: String,
        repo: String,
        workflow_id: u64,
        workflow_name: String,
    },
    /// Jobs for a specific run
    Jobs {
        owner: String,
        repo: String,
        workflow_id: u64,
        run_id: u64,
        run_number: u64,
    },
    /// Log viewer for a specific job
    Logs {
        owner: String,
        repo: String,
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
        match self {
            ViewLevel::Owners => "Owners".to_string(),
            ViewLevel::Repositories { owner } => format!("{} / Repositories", owner),
            ViewLevel::Workflows { owner, repo } => format!("{}/{} / Workflows", owner, repo),
            ViewLevel::Runs { workflow_name, .. } => format!("{} / Runs", workflow_name),
            ViewLevel::Jobs { run_number, .. } => format!("Run #{} / Jobs", run_number),
            ViewLevel::Logs { job_name, .. } => format!("{} / Logs", job_name),
        }
    }

    /// Create a breadcrumb node for this view level.
    pub fn to_breadcrumb(&self) -> BreadcrumbNode {
        let label = match self {
            ViewLevel::Owners => "Owners".to_string(),
            ViewLevel::Repositories { owner } => owner.clone(),
            ViewLevel::Workflows { repo, .. } => repo.clone(),
            ViewLevel::Runs { workflow_name, .. } => workflow_name.clone(),
            ViewLevel::Jobs { run_number, .. } => format!("#{}", run_number),
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
        Self::new(ViewLevel::Owners)
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

        // Push repositories level
        nav.push(ViewLevel::Repositories {
            owner: "phatblat".to_string(),
        });
        assert_eq!(nav.depth(), 2);
        assert!(nav.can_go_back());

        // Push workflows level
        nav.push(ViewLevel::Workflows {
            owner: "phatblat".to_string(),
            repo: "jolt".to_string(),
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
        nav.push(ViewLevel::Repositories {
            owner: "phatblat".to_string(),
        });
        nav.push(ViewLevel::Workflows {
            owner: "phatblat".to_string(),
            repo: "jolt".to_string(),
        });

        let breadcrumbs = nav.breadcrumbs();
        assert_eq!(breadcrumbs.len(), 3);
        assert_eq!(breadcrumbs[0].label, "Owners");
        assert_eq!(breadcrumbs[1].label, "phatblat");
        assert_eq!(breadcrumbs[2].label, "jolt");
    }
}
