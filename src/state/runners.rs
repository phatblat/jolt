// Runners tab state management.
// Handles navigation and data for the runners tab.

use serde::{Deserialize, Serialize};

use crate::github::{JobGroup, JobListItem, RunConclusion, RunStatus, WorkflowRun};
use crate::types::{Platform, Project, Runner};

use super::workflows::{LoadingState, SelectableList};

/// Navigation level for the Runners tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnersViewLevel {
    /// Top level: projects (repositories) with runners
    Projects,
    /// Runners for a specific project
    Runners { org_id: String, project_id: String },
    /// Workflow runs (optionally filtered by runner)
    Runs {
        org_id: String,
        project_id: String,
        runner_name: Option<String>,
    },
    /// Jobs for a specific run
    Jobs {
        org_id: String,
        project_id: String,
        run_id: String,
        run_number: u64,
    },
    /// Log viewer for a specific job
    Logs {
        org_id: String,
        project_id: String,
        run_id: String,
        job_id: String,
        job_name: String,
        job_status: RunStatus,
        job_conclusion: Option<RunConclusion>,
    },
}

impl RunnersViewLevel {
    /// Get the display title for this view level.
    pub fn title(&self) -> String {
        self.title_for_platform(None)
    }

    /// Get the display title for this view level with platform-aware labels.
    pub fn title_for_platform(&self, platform: Option<Platform>) -> String {
        match self {
            RunnersViewLevel::Projects => match platform {
                Some(Platform::Harness) => "Projects".to_string(),
                Some(Platform::GitHub) => "Repositories".to_string(),
                None => "Projects".to_string(),
            },
            RunnersViewLevel::Runners { org_id, project_id } => {
                format!("{org_id}/{project_id} / Runners")
            }
            RunnersViewLevel::Runs { runner_name, .. } => {
                let label = match platform {
                    Some(Platform::Harness) => "Executions",
                    _ => "Runs",
                };
                if let Some(name) = runner_name {
                    format!("{name} / {label}")
                } else {
                    format!("All {}", label)
                }
            }
            RunnersViewLevel::Jobs { run_number, .. } => {
                let label = match platform {
                    Some(Platform::Harness) => "Stages",
                    _ => "Jobs",
                };
                format!("Run #{run_number} / {label}")
            }
            RunnersViewLevel::Logs { job_name, .. } => format!("{job_name} / Logs"),
        }
    }

    /// Create a breadcrumb label for this level.
    pub fn breadcrumb_label(&self) -> String {
        self.breadcrumb_label_for_platform(None)
    }

    /// Create a breadcrumb label for this level with platform-aware labels.
    pub fn breadcrumb_label_for_platform(&self, platform: Option<Platform>) -> String {
        match self {
            RunnersViewLevel::Projects => match platform {
                Some(Platform::Harness) => "Projects".to_string(),
                Some(Platform::GitHub) => "Repos".to_string(),
                None => "Projects".to_string(),
            },
            RunnersViewLevel::Runners { project_id, .. } => project_id.clone(),
            RunnersViewLevel::Runs { runner_name, .. } => {
                let label = match platform {
                    Some(Platform::Harness) => "Executions",
                    _ => "Runs",
                };
                runner_name.clone().unwrap_or_else(|| label.to_string())
            }
            RunnersViewLevel::Jobs { run_number, .. } => format!("#{run_number}"),
            RunnersViewLevel::Logs { job_name, .. } => job_name.clone(),
        }
    }
}

/// Breadcrumb node for runners navigation.
#[derive(Debug, Clone)]
pub struct RunnersBreadcrumb {
    pub label: String,
    pub level: RunnersViewLevel,
}

/// Navigation stack for runners tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnersNavStack {
    stack: Vec<RunnersViewLevel>,
}

impl Default for RunnersNavStack {
    fn default() -> Self {
        Self {
            stack: vec![RunnersViewLevel::Projects],
        }
    }
}

impl RunnersNavStack {
    /// Get the current view level.
    pub fn current(&self) -> &RunnersViewLevel {
        self.stack.last().unwrap()
    }

    /// Push a new level onto the stack.
    pub fn push(&mut self, level: RunnersViewLevel) {
        self.stack.push(level);
    }

    /// Pop the current level and return to the previous one.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    /// Get the breadcrumb trail.
    pub fn breadcrumbs(&self) -> Vec<RunnersBreadcrumb> {
        self.breadcrumbs_for_platform(None)
    }

    /// Get the breadcrumb trail with platform-aware labels.
    pub fn breadcrumbs_for_platform(&self, platform: Option<Platform>) -> Vec<RunnersBreadcrumb> {
        self.stack
            .iter()
            .map(|level| RunnersBreadcrumb {
                label: level.breadcrumb_label_for_platform(platform),
                level: level.clone(),
            })
            .collect()
    }
}

/// Complete state for the runners tab.
#[derive(Debug)]
pub struct RunnersTabState {
    /// Navigation stack for breadcrumb trail.
    pub nav: RunnersNavStack,
    /// Projects (repositories) with runners.
    pub projects: SelectableList<Project>,
    /// Runners list for current project (unified type).
    pub runners: SelectableList<Runner>,
    /// Workflow runs list (still GitHub-specific for hybrid approach).
    pub runs: SelectableList<WorkflowRun>,
    /// Jobs list for current run (still GitHub-specific for hybrid approach).
    pub jobs: SelectableList<crate::github::Job>,
    /// Grouped jobs with attempts.
    pub job_groups: Vec<JobGroup>,
    /// Flattened job list items for display.
    pub job_list_items: Vec<JobListItem>,
    /// Log content for current job.
    pub log_content: LoadingState<String>,
    /// Horizontal scroll offset for log viewer.
    pub log_scroll_x: u16,
    /// Vertical scroll offset for log viewer.
    pub log_scroll_y: u16,
    /// Selection anchor line in log viewer (0-indexed).
    pub log_selection_anchor: usize,
    /// Selection cursor line in log viewer (0-indexed).
    pub log_selection_cursor: usize,
    /// When we entered the runners list view (for auto-refresh).
    pub runners_view_entered_at: Option<std::time::Instant>,
    /// When to next refresh the runners list.
    pub runners_next_refresh: Option<std::time::Instant>,
    /// Whether enrichment data is currently being loaded.
    pub enrichment_loading: bool,
}

impl Default for RunnersTabState {
    fn default() -> Self {
        Self {
            nav: RunnersNavStack::default(),
            projects: SelectableList::new(),
            runners: SelectableList::new(),
            runs: SelectableList::new(),
            jobs: SelectableList::new(),
            job_groups: Vec::new(),
            job_list_items: Vec::new(),
            log_content: LoadingState::Idle,
            log_scroll_x: 0,
            log_scroll_y: 0,
            log_selection_anchor: 0,
            log_selection_cursor: 0,
            runners_view_entered_at: None,
            runners_next_refresh: None,
            enrichment_loading: false,
        }
    }
}

impl RunnersTabState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current view level.
    pub fn current_view(&self) -> &RunnersViewLevel {
        self.nav.current()
    }

    /// Get the platform of the currently selected project, if any.
    /// Used for platform-aware labels in breadcrumbs and titles.
    pub fn current_platform(&self) -> Option<Platform> {
        self.projects.selected_item().map(|p| p.platform)
    }

    /// Navigate back (Escape key).
    /// Clears all child list data so fresh data loads when drilling down again.
    pub fn go_back(&mut self) -> bool {
        let current = self.nav.current().clone();
        let popped = self.nav.pop();

        if popped {
            match current {
                RunnersViewLevel::Runners { .. } => {
                    self.runners = SelectableList::new();
                    self.runs = SelectableList::new();
                    self.jobs = SelectableList::new();
                    self.job_groups = Vec::new();
                    self.job_list_items = Vec::new();
                    self.log_content = LoadingState::Idle;
                }
                RunnersViewLevel::Runs { .. } => {
                    self.runs = SelectableList::new();
                    self.jobs = SelectableList::new();
                    self.job_groups = Vec::new();
                    self.job_list_items = Vec::new();
                    self.log_content = LoadingState::Idle;
                }
                RunnersViewLevel::Jobs { .. } => {
                    self.jobs = SelectableList::new();
                    self.job_groups = Vec::new();
                    self.job_list_items = Vec::new();
                    self.log_content = LoadingState::Idle;
                }
                RunnersViewLevel::Logs { .. } => {
                    self.log_content = LoadingState::Idle;
                    self.log_scroll_x = 0;
                    self.log_scroll_y = 0;
                    self.log_selection_anchor = 0;
                    self.log_selection_cursor = 0;
                }
                RunnersViewLevel::Projects => {}
            }
        }
        popped
    }

    /// Handle up arrow key.
    pub fn select_prev(&mut self) {
        match self.nav.current() {
            RunnersViewLevel::Projects => self.projects.select_prev(),
            RunnersViewLevel::Runners { .. } => self.runners.select_prev(),
            RunnersViewLevel::Runs { .. } => self.runs.select_prev(),
            RunnersViewLevel::Jobs { .. } => self.jobs.select_prev(),
            RunnersViewLevel::Logs { .. } => {
                self.log_scroll_y = self.log_scroll_y.saturating_sub(1);
            }
        }
    }

    /// Handle down arrow key.
    pub fn select_next(&mut self) {
        match self.nav.current() {
            RunnersViewLevel::Projects => self.projects.select_next(),
            RunnersViewLevel::Runners { .. } => self.runners.select_next(),
            RunnersViewLevel::Runs { .. } => self.runs.select_next(),
            RunnersViewLevel::Jobs { .. } => self.jobs.select_next(),
            RunnersViewLevel::Logs { .. } => {
                self.log_scroll_y = self.log_scroll_y.saturating_add(1);
            }
        }
    }

    /// Handle left arrow key (horizontal scroll in logs).
    pub fn scroll_left(&mut self) {
        if matches!(self.nav.current(), RunnersViewLevel::Logs { .. }) {
            self.log_scroll_x = self.log_scroll_x.saturating_sub(4);
        }
    }

    /// Handle right arrow key (horizontal scroll in logs).
    pub fn scroll_right(&mut self) {
        if matches!(self.nav.current(), RunnersViewLevel::Logs { .. }) {
            self.log_scroll_x = self.log_scroll_x.saturating_add(4);
        }
    }

    /// Handle Page Up key.
    pub fn page_up(&mut self) {
        if matches!(self.nav.current(), RunnersViewLevel::Logs { .. }) {
            self.log_scroll_y = self.log_scroll_y.saturating_sub(20);
        }
    }

    /// Handle Page Down key.
    pub fn page_down(&mut self) {
        if matches!(self.nav.current(), RunnersViewLevel::Logs { .. }) {
            self.log_scroll_y = self.log_scroll_y.saturating_add(20);
        }
    }

    /// Scroll to start of logs.
    pub fn scroll_to_start(&mut self) {
        if matches!(self.nav.current(), RunnersViewLevel::Logs { .. }) {
            self.log_scroll_y = 0;
            self.log_scroll_x = 0;
        }
    }

    /// Scroll to end of logs.
    #[allow(clippy::collapsible_if)]
    pub fn scroll_to_end(&mut self) {
        if matches!(self.nav.current(), RunnersViewLevel::Logs { .. }) {
            if let LoadingState::Loaded(logs) = &self.log_content {
                let line_count = logs.lines().count() as u16;
                self.log_scroll_y = line_count.saturating_sub(10);
            }
        }
    }

    /// Clear current list data (for refresh).
    pub fn clear_current(&mut self) {
        match self.nav.current() {
            RunnersViewLevel::Projects => self.projects = SelectableList::new(),
            RunnersViewLevel::Runners { .. } => self.runners = SelectableList::new(),
            RunnersViewLevel::Runs { .. } => self.runs = SelectableList::new(),
            RunnersViewLevel::Jobs { .. } => {
                self.jobs = SelectableList::new();
                self.job_groups = Vec::new();
                self.job_list_items = Vec::new();
            }
            RunnersViewLevel::Logs { .. } => {
                self.log_content = LoadingState::Idle;
                self.log_scroll_x = 0;
                self.log_scroll_y = 0;
                self.log_selection_anchor = 0;
                self.log_selection_cursor = 0;
            }
        }
    }

    /// Get the current selection range (start, end) as 0-indexed line numbers.
    pub fn log_selection_range(&self) -> (usize, usize) {
        let start = self.log_selection_anchor.min(self.log_selection_cursor);
        let end = self.log_selection_anchor.max(self.log_selection_cursor);
        (start, end)
    }

    /// Move selection cursor up (with optional extend for shift+up).
    pub fn selection_up(&mut self, extend: bool) {
        if let LoadingState::Loaded(_) = &self.log_content
            && self.log_selection_cursor > 0
        {
            self.log_selection_cursor -= 1;
            if !extend {
                self.log_selection_anchor = self.log_selection_cursor;
            }
        }
    }

    /// Move selection cursor down (with optional extend for shift+down).
    pub fn selection_down(&mut self, extend: bool) {
        if let LoadingState::Loaded(logs) = &self.log_content {
            let max_line = logs.lines().count().saturating_sub(1);
            if self.log_selection_cursor < max_line {
                self.log_selection_cursor += 1;
                if !extend {
                    self.log_selection_anchor = self.log_selection_cursor;
                }
            }
        }
    }

    /// Move selection to start of file.
    pub fn selection_to_start(&mut self, extend: bool) {
        self.log_selection_cursor = 0;
        if !extend {
            self.log_selection_anchor = 0;
        }
    }

    /// Move selection to end of file.
    pub fn selection_to_end(&mut self, extend: bool) {
        if let LoadingState::Loaded(logs) = &self.log_content {
            let max_line = logs.lines().count().saturating_sub(1);
            self.log_selection_cursor = max_line;
            if !extend {
                self.log_selection_anchor = max_line;
            }
        }
    }

    /// Move selection up by a page.
    pub fn selection_page_up(&mut self, extend: bool) {
        if let LoadingState::Loaded(_) = &self.log_content {
            self.log_selection_cursor = self.log_selection_cursor.saturating_sub(20);
            if !extend {
                self.log_selection_anchor = self.log_selection_cursor;
            }
        }
    }

    /// Move selection down by a page.
    pub fn selection_page_down(&mut self, extend: bool) {
        if let LoadingState::Loaded(logs) = &self.log_content {
            let max_line = logs.lines().count().saturating_sub(1);
            self.log_selection_cursor = (self.log_selection_cursor + 20).min(max_line);
            if !extend {
                self.log_selection_anchor = self.log_selection_cursor;
            }
        }
    }
}
