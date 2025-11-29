// Analysis tab state management.
// Handles saved log excerpts with full context for later review.

use chrono::{DateTime, Utc};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

use crate::github::{RunConclusion, RunStatus};

/// Source tab from which the analysis was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceTab {
    Workflows,
    Runners,
}

/// Navigation context to return to original log position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationContext {
    /// Which tab the session was captured from.
    pub source_tab: SourceTab,
    /// Owner (user or org login).
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Workflow ID (only for Workflows tab).
    pub workflow_id: Option<u64>,
    /// Workflow name.
    pub workflow_name: Option<String>,
    /// Run ID.
    pub run_id: u64,
    /// Run number (e.g., #123).
    pub run_number: u64,
    /// Job ID.
    pub job_id: u64,
    /// Job name.
    pub job_name: String,
    /// Job status at capture time.
    pub job_status: RunStatus,
    /// Job conclusion at capture time.
    pub job_conclusion: Option<RunConclusion>,
    /// Line number to scroll to (selection start).
    pub scroll_to_line: usize,
    /// Selection anchor line.
    pub selection_anchor: usize,
    /// Selection cursor line.
    pub selection_cursor: usize,
}

/// Metadata about the run that triggered the workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    /// PR number if this run was triggered by a PR.
    pub pr_number: Option<u64>,
    /// Branch name.
    pub branch_name: Option<String>,
    /// Commit SHA (short form).
    pub commit_sha: String,
    /// Author who triggered the run (from commit or PR).
    pub author: Option<String>,
    /// Runner name that executed the job.
    pub runner_name: Option<String>,
    /// Runner labels/tags.
    pub runner_labels: Vec<String>,
}

/// A saved analysis session containing log excerpt and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSession {
    /// Unique identifier for the session.
    pub id: String,
    /// Human-readable title (auto-generated).
    pub title: String,
    /// User-added notes (supports multi-line). Reserved for v2.
    pub notes: Option<String>,
    /// User-added tags for filtering/grouping. Reserved for v2.
    pub tags: Vec<String>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last modified.
    pub updated_at: DateTime<Utc>,
    /// Navigation context to return to original log.
    pub nav_context: NavigationContext,
    /// Run metadata (PR, branch, author, runner).
    pub run_metadata: RunMetadata,
    /// GitHub URL to the job logs.
    pub github_url: String,
    /// The saved log lines (excerpt).
    pub log_excerpt: String,
    /// Total line count of original log file.
    pub total_log_lines: usize,
    /// Start line of excerpt (0-indexed).
    pub excerpt_start_line: usize,
    /// End line of excerpt (0-indexed).
    pub excerpt_end_line: usize,
}

impl AnalysisSession {
    /// Create a new analysis session with auto-generated title and ID.
    pub fn new(
        nav_context: NavigationContext,
        run_metadata: RunMetadata,
        github_url: String,
        log_excerpt: String,
        total_log_lines: usize,
        excerpt_start_line: usize,
        excerpt_end_line: usize,
    ) -> Self {
        let line_count = excerpt_end_line - excerpt_start_line + 1;
        let title = format!(
            "{} - {} lines from {}",
            nav_context.job_name, line_count, nav_context.repo
        );

        let now = Utc::now();
        // Generate a simple unique ID using timestamp + random suffix
        let id = format!("{}-{:04x}", now.timestamp(), rand_u16());

        Self {
            id,
            title,
            notes: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            nav_context,
            run_metadata,
            github_url,
            log_excerpt,
            total_log_lines,
            excerpt_start_line,
            excerpt_end_line,
        }
    }
}

/// Simple random u16 for ID generation (avoid uuid dependency for now).
fn rand_u16() -> u16 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos & 0xFFFF) as u16
}

/// View level within the Analyze tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyzeViewLevel {
    /// Session list view.
    List,
    /// Detail view for a specific session.
    Detail { session_id: String },
}

/// State for the Analyze tab.
#[derive(Debug)]
pub struct AnalyzeTabState {
    /// Current view level.
    pub view: AnalyzeViewLevel,
    /// All loaded sessions.
    pub sessions: Vec<AnalysisSession>,
    /// List selection state.
    pub list_state: ListState,
    /// Scroll offset for detail view log excerpt.
    pub detail_scroll_y: u16,
}

impl Default for AnalyzeTabState {
    fn default() -> Self {
        Self {
            view: AnalyzeViewLevel::List,
            sessions: Vec::new(),
            list_state: ListState::default(),
            detail_scroll_y: 0,
        }
    }
}

impl AnalyzeTabState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected session index.
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Get the currently selected session.
    pub fn selected_session(&self) -> Option<&AnalysisSession> {
        self.list_state
            .selected()
            .and_then(|i| self.sessions.get(i))
    }

    /// Select the next session in the list.
    pub fn select_next(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.sessions.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Select the previous session in the list.
    pub fn select_prev(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Reset selection to first session.
    pub fn reset_selection(&mut self) {
        if self.sessions.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    /// Add a new session (inserts at front, newest first).
    pub fn add_session(&mut self, session: AnalysisSession) {
        self.sessions.insert(0, session);
        self.reset_selection();
    }

    /// Delete session by ID.
    pub fn delete_session(&mut self, id: &str) {
        self.sessions.retain(|s| s.id != id);
        // Adjust selection if needed
        if let Some(selected) = self.list_state.selected() {
            if selected >= self.sessions.len() {
                self.reset_selection();
            }
        }
    }

    /// Find session by ID.
    pub fn find_session(&self, id: &str) -> Option<&AnalysisSession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// Find session where any selected lines overlap with an existing session.
    pub fn find_overlapping(
        &self,
        job_id: u64,
        run_id: u64,
        start_line: usize,
        end_line: usize,
    ) -> Option<&AnalysisSession> {
        self.sessions.iter().find(|s| {
            s.nav_context.job_id == job_id
                && s.nav_context.run_id == run_id
                && Self::ranges_overlap(
                    start_line,
                    end_line,
                    s.excerpt_start_line,
                    s.excerpt_end_line,
                )
        })
    }

    /// Check if two line ranges overlap.
    fn ranges_overlap(start1: usize, end1: usize, start2: usize, end2: usize) -> bool {
        start1 <= end2 && start2 <= end1
    }

    /// Get all session line ranges for a specific job.
    /// Returns Vec of (start_line, end_line, session_id) for decoration.
    pub fn get_session_lines(&self, job_id: u64, run_id: u64) -> Vec<(usize, usize, String)> {
        self.sessions
            .iter()
            .filter(|s| s.nav_context.job_id == job_id && s.nav_context.run_id == run_id)
            .map(|s| (s.excerpt_start_line, s.excerpt_end_line, s.id.clone()))
            .collect()
    }

    /// Enter detail view for a specific session by ID.
    pub fn enter_detail_by_id(&mut self, session_id: &str) {
        // Find the index to select it in the list
        if let Some(index) = self.sessions.iter().position(|s| s.id == session_id) {
            self.list_state.select(Some(index));
            self.view = AnalyzeViewLevel::Detail {
                session_id: session_id.to_string(),
            };
            self.detail_scroll_y = 0;
        }
    }

    /// Enter detail view for the selected session.
    pub fn enter_detail(&mut self) {
        if let Some(session) = self.selected_session() {
            self.view = AnalyzeViewLevel::Detail {
                session_id: session.id.clone(),
            };
            self.detail_scroll_y = 0;
        }
    }

    /// Return to list view from detail view.
    pub fn exit_detail(&mut self) {
        self.view = AnalyzeViewLevel::List;
    }

    /// Scroll detail view down.
    pub fn scroll_down(&mut self) {
        if matches!(self.view, AnalyzeViewLevel::Detail { .. }) {
            self.detail_scroll_y = self.detail_scroll_y.saturating_add(1);
        }
    }

    /// Scroll detail view up.
    pub fn scroll_up(&mut self) {
        if matches!(self.view, AnalyzeViewLevel::Detail { .. }) {
            self.detail_scroll_y = self.detail_scroll_y.saturating_sub(1);
        }
    }

    /// Page down in detail view.
    pub fn page_down(&mut self) {
        if matches!(self.view, AnalyzeViewLevel::Detail { .. }) {
            self.detail_scroll_y = self.detail_scroll_y.saturating_add(20);
        }
    }

    /// Page up in detail view.
    pub fn page_up(&mut self) {
        if matches!(self.view, AnalyzeViewLevel::Detail { .. }) {
            self.detail_scroll_y = self.detail_scroll_y.saturating_sub(20);
        }
    }
}
