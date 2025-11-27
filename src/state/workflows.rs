// Workflows tab state management.
// Handles data loading, caching, and list state for the workflows tab.

use ratatui::widgets::ListState;

use crate::github::{Job, Owner, Repository, Workflow, WorkflowRun};

use super::navigation::{NavigationStack, ViewLevel};

/// Loading state for async data.
#[derive(Debug, Clone, Default)]
pub enum LoadingState<T> {
    #[default]
    Idle,
    Loading,
    Loaded(T),
    Error(String),
}

impl<T> LoadingState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, LoadingState::Loading)
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, LoadingState::Loaded(_))
    }

    pub fn data(&self) -> Option<&T> {
        match self {
            LoadingState::Loaded(data) => Some(data),
            _ => None,
        }
    }
}

/// Paginated list data.
#[derive(Debug, Clone)]
pub struct PaginatedList<T> {
    pub items: Vec<T>,
    pub total_count: u64,
    pub current_page: u32,
    pub has_more: bool,
    pub loading_more: bool,
}

impl<T> Default for PaginatedList<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total_count: 0,
            current_page: 1,
            has_more: false,
            loading_more: false,
        }
    }
}

impl<T> PaginatedList<T> {
    pub fn new(items: Vec<T>, total_count: u64) -> Self {
        let has_more = items.len() < total_count as usize;
        Self {
            items,
            total_count,
            current_page: 1,
            has_more,
            loading_more: false,
        }
    }

    pub fn append(&mut self, mut items: Vec<T>, total_count: u64) {
        self.items.append(&mut items);
        self.total_count = total_count;
        self.current_page += 1;
        self.has_more = self.items.len() < total_count as usize;
        self.loading_more = false;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// State for a selectable list with keyboard navigation.
#[derive(Debug, Clone)]
pub struct SelectableList<T> {
    pub data: LoadingState<PaginatedList<T>>,
    pub list_state: ListState,
    pub filter: Option<String>,
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        Self {
            data: LoadingState::Idle,
            list_state: ListState::default(),
            filter: None,
        }
    }
}

impl<T> SelectableList<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected index.
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Select the next item in the list.
    pub fn select_next(&mut self) {
        if let Some(items) = self.data.data() {
            if items.is_empty() {
                return;
            }
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= items.len() - 1 {
                        i // Stay at end
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Select the previous item in the list.
    pub fn select_prev(&mut self) {
        if let Some(items) = self.data.data() {
            if items.is_empty() {
                return;
            }
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        0 // Stay at start
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    /// Get the selected item.
    pub fn selected_item(&self) -> Option<&T> {
        let index = self.list_state.selected()?;
        let items = self.data.data()?;
        items.items.get(index)
    }

    /// Check if we're near the end of the list (for pagination trigger).
    pub fn near_end(&self, threshold: usize) -> bool {
        if let (Some(index), Some(items)) = (self.list_state.selected(), self.data.data()) {
            items.has_more && index >= items.len().saturating_sub(threshold)
        } else {
            false
        }
    }

    /// Reset selection to first item.
    pub fn reset_selection(&mut self) {
        if let Some(items) = self.data.data() {
            if !items.is_empty() {
                self.list_state.select(Some(0));
            } else {
                self.list_state.select(None);
            }
        } else {
            self.list_state.select(None);
        }
    }

    /// Set loaded data.
    pub fn set_loaded(&mut self, items: Vec<T>, total_count: u64) {
        self.data = LoadingState::Loaded(PaginatedList::new(items, total_count));
        self.reset_selection();
    }

    /// Set loading state.
    pub fn set_loading(&mut self) {
        self.data = LoadingState::Loading;
    }

    /// Set error state.
    pub fn set_error(&mut self, error: String) {
        self.data = LoadingState::Error(error);
    }
}

/// Complete state for the workflows tab.
#[derive(Debug)]
pub struct WorkflowsTabState {
    /// Navigation stack for breadcrumb trail.
    pub nav: NavigationStack,
    /// Owners list (user + orgs).
    pub owners: SelectableList<Owner>,
    /// Repositories list for current owner.
    pub repositories: SelectableList<Repository>,
    /// Workflows list for current repository.
    pub workflows: SelectableList<Workflow>,
    /// Workflow runs list for current workflow.
    pub runs: SelectableList<WorkflowRun>,
    /// Jobs list for current run.
    pub jobs: SelectableList<Job>,
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
}

impl Default for WorkflowsTabState {
    fn default() -> Self {
        Self {
            nav: NavigationStack::default(),
            owners: SelectableList::new(),
            repositories: SelectableList::new(),
            workflows: SelectableList::new(),
            runs: SelectableList::new(),
            jobs: SelectableList::new(),
            log_content: LoadingState::Idle,
            log_scroll_x: 0,
            log_scroll_y: 0,
            log_selection_anchor: 0,
            log_selection_cursor: 0,
        }
    }
}

impl WorkflowsTabState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current list based on navigation level.
    pub fn current_view(&self) -> &ViewLevel {
        self.nav.current()
    }

    /// Navigate back (Escape key).
    /// Clears all child list data so fresh data loads when drilling down again.
    pub fn go_back(&mut self) -> bool {
        let current = self.nav.current().clone();
        let popped = self.nav.pop();

        if popped {
            // Clear all lists below the level we came from
            match current {
                ViewLevel::Repositories { .. } => {
                    self.repositories = SelectableList::new();
                    self.workflows = SelectableList::new();
                    self.runs = SelectableList::new();
                    self.jobs = SelectableList::new();
                    self.log_content = LoadingState::Idle;
                }
                ViewLevel::Workflows { .. } => {
                    self.workflows = SelectableList::new();
                    self.runs = SelectableList::new();
                    self.jobs = SelectableList::new();
                    self.log_content = LoadingState::Idle;
                }
                ViewLevel::Runs { .. } => {
                    self.runs = SelectableList::new();
                    self.jobs = SelectableList::new();
                    self.log_content = LoadingState::Idle;
                }
                ViewLevel::Jobs { .. } => {
                    self.jobs = SelectableList::new();
                    self.log_content = LoadingState::Idle;
                }
                ViewLevel::Logs { .. } => {
                    self.log_content = LoadingState::Idle;
                    self.log_scroll_x = 0;
                    self.log_scroll_y = 0;
                    self.log_selection_anchor = 0;
                    self.log_selection_cursor = 0;
                }
                ViewLevel::Owners => {}
            }
        }
        popped
    }

    /// Handle up arrow key.
    pub fn select_prev(&mut self) {
        match self.nav.current() {
            ViewLevel::Owners => self.owners.select_prev(),
            ViewLevel::Repositories { .. } => self.repositories.select_prev(),
            ViewLevel::Workflows { .. } => self.workflows.select_prev(),
            ViewLevel::Runs { .. } => self.runs.select_prev(),
            ViewLevel::Jobs { .. } => self.jobs.select_prev(),
            ViewLevel::Logs { .. } => {
                self.log_scroll_y = self.log_scroll_y.saturating_sub(1);
            }
        }
    }

    /// Handle down arrow key.
    pub fn select_next(&mut self) {
        match self.nav.current() {
            ViewLevel::Owners => self.owners.select_next(),
            ViewLevel::Repositories { .. } => self.repositories.select_next(),
            ViewLevel::Workflows { .. } => self.workflows.select_next(),
            ViewLevel::Runs { .. } => self.runs.select_next(),
            ViewLevel::Jobs { .. } => self.jobs.select_next(),
            ViewLevel::Logs { .. } => {
                self.log_scroll_y = self.log_scroll_y.saturating_add(1);
            }
        }
    }

    /// Handle left arrow key (horizontal scroll in logs).
    pub fn scroll_left(&mut self) {
        if matches!(self.nav.current(), ViewLevel::Logs { .. }) {
            self.log_scroll_x = self.log_scroll_x.saturating_sub(4);
        }
    }

    /// Handle right arrow key (horizontal scroll in logs).
    pub fn scroll_right(&mut self) {
        if matches!(self.nav.current(), ViewLevel::Logs { .. }) {
            self.log_scroll_x = self.log_scroll_x.saturating_add(4);
        }
    }

    /// Handle Page Up key (scroll logs by page).
    pub fn page_up(&mut self) {
        if matches!(self.nav.current(), ViewLevel::Logs { .. }) {
            self.log_scroll_y = self.log_scroll_y.saturating_sub(20);
        }
    }

    /// Handle Page Down key (scroll logs by page).
    pub fn page_down(&mut self) {
        if matches!(self.nav.current(), ViewLevel::Logs { .. }) {
            self.log_scroll_y = self.log_scroll_y.saturating_add(20);
        }
    }

    /// Scroll to start of logs.
    pub fn scroll_to_start(&mut self) {
        if matches!(self.nav.current(), ViewLevel::Logs { .. }) {
            self.log_scroll_y = 0;
            self.log_scroll_x = 0;
        }
    }

    /// Scroll to end of logs.
    #[allow(clippy::collapsible_if)]
    pub fn scroll_to_end(&mut self) {
        if matches!(self.nav.current(), ViewLevel::Logs { .. }) {
            if let LoadingState::Loaded(logs) = &self.log_content {
                let line_count = logs.lines().count() as u16;
                self.log_scroll_y = line_count.saturating_sub(10);
            }
        }
    }

    /// Clear current list data (for refresh).
    pub fn clear_current(&mut self) {
        match self.nav.current() {
            ViewLevel::Owners => self.owners = SelectableList::new(),
            ViewLevel::Repositories { .. } => self.repositories = SelectableList::new(),
            ViewLevel::Workflows { .. } => self.workflows = SelectableList::new(),
            ViewLevel::Runs { .. } => self.runs = SelectableList::new(),
            ViewLevel::Jobs { .. } => self.jobs = SelectableList::new(),
            ViewLevel::Logs { .. } => {
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
        if let LoadingState::Loaded(_) = &self.log_content {
            if self.log_selection_cursor > 0 {
                self.log_selection_cursor -= 1;
                if !extend {
                    self.log_selection_anchor = self.log_selection_cursor;
                }
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
