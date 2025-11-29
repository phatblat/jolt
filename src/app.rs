// App state and main event loop.
// Manages tabs, navigation state, and keyboard input handling.

use std::collections::{HashMap, HashSet};
use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use serde::{Deserialize, Serialize};

use crate::cache;
use crate::github::GitHubClient;
use crate::state::{
    AnalysisSession, AnalyzeTabState, AnalyzeViewLevel, LoadingState, NavigationContext,
    NavigationStack, RunMetadata, RunnersNavStack, RunnersTabState, RunnersViewLevel, SourceTab,
    SyncTabState, ViewLevel, WorkflowsTabState,
};
use crate::ui;

/// Active tab in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tab {
    Runners,
    #[default]
    Workflows,
    Analyze,
    Sync,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Runners => "Runners",
            Tab::Workflows => "Workflows",
            Tab::Analyze => "Analyze",
            Tab::Sync => "Sync",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Tab::Runners => Tab::Workflows,
            Tab::Workflows => Tab::Analyze,
            Tab::Analyze => Tab::Sync,
            Tab::Sync => Tab::Runners,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Runners => Tab::Sync,
            Tab::Workflows => Tab::Runners,
            Tab::Analyze => Tab::Workflows,
            Tab::Sync => Tab::Analyze,
        }
    }
}

/// Saved view state for a log file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogViewState {
    /// Selection anchor line (0-indexed).
    pub selection_anchor: usize,
    /// Selection cursor line (0-indexed).
    pub selection_cursor: usize,
    /// Vertical scroll position.
    pub scroll_y: u16,
}

/// Persisted application state saved between sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    /// Last active tab.
    pub active_tab: Tab,
    /// Workflows tab navigation stack.
    #[serde(default)]
    pub workflows_nav: Option<NavigationStack>,
    /// Runners tab navigation stack.
    #[serde(default)]
    pub runners_nav: Option<RunnersNavStack>,
    /// Per-log view state, keyed by job_id.
    #[serde(default)]
    pub log_view_states: HashMap<u64, LogViewState>,
    /// Favorite owners (by login).
    #[serde(default)]
    pub favorite_owners: HashSet<String>,
    /// Favorite repositories (as "owner/repo").
    #[serde(default)]
    pub favorite_repos: HashSet<String>,
    /// Favorite workflows (as "owner/repo/workflow_id").
    #[serde(default)]
    pub favorite_workflows: HashSet<String>,
    /// Favorite runners (as "owner/repo/runner_name").
    #[serde(default)]
    pub favorite_runners: HashSet<String>,
    /// Branch history for workflows tab.
    #[serde(default)]
    pub branch_history: Vec<String>,
    /// Currently selected branch for workflows tab.
    #[serde(default)]
    pub current_branch: Option<String>,
}

impl PersistedState {
    /// Load persisted state from disk.
    #[allow(clippy::collapsible_if)]
    pub fn load() -> Self {
        if let Some(path) = cache::state_path() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str(&contents) {
                    return state;
                }
            }
        }
        Self::default()
    }

    /// Save persisted state to disk.
    pub fn save(&self) {
        if let Some(path) = cache::state_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&path, json);
            }
        }
    }
}

/// Main application state.
pub struct App {
    /// Currently active tab.
    pub active_tab: Tab,
    /// Whether the app should exit.
    pub should_quit: bool,
    /// Whether to show the help overlay.
    pub show_help: bool,
    /// Whether search input is active.
    pub search_active: bool,
    /// Current search query.
    pub search_query: String,
    /// Line numbers containing search matches.
    pub search_matches: Vec<usize>,
    /// Index of current match in search_matches.
    pub search_match_index: usize,
    /// GitHub API client (None if no token).
    pub github_client: Option<GitHubClient>,
    /// Workflows tab state.
    pub workflows: WorkflowsTabState,
    /// Runners tab state.
    pub runners: RunnersTabState,
    /// Analyze tab state.
    pub analyze: AnalyzeTabState,
    /// Sync tab state.
    pub sync: SyncTabState,
    /// Per-log view states, keyed by job_id.
    pub log_view_states: HashMap<u64, LogViewState>,
    /// When clipboard flash indicator should expire.
    pub clipboard_flash_until: Option<std::time::Instant>,
    /// Favorite owners.
    pub favorite_owners: HashSet<String>,
    /// Favorite repositories.
    pub favorite_repos: HashSet<String>,
    /// Favorite workflows.
    pub favorite_workflows: HashSet<String>,
    /// Favorite runners.
    pub favorite_runners: HashSet<String>,
}

impl App {
    pub fn new() -> Self {
        // Load persisted state from previous session
        let persisted = PersistedState::load();

        // Try to create GitHub client from env
        let github_client = match GitHubClient::from_env() {
            Ok(client) => Some(client),
            Err(e) => {
                // Will show error in console tab
                eprintln!("GitHub client error: {}", e);
                None
            }
        };

        // Create tab states and restore navigation if available
        let mut workflows = WorkflowsTabState::new();
        if let Some(nav) = persisted.workflows_nav {
            workflows.nav = nav;
        }
        workflows.branch_history = persisted.branch_history;
        workflows.current_branch = persisted.current_branch;

        let mut runners = RunnersTabState::new();
        if let Some(nav) = persisted.runners_nav {
            runners.nav = nav;
        }

        // Restore log view state for current job if viewing logs
        if let ViewLevel::Logs { job_id, .. } = workflows.nav.current() {
            if let Some(state) = persisted.log_view_states.get(job_id) {
                workflows.log_selection_anchor = state.selection_anchor;
                workflows.log_selection_cursor = state.selection_cursor;
                workflows.log_scroll_y = state.scroll_y;
            }
        }
        if let RunnersViewLevel::Logs { job_id, .. } = runners.nav.current() {
            if let Some(state) = persisted.log_view_states.get(job_id) {
                runners.log_selection_anchor = state.selection_anchor;
                runners.log_selection_cursor = state.selection_cursor;
                runners.log_scroll_y = state.scroll_y;
            }
        }

        Self {
            active_tab: persisted.active_tab,
            should_quit: false,
            show_help: false,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_index: 0,
            github_client,
            workflows,
            runners,
            analyze: AnalyzeTabState::new(),
            sync: SyncTabState::new(),
            log_view_states: persisted.log_view_states,
            clipboard_flash_until: None,
            favorite_owners: persisted.favorite_owners,
            favorite_repos: persisted.favorite_repos,
            favorite_workflows: persisted.favorite_workflows,
            favorite_runners: persisted.favorite_runners,
        }
    }

    /// Save application state for next session.
    pub fn save_state(&self) {
        // Save current log view state to the map
        let mut log_view_states = self.log_view_states.clone();

        // Save workflows log state if viewing logs
        if let ViewLevel::Logs { job_id, .. } = self.workflows.nav.current() {
            log_view_states.insert(
                *job_id,
                LogViewState {
                    selection_anchor: self.workflows.log_selection_anchor,
                    selection_cursor: self.workflows.log_selection_cursor,
                    scroll_y: self.workflows.log_scroll_y,
                },
            );
        }

        // Save runners log state if viewing logs
        if let RunnersViewLevel::Logs { job_id, .. } = self.runners.nav.current() {
            log_view_states.insert(
                *job_id,
                LogViewState {
                    selection_anchor: self.runners.log_selection_anchor,
                    selection_cursor: self.runners.log_selection_cursor,
                    scroll_y: self.runners.log_scroll_y,
                },
            );
        }

        let state = PersistedState {
            active_tab: self.active_tab,
            workflows_nav: Some(self.workflows.nav.clone()),
            runners_nav: Some(self.runners.nav.clone()),
            log_view_states,
            favorite_owners: self.favorite_owners.clone(),
            favorite_repos: self.favorite_repos.clone(),
            favorite_workflows: self.favorite_workflows.clone(),
            favorite_runners: self.favorite_runners.clone(),
            branch_history: self.workflows.branch_history.clone(),
            current_branch: self.workflows.current_branch.clone(),
        };
        state.save();
    }

    /// Save current log view state for the active job.
    fn save_current_log_state(&mut self) {
        match self.active_tab {
            Tab::Workflows => {
                if let ViewLevel::Logs { job_id, .. } = self.workflows.nav.current() {
                    self.log_view_states.insert(
                        *job_id,
                        LogViewState {
                            selection_anchor: self.workflows.log_selection_anchor,
                            selection_cursor: self.workflows.log_selection_cursor,
                            scroll_y: self.workflows.log_scroll_y,
                        },
                    );
                }
            }
            Tab::Runners => {
                if let RunnersViewLevel::Logs { job_id, .. } = self.runners.nav.current() {
                    self.log_view_states.insert(
                        *job_id,
                        LogViewState {
                            selection_anchor: self.runners.log_selection_anchor,
                            selection_cursor: self.runners.log_selection_cursor,
                            scroll_y: self.runners.log_scroll_y,
                        },
                    );
                }
            }
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Restore log view state for a job if previously saved.
    fn restore_log_state(&mut self, job_id: u64) {
        if let Some(state) = self.log_view_states.get(&job_id) {
            match self.active_tab {
                Tab::Workflows => {
                    self.workflows.log_selection_anchor = state.selection_anchor;
                    self.workflows.log_selection_cursor = state.selection_cursor;
                    self.workflows.log_scroll_y = state.scroll_y;
                }
                Tab::Runners => {
                    self.runners.log_selection_anchor = state.selection_anchor;
                    self.runners.log_selection_cursor = state.selection_cursor;
                    self.runners.log_scroll_y = state.scroll_y;
                }
                Tab::Analyze | Tab::Sync => {}
            }
        }
    }

    /// Main event loop.
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        // Initial data load for active tab
        self.on_tab_change().await;

        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, self))?;
            self.handle_events().await?;
        }

        // Save state for next session
        self.save_state();
        Ok(())
    }

    /// Handle keyboard and other events.
    #[allow(clippy::collapsible_if)]
    async fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // When help is shown, only handle close keys
                    if self.show_help {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                                self.show_help = false;
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    // When search input is active, capture text input
                    if self.search_active {
                        match key.code {
                            KeyCode::Esc => {
                                self.search_active = false;
                                self.search_query.clear();
                                self.search_matches.clear();
                            }
                            KeyCode::Enter => {
                                self.search_active = false;
                                self.execute_search();
                            }
                            KeyCode::Backspace => {
                                self.search_query.pop();
                            }
                            KeyCode::Char(c) => {
                                self.search_query.push(c);
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    // When branch modal is active, handle input
                    if self.active_tab == Tab::Workflows && self.workflows.branch_modal_visible {
                        match key.code {
                            KeyCode::Esc => {
                                self.workflows.branch_modal_visible = false;
                                self.workflows.branch_input.clear();
                                self.workflows.branch_history_selection = 0;
                            }
                            KeyCode::Enter => {
                                self.handle_branch_switch().await;
                            }
                            KeyCode::Up => {
                                if !self.workflows.branch_history.is_empty() {
                                    if self.workflows.branch_history_selection > 0 {
                                        self.workflows.branch_history_selection -= 1;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if !self.workflows.branch_history.is_empty() {
                                    let max = self.workflows.branch_history.len() - 1;
                                    if self.workflows.branch_history_selection < max {
                                        self.workflows.branch_history_selection += 1;
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                self.workflows.branch_input.pop();
                            }
                            KeyCode::Char(c) => {
                                self.workflows.branch_input.push(c);
                            }
                            _ => {}
                        }
                        return Ok(());
                    }

                    // Handle Ctrl modifier keys first
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        let shift_held = key.modifiers.contains(KeyModifiers::SHIFT);
                        match key.code {
                            KeyCode::Char('d') => self.handle_page_down(shift_held),
                            KeyCode::Char('u') => self.handle_page_up(shift_held),
                            KeyCode::Char('f') => self.handle_page_down(shift_held),
                            KeyCode::Char('b') => self.handle_page_up(shift_held),
                            _ => {}
                        }
                        return Ok(());
                    }

                    // Check if shift is held for selection extension
                    let shift_held = key.modifiers.contains(KeyModifiers::SHIFT);

                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('?') => self.show_help = true,
                        KeyCode::Tab => {
                            self.active_tab = self.active_tab.next();
                            self.on_tab_change().await;
                        }
                        KeyCode::BackTab => {
                            self.active_tab = self.active_tab.prev();
                            self.on_tab_change().await;
                        }
                        // Global sync toggle (Shift+S)
                        KeyCode::Char('S') => {
                            self.toggle_sync();
                        }
                        // Direct tab selection
                        KeyCode::Char('1') => {
                            self.active_tab = Tab::Runners;
                            self.on_tab_change().await;
                        }
                        KeyCode::Char('2') => {
                            self.active_tab = Tab::Workflows;
                            self.on_tab_change().await;
                        }
                        KeyCode::Char('3') => {
                            self.active_tab = Tab::Analyze;
                            self.on_tab_change().await;
                        }
                        KeyCode::Char('4') => {
                            self.active_tab = Tab::Sync;
                            self.on_tab_change().await;
                        }
                        // Arrow keys
                        KeyCode::Up => self.handle_up(shift_held),
                        KeyCode::Down => self.handle_down(shift_held),
                        KeyCode::Left => self.handle_left(),
                        KeyCode::Right => self.handle_right(),
                        // Vim navigation
                        KeyCode::Char('k') => self.handle_up(shift_held),
                        KeyCode::Char('j') => self.handle_down(shift_held),
                        KeyCode::Char('K') => self.handle_up(true), // Uppercase extends selection
                        KeyCode::Char('J') => self.handle_down(true), // Uppercase extends selection
                        KeyCode::Char('h') => self.handle_left(),
                        KeyCode::Char('l') => self.handle_right(),
                        // Page navigation
                        KeyCode::PageUp => self.handle_page_up(shift_held),
                        KeyCode::PageDown => self.handle_page_down(shift_held),
                        // Jump to start/end
                        KeyCode::Home => self.handle_home(shift_held),
                        KeyCode::End => self.handle_end(shift_held),
                        KeyCode::Char('g') => self.handle_home(false), // Vim: go to start
                        KeyCode::Char('G') => self.handle_end(false),  // Vim: go to end
                        // Actions
                        KeyCode::Enter => self.handle_enter().await,
                        KeyCode::Esc => self.handle_escape().await,
                        KeyCode::Char('r') => self.handle_refresh().await,
                        KeyCode::Char('/') => self.handle_search_start(),
                        KeyCode::Char('o') => self.handle_open_in_browser(),
                        KeyCode::Char('f') => self.toggle_favorite(),
                        KeyCode::Char('c') => self.copy_selection(),
                        KeyCode::Char('a') => self.save_to_analyze(),
                        KeyCode::Char('b') => self.handle_branch_modal_open(),
                        // Search navigation
                        KeyCode::Char('n') => self.search_next(),
                        KeyCode::Char('N') => self.search_prev(),
                        _ => {}
                    }
                }
            }
        }

        // Check if runners list needs auto-refresh
        if self.active_tab == Tab::Runners {
            if let RunnersViewLevel::Runners {
                ref owner,
                ref repo,
            } = self.runners.nav.current().clone()
            {
                if let Some(next_refresh) = self.runners.runners_next_refresh {
                    if std::time::Instant::now() >= next_refresh {
                        // Time to refresh - force reload
                        self.runners.runners.set_loading();
                        let owner = owner.clone();
                        let repo = repo.clone();
                        let result = self
                            .github_client
                            .as_mut()
                            .unwrap()
                            .get_enriched_runners(&owner, &repo, 1, 30)
                            .await;
                        match result {
                            Ok((runners, count)) => {
                                self.runners.runners.set_loaded(runners, count);
                            }
                            Err(e) => {
                                self.runners.runners.set_error(e.to_string());
                            }
                        }
                        // Schedule next refresh
                        self.runners.runners_next_refresh =
                            Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle up arrow key.
    fn handle_up(&mut self, shift_held: bool) {
        match self.active_tab {
            Tab::Workflows => {
                if matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    self.workflows.selection_up(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.workflows.select_prev();
                }
            }
            Tab::Runners => {
                if matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    self.runners.selection_up(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.runners.select_prev();
                }
            }
            Tab::Analyze => {
                if matches!(self.analyze.view, AnalyzeViewLevel::List) {
                    self.analyze.select_prev();
                } else {
                    self.analyze.scroll_up();
                }
            }
            Tab::Sync => self.sync.select_prev(),
        }
    }

    /// Handle down arrow key.
    fn handle_down(&mut self, shift_held: bool) {
        match self.active_tab {
            Tab::Workflows => {
                if matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    self.workflows.selection_down(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.workflows.select_next();
                }
            }
            Tab::Runners => {
                if matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    self.runners.selection_down(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.runners.select_next();
                }
            }
            Tab::Analyze => {
                if matches!(self.analyze.view, AnalyzeViewLevel::List) {
                    self.analyze.select_next();
                } else {
                    self.analyze.scroll_down();
                }
            }
            Tab::Sync => self.sync.select_next(),
        }
    }

    /// Handle left arrow key.
    fn handle_left(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.scroll_left(),
            Tab::Runners => self.runners.scroll_left(),
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Handle right arrow key.
    fn handle_right(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.scroll_right(),
            Tab::Runners => self.runners.scroll_right(),
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Handle Page Up key.
    fn handle_page_up(&mut self, shift_held: bool) {
        match self.active_tab {
            Tab::Workflows => {
                if matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    self.workflows.selection_page_up(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.workflows.page_up();
                }
            }
            Tab::Runners => {
                if matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    self.runners.selection_page_up(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.runners.page_up();
                }
            }
            Tab::Analyze => self.analyze.page_up(),
            Tab::Sync => {}
        }
    }

    /// Handle Page Down key.
    fn handle_page_down(&mut self, shift_held: bool) {
        match self.active_tab {
            Tab::Workflows => {
                if matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    self.workflows.selection_page_down(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.workflows.page_down();
                }
            }
            Tab::Runners => {
                if matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    self.runners.selection_page_down(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.runners.page_down();
                }
            }
            Tab::Analyze => self.analyze.page_down(),
            Tab::Sync => {}
        }
    }

    /// Handle Home key.
    fn handle_home(&mut self, shift_held: bool) {
        match self.active_tab {
            Tab::Workflows => {
                if matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    self.workflows.selection_to_start(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.workflows.scroll_to_start();
                }
            }
            Tab::Runners => {
                if matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    self.runners.selection_to_start(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.runners.scroll_to_start();
                }
            }
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Handle End key.
    fn handle_end(&mut self, shift_held: bool) {
        match self.active_tab {
            Tab::Workflows => {
                if matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    self.workflows.selection_to_end(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.workflows.scroll_to_end();
                }
            }
            Tab::Runners => {
                if matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    self.runners.selection_to_end(shift_held);
                    self.scroll_to_selection();
                } else {
                    self.runners.scroll_to_end();
                }
            }
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Handle search start (/ key).
    fn handle_search_start(&mut self) {
        // Only activate search when viewing logs
        let in_logs = match self.active_tab {
            Tab::Workflows => matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }),
            Tab::Runners => matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }),
            Tab::Analyze | Tab::Sync => false,
        };
        if in_logs {
            self.search_active = true;
            self.search_query.clear();
            self.search_matches.clear();
            self.search_match_index = 0;
        }
    }

    /// Execute search on current log content.
    fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_matches.clear();
            return;
        }

        let logs = match self.active_tab {
            Tab::Workflows => {
                if let LoadingState::Loaded(ref logs) = self.workflows.log_content {
                    logs.clone()
                } else {
                    return;
                }
            }
            Tab::Runners => {
                if let LoadingState::Loaded(ref logs) = self.runners.log_content {
                    logs.clone()
                } else {
                    return;
                }
            }
            Tab::Analyze | Tab::Sync => return,
        };

        // Find all matching line numbers (0-indexed)
        let query_lower = self.search_query.to_lowercase();
        self.search_matches = logs
            .lines()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&query_lower))
            .map(|(i, _)| i)
            .collect();

        // Jump to first match if any
        if !self.search_matches.is_empty() {
            self.search_match_index = 0;
            self.scroll_to_match();
        }
    }

    /// Navigate to next search match.
    fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_index = (self.search_match_index + 1) % self.search_matches.len();
        self.scroll_to_match();
    }

    /// Navigate to previous search match.
    fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_match_index == 0 {
            self.search_match_index = self.search_matches.len() - 1;
        } else {
            self.search_match_index -= 1;
        }
        self.scroll_to_match();
    }

    /// Scroll log view to current search match.
    fn scroll_to_match(&mut self) {
        if let Some(&line) = self.search_matches.get(self.search_match_index) {
            match self.active_tab {
                Tab::Workflows => {
                    self.workflows.log_scroll_y = line as u16;
                }
                Tab::Runners => {
                    self.runners.log_scroll_y = line as u16;
                }
                Tab::Analyze | Tab::Sync => {}
            }
        }
    }

    /// Scroll log view to keep selection cursor visible.
    fn scroll_to_selection(&mut self) {
        // Approximate visible lines (will be refined when we have actual area height)
        const VISIBLE_LINES: u16 = 20;

        match self.active_tab {
            Tab::Workflows => {
                let cursor = self.workflows.log_selection_cursor as u16;
                let scroll_y = self.workflows.log_scroll_y;

                // Scroll up if cursor is above visible area
                if cursor < scroll_y {
                    self.workflows.log_scroll_y = cursor;
                }
                // Scroll down if cursor is below visible area
                else if cursor >= scroll_y + VISIBLE_LINES {
                    self.workflows.log_scroll_y = cursor.saturating_sub(VISIBLE_LINES - 1);
                }
            }
            Tab::Runners => {
                let cursor = self.runners.log_selection_cursor as u16;
                let scroll_y = self.runners.log_scroll_y;

                if cursor < scroll_y {
                    self.runners.log_scroll_y = cursor;
                } else if cursor >= scroll_y + VISIBLE_LINES {
                    self.runners.log_scroll_y = cursor.saturating_sub(VISIBLE_LINES - 1);
                }
            }
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Open the current item in GitHub in the browser.
    fn handle_open_in_browser(&mut self) {
        let url = match self.active_tab {
            Tab::Workflows => self.get_workflows_github_url(),
            Tab::Runners => self.get_runners_github_url(),
            Tab::Analyze => {
                // Open GitHub URL for selected analysis session
                self.analyze
                    .selected_session()
                    .map(|s| s.github_url.clone())
            }
            Tab::Sync => None,
        };

        #[allow(clippy::collapsible_if)]
        if let Some(url) = url {
            if let Err(e) = std::process::Command::new("open").arg(&url).spawn() {
                self.log_error(format!("Failed to open browser: {}", e));
            }
        }
    }

    /// Copy selected log lines to macOS clipboard.
    fn copy_selection(&mut self) {
        let selected_text = match self.active_tab {
            Tab::Workflows => {
                if !matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }) {
                    return;
                }
                let logs = match &self.workflows.log_content {
                    LoadingState::Loaded(logs) => logs,
                    _ => return,
                };
                let (start, end) = self.workflows.log_selection_range();
                logs.lines()
                    .skip(start)
                    .take(end - start + 1)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Tab::Runners => {
                if !matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }) {
                    return;
                }
                let logs = match &self.runners.log_content {
                    LoadingState::Loaded(logs) => logs,
                    _ => return,
                };
                let (start, end) = self.runners.log_selection_range();
                logs.lines()
                    .skip(start)
                    .take(end - start + 1)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Tab::Analyze => {
                // Copy log excerpt from selected analysis session
                match self.analyze.selected_session() {
                    Some(session) => session.log_excerpt.clone(),
                    None => return,
                }
            }
            Tab::Sync => return,
        };

        // Copy to macOS clipboard using pbcopy
        if let Ok(mut child) = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                let _ = stdin.write_all(selected_text.as_bytes());
            }
            if child.wait().is_ok() {
                // Flash clipboard indicator for 1.5 seconds
                self.clipboard_flash_until =
                    Some(std::time::Instant::now() + Duration::from_millis(1500));
            }
        }
    }

    /// Save current log selection to the Analyze tab.
    fn save_to_analyze(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.save_workflows_to_analyze(),
            Tab::Runners => self.save_runners_to_analyze(),
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Save from Workflows tab log viewer to Analyze.
    fn save_workflows_to_analyze(&mut self) {
        // Extract context from current view level
        let (
            owner,
            repo,
            workflow_id,
            workflow_name,
            run_id,
            run_number,
            job_id,
            job_name,
            job_status,
            job_conclusion,
        ) = match self.workflows.nav.current() {
            ViewLevel::Logs {
                owner,
                repo,
                workflow_id,
                run_id,
                job_id,
                job_name,
                job_status,
                job_conclusion,
            } => {
                // Get run_number and workflow_name from navigation stack
                let (run_number, workflow_name) = self.get_workflows_run_context();
                (
                    owner.clone(),
                    repo.clone(),
                    Some(*workflow_id),
                    workflow_name,
                    *run_id,
                    run_number,
                    *job_id,
                    job_name.clone(),
                    *job_status,
                    *job_conclusion,
                )
            }
            _ => return, // Not in log view
        };

        // Get log content
        let logs = match &self.workflows.log_content {
            LoadingState::Loaded(logs) => logs,
            _ => return,
        };

        // Get selection range
        let (sel_start, sel_end) = self.workflows.log_selection_range();

        // Check for existing session that overlaps with selected lines
        if let Some(existing) = self
            .analyze
            .find_overlapping(job_id, run_id, sel_start, sel_end)
        {
            let session_id = existing.id.clone();
            self.sync
                .log_info("Selection overlaps existing session - navigating");
            self.active_tab = Tab::Analyze;
            self.analyze.enter_detail_by_id(&session_id);
            return;
        }

        let total_lines = logs.lines().count();

        // Extract selected lines
        let log_excerpt: String = logs
            .lines()
            .skip(sel_start)
            .take(sel_end - sel_start + 1)
            .collect::<Vec<_>>()
            .join("\n");

        // Get run metadata
        let run_metadata = self.get_workflows_run_metadata();

        // Build navigation context
        let nav_context = NavigationContext {
            source_tab: SourceTab::Workflows,
            owner: owner.clone(),
            repo: repo.clone(),
            workflow_id,
            workflow_name,
            run_id,
            run_number,
            job_id,
            job_name,
            job_status,
            job_conclusion,
            scroll_to_line: sel_start,
            selection_anchor: self.workflows.log_selection_anchor,
            selection_cursor: self.workflows.log_selection_cursor,
        };

        // Build GitHub URL
        let github_url = format!(
            "https://github.com/{}/{}/actions/runs/{}/job/{}",
            owner, repo, run_id, job_id
        );

        // Create and add session
        let session = AnalysisSession::new(
            nav_context,
            run_metadata,
            github_url,
            log_excerpt,
            total_lines,
            sel_start,
            sel_end,
        );

        let session_id = session.id.clone();
        self.analyze.add_session(session);
        self.sync.log_info(format!(
            "Saved {} lines to Analyze",
            sel_end - sel_start + 1
        ));

        // Switch to Analyze tab and show detail view
        self.active_tab = Tab::Analyze;
        self.analyze.enter_detail_by_id(&session_id);
    }

    /// Save from Runners tab log viewer to Analyze.
    fn save_runners_to_analyze(&mut self) {
        // Extract context from current view level
        let (owner, repo, run_id, job_id, job_name, job_status, job_conclusion) =
            match self.runners.nav.current() {
                RunnersViewLevel::Logs {
                    owner,
                    repo,
                    run_id,
                    job_id,
                    job_name,
                    job_status,
                    job_conclusion,
                } => (
                    owner.clone(),
                    repo.clone(),
                    *run_id,
                    *job_id,
                    job_name.clone(),
                    *job_status,
                    *job_conclusion,
                ),
                _ => return, // Not in log view
            };

        // Get log content
        let logs = match &self.runners.log_content {
            LoadingState::Loaded(logs) => logs,
            _ => return,
        };

        // Get selection range
        let (sel_start, sel_end) = self.runners.log_selection_range();

        // Check for existing session that overlaps with selected lines
        if let Some(existing) = self
            .analyze
            .find_overlapping(job_id, run_id, sel_start, sel_end)
        {
            let session_id = existing.id.clone();
            self.sync
                .log_info("Selection overlaps existing session - navigating");
            self.active_tab = Tab::Analyze;
            self.analyze.enter_detail_by_id(&session_id);
            return;
        }

        let total_lines = logs.lines().count();

        // Extract selected lines
        let log_excerpt: String = logs
            .lines()
            .skip(sel_start)
            .take(sel_end - sel_start + 1)
            .collect::<Vec<_>>()
            .join("\n");

        // Get run metadata
        let run_metadata = self.get_runners_run_metadata();

        // Get run_number from navigation stack
        let run_number = self.get_runners_run_number();

        // Build navigation context
        let nav_context = NavigationContext {
            source_tab: SourceTab::Runners,
            owner: owner.clone(),
            repo: repo.clone(),
            workflow_id: None,
            workflow_name: None,
            run_id,
            run_number,
            job_id,
            job_name,
            job_status,
            job_conclusion,
            scroll_to_line: sel_start,
            selection_anchor: self.runners.log_selection_anchor,
            selection_cursor: self.runners.log_selection_cursor,
        };

        // Build GitHub URL
        let github_url = format!(
            "https://github.com/{}/{}/actions/runs/{}/job/{}",
            owner, repo, run_id, job_id
        );

        // Create and add session
        let session = AnalysisSession::new(
            nav_context,
            run_metadata,
            github_url,
            log_excerpt,
            total_lines,
            sel_start,
            sel_end,
        );

        let session_id = session.id.clone();
        self.analyze.add_session(session);
        self.sync.log_info(format!(
            "Saved {} lines to Analyze",
            sel_end - sel_start + 1
        ));

        // Switch to Analyze tab and show detail view
        self.active_tab = Tab::Analyze;
        self.analyze.enter_detail_by_id(&session_id);
    }

    /// Get run context (run_number, workflow_name) from Workflows nav stack.
    fn get_workflows_run_context(&self) -> (u64, Option<String>) {
        let breadcrumbs = self.workflows.nav.breadcrumbs();
        let mut run_number = 0u64;
        let mut workflow_name = None;

        for node in &breadcrumbs {
            match &node.level {
                ViewLevel::Runs {
                    workflow_name: wn, ..
                } => {
                    workflow_name = Some(wn.clone());
                }
                ViewLevel::Jobs { run_number: rn, .. } => {
                    run_number = *rn;
                }
                _ => {}
            }
        }

        (run_number, workflow_name)
    }

    /// Get run_number from Runners nav stack.
    fn get_runners_run_number(&self) -> u64 {
        let breadcrumbs = self.runners.nav.breadcrumbs();
        for node in &breadcrumbs {
            if let RunnersViewLevel::Jobs { run_number, .. } = &node.level {
                return *run_number;
            }
        }
        0
    }

    /// Get run metadata for Workflows tab.
    fn get_workflows_run_metadata(&self) -> RunMetadata {
        // Try to get from loaded runs data
        let (pr_number, branch_name, commit_sha) = self
            .workflows
            .runs
            .data
            .data()
            .and_then(|data| {
                // Find matching run by checking current job's run_id
                if let ViewLevel::Logs { run_id, .. } = self.workflows.nav.current() {
                    data.items.iter().find(|r| r.id == *run_id).map(|run| {
                        let pr = run.pull_requests.first().map(|pr| pr.number);
                        let branch = run.head_branch.clone();
                        let sha = run.head_sha[..7.min(run.head_sha.len())].to_string();
                        (pr, branch, sha)
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None, "unknown".to_string()));

        // Try to get runner info from current job
        let (runner_name, runner_labels) = self
            .workflows
            .jobs
            .data
            .data()
            .and_then(|data| {
                if let ViewLevel::Logs { job_id, .. } = self.workflows.nav.current() {
                    data.items
                        .iter()
                        .find(|j| j.id == *job_id)
                        .map(|job| (job.runner_name.clone(), Vec::new()))
                } else {
                    None
                }
            })
            .unwrap_or((None, Vec::new()));

        RunMetadata {
            pr_number,
            branch_name,
            commit_sha,
            author: None, // Not available in current data
            runner_name,
            runner_labels,
        }
    }

    /// Get run metadata for Runners tab.
    fn get_runners_run_metadata(&self) -> RunMetadata {
        // Try to get from loaded runs data
        let (pr_number, branch_name, commit_sha) = self
            .runners
            .runs
            .data
            .data()
            .and_then(|data| {
                // Find matching run by checking current job's run_id
                if let RunnersViewLevel::Logs { run_id, .. } = self.runners.nav.current() {
                    data.items.iter().find(|r| r.id == *run_id).map(|run| {
                        let pr = run.pull_requests.first().map(|pr| pr.number);
                        let branch = run.head_branch.clone();
                        let sha = run.head_sha[..7.min(run.head_sha.len())].to_string();
                        (pr, branch, sha)
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None, "unknown".to_string()));

        // Try to get runner info from current job
        let (runner_name, runner_labels) = self
            .runners
            .jobs
            .data
            .data()
            .and_then(|data| {
                if let RunnersViewLevel::Logs { job_id, .. } = self.runners.nav.current() {
                    data.items
                        .iter()
                        .find(|j| j.id == *job_id)
                        .map(|job| (job.runner_name.clone(), Vec::new()))
                } else {
                    None
                }
            })
            .unwrap_or((None, Vec::new()));

        RunMetadata {
            pr_number,
            branch_name,
            commit_sha,
            author: None,
            runner_name,
            runner_labels,
        }
    }

    /// Navigate to the source log for an analysis session.
    async fn go_to_source(&mut self, session_id: &str) {
        let session = match self.analyze.find_session(session_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let ctx = &session.nav_context;

        match ctx.source_tab {
            SourceTab::Workflows => {
                self.go_to_workflows_source(&session).await;
            }
            SourceTab::Runners => {
                self.go_to_runners_source(&session).await;
            }
        }
    }

    /// Navigate to Workflows tab source for an analysis session.
    async fn go_to_workflows_source(&mut self, session: &AnalysisSession) {
        let ctx = &session.nav_context;

        // Build the navigation stack
        let workflow_id = ctx.workflow_id.unwrap_or(0);
        let workflow_name = ctx.workflow_name.clone().unwrap_or_default();

        // Reset workflows state and build nav stack
        self.workflows.nav = NavigationStack::new(ViewLevel::Owners);
        self.workflows.nav.push(ViewLevel::Repositories {
            owner: ctx.owner.clone(),
        });
        self.workflows.nav.push(ViewLevel::Workflows {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
        });
        self.workflows.nav.push(ViewLevel::Runs {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
            workflow_id,
            workflow_name,
        });
        self.workflows.nav.push(ViewLevel::Jobs {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
            workflow_id,
            run_id: ctx.run_id,
            run_number: ctx.run_number,
        });
        self.workflows.nav.push(ViewLevel::Logs {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
            workflow_id,
            run_id: ctx.run_id,
            job_id: ctx.job_id,
            job_name: ctx.job_name.clone(),
            job_status: ctx.job_status,
            job_conclusion: ctx.job_conclusion,
        });

        // Reset log state and restore selection from session
        self.workflows.log_content = LoadingState::Idle;
        self.workflows.log_selection_anchor = ctx.selection_anchor;
        self.workflows.log_selection_cursor = ctx.selection_cursor;
        self.workflows.log_scroll_y = ctx.scroll_to_line as u16;
        self.workflows.log_scroll_x = 0;

        // Switch tab and load logs
        self.active_tab = Tab::Workflows;
        self.analyze.exit_detail();
        self.load_current_view().await;
    }

    /// Navigate to Runners tab source for an analysis session.
    async fn go_to_runners_source(&mut self, session: &AnalysisSession) {
        let ctx = &session.nav_context;

        // Reset runners state and build nav stack
        self.runners.nav = RunnersNavStack::default();
        self.runners.nav.push(RunnersViewLevel::Runners {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
        });
        self.runners.nav.push(RunnersViewLevel::Runs {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
            runner_name: session.run_metadata.runner_name.clone(),
        });
        self.runners.nav.push(RunnersViewLevel::Jobs {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
            run_id: ctx.run_id,
            run_number: ctx.run_number,
        });
        self.runners.nav.push(RunnersViewLevel::Logs {
            owner: ctx.owner.clone(),
            repo: ctx.repo.clone(),
            run_id: ctx.run_id,
            job_id: ctx.job_id,
            job_name: ctx.job_name.clone(),
            job_status: ctx.job_status,
            job_conclusion: ctx.job_conclusion,
        });

        // Reset log state and restore selection from session
        self.runners.log_content = LoadingState::Idle;
        self.runners.log_selection_anchor = ctx.selection_anchor;
        self.runners.log_selection_cursor = ctx.selection_cursor;
        self.runners.log_scroll_y = ctx.scroll_to_line as u16;
        self.runners.log_scroll_x = 0;

        // Switch tab and load logs
        self.active_tab = Tab::Runners;
        self.analyze.exit_detail();
        self.load_current_view().await;
    }

    /// Toggle favorite status for the currently selected item.
    fn toggle_favorite(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.toggle_workflows_favorite(),
            Tab::Runners => self.toggle_runners_favorite(),
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Toggle favorite in Workflows tab.
    fn toggle_workflows_favorite(&mut self) {
        match self.workflows.nav.current().clone() {
            ViewLevel::Owners => {
                // Get selected index and sort data the same way as rendering
                let index = match self.workflows.owners.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.workflows.owners.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fav = self.favorite_owners.contains(&a.login);
                    let b_fav = self.favorite_owners.contains(&b.login);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.login.cmp(&b.login),
                    }
                });
                if let Some(owner) = sorted.get(index) {
                    let key = owner.login.clone();
                    if self.favorite_owners.contains(&key) {
                        self.favorite_owners.remove(&key);
                    } else {
                        self.favorite_owners.insert(key);
                    }
                }
            }
            ViewLevel::Repositories { ref owner } => {
                let index = match self.workflows.repositories.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.workflows.repositories.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", owner, a.name);
                    let b_key = format!("{}/{}", owner, b.name);
                    let a_fav = self.favorite_repos.contains(&a_key);
                    let b_fav = self.favorite_repos.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                if let Some(repo) = sorted.get(index) {
                    let key = format!("{}/{}", owner, repo.name);
                    if self.favorite_repos.contains(&key) {
                        self.favorite_repos.remove(&key);
                    } else {
                        self.favorite_repos.insert(key);
                    }
                }
            }
            ViewLevel::Workflows {
                ref owner,
                ref repo,
            } => {
                let index = match self.workflows.workflows.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.workflows.workflows.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                let repo = repo.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.id);
                    let b_key = format!("{}/{}/{}", owner, repo, b.id);
                    let a_fav = self.favorite_workflows.contains(&a_key);
                    let b_fav = self.favorite_workflows.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                if let Some(workflow) = sorted.get(index) {
                    let key = format!("{}/{}/{}", owner, repo, workflow.id);
                    if self.favorite_workflows.contains(&key) {
                        self.favorite_workflows.remove(&key);
                    } else {
                        self.favorite_workflows.insert(key);
                    }
                }
            }
            _ => {} // Can't favorite runs, jobs, or logs
        }
    }

    /// Toggle favorite in Runners tab.
    fn toggle_runners_favorite(&mut self) {
        match self.runners.nav.current().clone() {
            RunnersViewLevel::Repositories => {
                let index = match self.runners.repositories.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.runners.repositories.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", a.owner.login, a.name);
                    let b_key = format!("{}/{}", b.owner.login, b.name);
                    let a_fav = self.favorite_repos.contains(&a_key);
                    let b_fav = self.favorite_repos.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a_key.cmp(&b_key),
                    }
                });
                if let Some(repo) = sorted.get(index) {
                    let key = format!("{}/{}", repo.owner.login, repo.name);
                    if self.favorite_repos.contains(&key) {
                        self.favorite_repos.remove(&key);
                    } else {
                        self.favorite_repos.insert(key);
                    }
                }
            }
            RunnersViewLevel::Runners {
                ref owner,
                ref repo,
            } => {
                let index = match self.runners.runners.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.runners.runners.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                let repo = repo.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.runner.name);
                    let b_key = format!("{}/{}/{}", owner, repo, b.runner.name);
                    let a_fav = self.favorite_runners.contains(&a_key);
                    let b_fav = self.favorite_runners.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.runner.name.cmp(&b.runner.name),
                    }
                });
                if let Some(enriched) = sorted.get(index) {
                    let key = format!("{}/{}/{}", owner, repo, enriched.runner.name);
                    if self.favorite_runners.contains(&key) {
                        self.favorite_runners.remove(&key);
                    } else {
                        self.favorite_runners.insert(key);
                    }
                }
            }
            _ => {} // Can't favorite runs, jobs, or logs
        }
    }

    /// Get GitHub URL for current Workflows tab view.
    fn get_workflows_github_url(&self) -> Option<String> {
        match self.workflows.nav.current().clone() {
            ViewLevel::Owners => {
                let index = self.workflows.owners.selected()?;
                let data = self.workflows.owners.data.data()?;
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fav = self.favorite_owners.contains(&a.login);
                    let b_fav = self.favorite_owners.contains(&b.login);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.login.cmp(&b.login),
                    }
                });
                sorted
                    .get(index)
                    .map(|owner| format!("https://github.com/{}", owner.login))
            }
            ViewLevel::Repositories { ref owner } => {
                let index = self.workflows.repositories.selected()?;
                let data = self.workflows.repositories.data.data()?;
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", owner, a.name);
                    let b_key = format!("{}/{}", owner, b.name);
                    let a_fav = self.favorite_repos.contains(&a_key);
                    let b_fav = self.favorite_repos.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                sorted
                    .get(index)
                    .map(|repo| format!("https://github.com/{}/{}", owner, repo.name))
            }
            ViewLevel::Workflows {
                ref owner,
                ref repo,
            } => {
                let index = self.workflows.workflows.selected()?;
                let data = self.workflows.workflows.data.data()?;
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                let repo = repo.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.id);
                    let b_key = format!("{}/{}/{}", owner, repo, b.id);
                    let a_fav = self.favorite_workflows.contains(&a_key);
                    let b_fav = self.favorite_workflows.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                sorted.get(index).map(|workflow| {
                    format!(
                        "https://github.com/{}/{}/actions/workflows/{}",
                        owner,
                        repo,
                        workflow.path.rsplit('/').next().unwrap_or(&workflow.path)
                    )
                })
            }
            ViewLevel::Runs { owner, repo, .. } => self.workflows.runs.selected_item().map(|run| {
                format!(
                    "https://github.com/{}/{}/actions/runs/{}",
                    owner, repo, run.id
                )
            }),
            ViewLevel::Jobs {
                owner,
                repo,
                run_id,
                ..
            } => {
                // Use flattened list to get the selected job (main or sub-item)
                if let Some(index) = self.workflows.jobs.selected() {
                    if let Some(list_item) = self.workflows.job_list_items.get(index) {
                        let job = list_item.get_job(&self.workflows.job_groups);
                        Some(format!(
                            "https://github.com/{}/{}/actions/runs/{}/job/{}",
                            owner, repo, run_id, job.id
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ViewLevel::Logs {
                owner,
                repo,
                run_id,
                job_id,
                ..
            } => Some(format!(
                "https://github.com/{}/{}/actions/runs/{}/job/{}",
                owner, repo, run_id, job_id
            )),
        }
    }

    /// Get GitHub URL for current Runners tab view.
    fn get_runners_github_url(&self) -> Option<String> {
        match self.runners.nav.current().clone() {
            RunnersViewLevel::Repositories => {
                let index = self.runners.repositories.selected()?;
                let data = self.runners.repositories.data.data()?;
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", a.owner.login, a.name);
                    let b_key = format!("{}/{}", b.owner.login, b.name);
                    let a_fav = self.favorite_repos.contains(&a_key);
                    let b_fav = self.favorite_repos.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a_key.cmp(&b_key),
                    }
                });
                sorted
                    .get(index)
                    .map(|repo| format!("https://github.com/{}/{}", repo.owner.login, repo.name))
            }
            RunnersViewLevel::Runners { owner, repo } => Some(format!(
                "https://github.com/{}/{}/settings/actions/runners",
                owner, repo
            )),
            RunnersViewLevel::Runs { owner, repo, .. } => {
                self.runners.runs.selected_item().map(|run| {
                    format!(
                        "https://github.com/{}/{}/actions/runs/{}",
                        owner, repo, run.id
                    )
                })
            }
            RunnersViewLevel::Jobs {
                owner,
                repo,
                run_id,
                ..
            } => self.runners.jobs.selected_item().map(|job| {
                format!(
                    "https://github.com/{}/{}/actions/runs/{}/job/{}",
                    owner, repo, run_id, job.id
                )
            }),
            RunnersViewLevel::Logs {
                owner,
                repo,
                run_id,
                job_id,
                ..
            } => Some(format!(
                "https://github.com/{}/{}/actions/runs/{}/job/{}",
                owner, repo, run_id, job_id
            )),
        }
    }

    /// Handle Enter key (drill down).
    async fn handle_enter(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.handle_workflows_enter().await,
            Tab::Runners => self.handle_runners_enter().await,
            Tab::Analyze => {
                match &self.analyze.view {
                    AnalyzeViewLevel::List => {
                        // Enter detail view for selected session
                        self.analyze.enter_detail();
                    }
                    AnalyzeViewLevel::Detail { session_id } => {
                        // Go to source log from detail view
                        let session_id = session_id.clone();
                        self.go_to_source(&session_id).await;
                    }
                }
            }
            Tab::Sync => {}
        }
    }

    /// Handle Enter in Workflows tab.
    async fn handle_workflows_enter(&mut self) {
        // Get the next navigation level based on current selection
        // Note: For views with favorites, we must sort to match the displayed order
        let next_level = match self.workflows.nav.current().clone() {
            ViewLevel::Owners => {
                let index = match self.workflows.owners.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.workflows.owners.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_fav = self.favorite_owners.contains(&a.login);
                    let b_fav = self.favorite_owners.contains(&b.login);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.login.cmp(&b.login),
                    }
                });
                sorted.get(index).map(|owner| ViewLevel::Repositories {
                    owner: owner.login.clone(),
                })
            }
            ViewLevel::Repositories { ref owner } => {
                let index = match self.workflows.repositories.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.workflows.repositories.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", owner, a.name);
                    let b_key = format!("{}/{}", owner, b.name);
                    let a_fav = self.favorite_repos.contains(&a_key);
                    let b_fav = self.favorite_repos.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                sorted.get(index).map(|repo| ViewLevel::Workflows {
                    owner,
                    repo: repo.name.clone(),
                })
            }
            ViewLevel::Workflows {
                ref owner,
                ref repo,
            } => {
                let index = match self.workflows.workflows.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.workflows.workflows.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                let repo = repo.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.id);
                    let b_key = format!("{}/{}/{}", owner, repo, b.id);
                    let a_fav = self.favorite_workflows.contains(&a_key);
                    let b_fav = self.favorite_workflows.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                sorted.get(index).map(|workflow| ViewLevel::Runs {
                    owner,
                    repo,
                    workflow_id: workflow.id,
                    workflow_name: workflow.name.clone(),
                })
            }
            ViewLevel::Runs {
                owner,
                repo,
                workflow_id,
                ..
            } => self
                .workflows
                .runs
                .selected_item()
                .map(|run| ViewLevel::Jobs {
                    owner,
                    repo,
                    workflow_id,
                    run_id: run.id,
                    run_number: run.run_number,
                }),
            ViewLevel::Jobs {
                owner,
                repo,
                workflow_id,
                run_id,
                ..
            } => {
                // Use flattened list to get the selected job (main or sub-item)
                if let Some(index) = self.workflows.jobs.selected() {
                    if let Some(list_item) = self.workflows.job_list_items.get(index) {
                        let job = list_item.get_job(&self.workflows.job_groups);
                        Some(ViewLevel::Logs {
                            owner,
                            repo,
                            workflow_id,
                            run_id,
                            job_id: job.id,
                            job_name: job.name.clone(),
                            job_status: job.status,
                            job_conclusion: job.conclusion,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            ViewLevel::Logs { .. } => None, // Can't drill down further
        };

        if let Some(level) = next_level {
            // Check if entering logs and restore saved state
            let job_id_to_restore = if let ViewLevel::Logs { job_id, .. } = &level {
                Some(*job_id)
            } else {
                None
            };

            self.workflows.nav.push(level);

            if let Some(job_id) = job_id_to_restore {
                self.restore_log_state(job_id);
            }

            self.load_current_view().await;
        }
    }

    /// Handle Enter in Runners tab.
    async fn handle_runners_enter(&mut self) {
        // Clear auto-refresh timer when navigating away from runners list
        if !matches!(self.runners.nav.current(), RunnersViewLevel::Runners { .. }) {
            self.runners.runners_view_entered_at = None;
            self.runners.runners_next_refresh = None;
        }

        // Note: For views with favorites, we must sort to match the displayed order
        let next_level = match self.runners.nav.current().clone() {
            RunnersViewLevel::Repositories => {
                let index = match self.runners.repositories.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.runners.repositories.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}", a.owner.login, a.name);
                    let b_key = format!("{}/{}", b.owner.login, b.name);
                    let a_fav = self.favorite_repos.contains(&a_key);
                    let b_fav = self.favorite_repos.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a_key.cmp(&b_key),
                    }
                });
                sorted.get(index).map(|repo| RunnersViewLevel::Runners {
                    owner: repo.owner.login.clone(),
                    repo: repo.name.clone(),
                })
            }
            RunnersViewLevel::Runners {
                ref owner,
                ref repo,
            } => {
                let index = match self.runners.runners.selected() {
                    Some(i) => i,
                    None => return,
                };
                let data = match self.runners.runners.data.data() {
                    Some(d) => d,
                    None => return,
                };
                let mut sorted: Vec<_> = data.items.iter().collect();
                let owner = owner.clone();
                let repo = repo.clone();
                sorted.sort_by(|a, b| {
                    let a_key = format!("{}/{}/{}", owner, repo, a.runner.name);
                    let b_key = format!("{}/{}/{}", owner, repo, b.runner.name);
                    let a_fav = self.favorite_runners.contains(&a_key);
                    let b_fav = self.favorite_runners.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.runner.name.cmp(&b.runner.name),
                    }
                });
                sorted.get(index).map(|enriched| RunnersViewLevel::Runs {
                    owner,
                    repo,
                    runner_name: Some(enriched.runner.name.clone()),
                })
            }
            RunnersViewLevel::Runs { owner, repo, .. } => {
                self.runners
                    .runs
                    .selected_item()
                    .map(|run| RunnersViewLevel::Jobs {
                        owner,
                        repo,
                        run_id: run.id,
                        run_number: run.run_number,
                    })
            }
            RunnersViewLevel::Jobs {
                owner,
                repo,
                run_id,
                ..
            } => {
                // Use flattened list to get the selected job (main or sub-item)
                if let Some(index) = self.runners.jobs.selected() {
                    if let Some(list_item) = self.runners.job_list_items.get(index) {
                        let job = list_item.get_job(&self.runners.job_groups);
                        Some(RunnersViewLevel::Logs {
                            owner,
                            repo,
                            run_id,
                            job_id: job.id,
                            job_name: job.name.clone(),
                            job_status: job.status,
                            job_conclusion: job.conclusion,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            RunnersViewLevel::Logs { .. } => None,
        };

        if let Some(level) = next_level {
            // Check if entering logs and restore saved state
            let job_id_to_restore = if let RunnersViewLevel::Logs { job_id, .. } = &level {
                Some(*job_id)
            } else {
                None
            };

            self.runners.nav.push(level);

            if let Some(job_id) = job_id_to_restore {
                self.restore_log_state(job_id);
            }

            self.load_runners_view().await;
        }
    }

    /// Handle Escape key (go back).
    async fn handle_escape(&mut self) {
        // Save log state before navigating away
        self.save_current_log_state();

        match self.active_tab {
            Tab::Workflows => {
                if self.workflows.go_back() {
                    self.load_current_view().await;
                }
            }
            Tab::Runners => {
                if self.runners.go_back() {
                    // Clear timer if we left the runners list view
                    if !matches!(self.runners.nav.current(), RunnersViewLevel::Runners { .. }) {
                        self.runners.runners_view_entered_at = None;
                        self.runners.runners_next_refresh = None;
                    }
                    self.load_runners_view().await;
                }
            }
            Tab::Analyze => {
                // Exit detail view back to list
                self.analyze.exit_detail();
            }
            Tab::Sync => {}
        }
    }

    /// Handle refresh key.
    async fn handle_refresh(&mut self) {
        match self.active_tab {
            Tab::Workflows => {
                self.workflows.clear_current();
                self.load_current_view().await;
            }
            Tab::Runners => {
                self.runners.clear_current();
                self.load_runners_view().await;
            }
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Called when switching tabs.
    async fn on_tab_change(&mut self) {
        // Clear runners auto-refresh timer when leaving Runners tab
        if self.active_tab != Tab::Runners {
            self.runners.runners_view_entered_at = None;
            self.runners.runners_next_refresh = None;
        }

        match self.active_tab {
            Tab::Workflows => self.load_current_view().await,
            Tab::Runners => self.load_runners_view().await,
            Tab::Analyze | Tab::Sync => {}
        }
    }

    /// Toggle sync enabled state.
    fn toggle_sync(&mut self) {
        let enabled = self.sync.toggle();
        if enabled {
            self.sync.log_info("Sync enabled");
        } else {
            self.sync.log_info("Sync disabled");
        }
    }

    /// Load data for the current view level.
    async fn load_current_view(&mut self) {
        if self.github_client.is_none() {
            self.log_error("No GitHub token configured");
            return;
        }

        let current_view = self.workflows.nav.current().clone();

        match current_view {
            ViewLevel::Owners => {
                if self.workflows.owners.data.is_loaded() {
                    return;
                }
                // Try to load from cache first
                if let Some(path) = cache::owners_list_path() {
                    if let Ok(Some(cached)) = cache::read_cached::<Vec<crate::github::Owner>>(&path)
                    {
                        if cached.is_valid(cache::DEFAULT_TTL) {
                            let count = cached.data.len() as u64;
                            self.workflows.owners.set_loaded(cached.data, count);
                            return;
                        }
                    }
                }
                // No valid cache, fetch from API
                self.workflows.owners.set_loading();
                let result = Self::fetch_owners(self.github_client.as_mut().unwrap()).await;
                match result {
                    Ok((owners, count)) => {
                        if let Some(path) = cache::owners_list_path() {
                            let _ = cache::write_cached(&path, &owners, false);
                        }
                        self.workflows.owners.set_loaded(owners, count);
                    }
                    Err(e) => {
                        self.workflows.owners.set_error(e.to_string());
                        self.log_error(format!("Failed to load owners: {}", e));
                    }
                }
            }
            ViewLevel::Repositories { ref owner } => {
                if self.workflows.repositories.data.is_loaded() {
                    return;
                }
                let owner = owner.clone();
                // Try to load from cache first
                if let Some(path) = cache::repos_list_path(&owner) {
                    if let Ok(Some(cached)) =
                        cache::read_cached::<Vec<crate::github::Repository>>(&path)
                    {
                        if cached.is_valid(cache::DEFAULT_TTL) {
                            let count = cached.data.len() as u64;
                            self.workflows.repositories.set_loaded(cached.data, count);
                            return;
                        }
                    }
                }
                // No valid cache, fetch from API
                self.workflows.repositories.set_loading();
                let result =
                    Self::fetch_repositories(self.github_client.as_mut().unwrap(), &owner).await;
                match result {
                    Ok((repos, count)) => {
                        if let Some(path) = cache::repos_list_path(&owner) {
                            let _ = cache::write_cached(&path, &repos, false);
                        }
                        self.workflows.repositories.set_loaded(repos, count);
                    }
                    Err(e) => {
                        self.workflows.repositories.set_error(e.to_string());
                        self.log_error(format!("Failed to load repositories: {}", e));
                    }
                }
            }
            ViewLevel::Workflows {
                ref owner,
                ref repo,
            } => {
                if self.workflows.workflows.data.is_loaded() {
                    return;
                }
                let owner = owner.clone();
                let repo = repo.clone();
                // Try to load from cache first
                if let Some(path) = cache::workflows_list_path(&owner, &repo) {
                    if let Ok(Some(cached)) =
                        cache::read_cached::<Vec<crate::github::Workflow>>(&path)
                    {
                        if cached.is_valid(cache::DEFAULT_TTL) {
                            let count = cached.data.len() as u64;
                            self.workflows.workflows.set_loaded(cached.data, count);
                            return;
                        }
                    }
                }
                // No valid cache, fetch from API
                self.workflows.workflows.set_loading();
                let result = self
                    .github_client
                    .as_mut()
                    .unwrap()
                    .get_workflows(&owner, &repo, 1, 30)
                    .await;
                match result {
                    Ok((workflows, count)) => {
                        if let Some(path) = cache::workflows_list_path(&owner, &repo) {
                            let _ = cache::write_cached(&path, &workflows, false);
                        }
                        self.workflows.workflows.set_loaded(workflows, count);
                    }
                    Err(e) => {
                        self.workflows.workflows.set_error(e.to_string());
                        self.log_error(format!("Failed to load workflows: {}", e));
                    }
                }
            }
            ViewLevel::Runs {
                ref owner,
                ref repo,
                workflow_id,
                ..
            } => {
                if self.workflows.runs.data.is_loaded() {
                    return;
                }
                let owner = owner.clone();
                let repo = repo.clone();
                // Try to load from cache first
                if let Some(path) = cache::runs_list_path(&owner, &repo, workflow_id) {
                    if let Ok(Some(cached)) =
                        cache::read_cached::<Vec<crate::github::WorkflowRun>>(&path)
                    {
                        if cached.is_valid(cache::DEFAULT_TTL) {
                            let count = cached.data.len() as u64;
                            self.workflows.runs.set_loaded(cached.data, count);
                            return;
                        }
                    }
                }
                // No valid cache, fetch from API
                self.workflows.runs.set_loading();
                let branch = self.workflows.current_branch.as_deref();
                let result = self
                    .github_client
                    .as_mut()
                    .unwrap()
                    .get_workflow_runs_for_workflow(&owner, &repo, workflow_id, 1, 30, branch)
                    .await;
                match result {
                    Ok((runs, count)) => {
                        if let Some(path) = cache::runs_list_path(&owner, &repo, workflow_id) {
                            let _ = cache::write_cached(&path, &runs, false);
                        }
                        self.workflows.runs.set_loaded(runs, count);
                    }
                    Err(e) => {
                        self.workflows.runs.set_error(e.to_string());
                        self.log_error(format!("Failed to load runs: {}", e));
                    }
                }
            }
            ViewLevel::Jobs {
                ref owner,
                ref repo,
                workflow_id,
                run_id,
                ..
            } => {
                if self.workflows.jobs.data.is_loaded() {
                    return;
                }
                let owner = owner.clone();
                let repo = repo.clone();
                // Try to load from cache first
                if let Some(path) = cache::jobs_list_path(&owner, &repo, workflow_id, run_id) {
                    if let Ok(Some(cached)) = cache::read_cached::<Vec<crate::github::Job>>(&path) {
                        if cached.is_valid(cache::DEFAULT_TTL) {
                            let count = cached.data.len() as u64;
                            self.workflows.jobs.set_loaded(cached.data.clone(), count);
                            // Group jobs by name and create flattened list
                            self.workflows.job_groups =
                                crate::github::JobGroup::group_by_name(cached.data);
                            self.workflows.job_list_items =
                                crate::github::JobListItem::flatten(&self.workflows.job_groups);
                            return;
                        }
                    }
                }
                // No valid cache, fetch from API
                self.workflows.jobs.set_loading();
                let result = self
                    .github_client
                    .as_mut()
                    .unwrap()
                    .get_jobs(&owner, &repo, run_id, 1, 30)
                    .await;
                match result {
                    Ok((jobs, count)) => {
                        if let Some(path) =
                            cache::jobs_list_path(&owner, &repo, workflow_id, run_id)
                        {
                            let _ = cache::write_cached(&path, &jobs, false);
                        }
                        self.workflows.jobs.set_loaded(jobs.clone(), count);
                        // Group jobs by name and create flattened list
                        self.workflows.job_groups = crate::github::JobGroup::group_by_name(jobs);
                        self.workflows.job_list_items =
                            crate::github::JobListItem::flatten(&self.workflows.job_groups);
                    }
                    Err(e) => {
                        self.workflows.jobs.set_error(e.to_string());
                        self.log_error(format!("Failed to load jobs: {}", e));
                    }
                }
            }
            ViewLevel::Logs {
                ref owner,
                ref repo,
                workflow_id,
                run_id,
                job_id,
                ..
            } => {
                if self.workflows.log_content.is_loaded() {
                    return;
                }
                let owner = owner.clone();
                let repo = repo.clone();
                // Try to load from cache first (logs are immutable once job completes)
                if let Some(path) = cache::job_log_path(&owner, &repo, workflow_id, run_id, job_id)
                {
                    if let Ok(Some(logs)) = cache::read_text(&path) {
                        self.workflows.log_content = LoadingState::Loaded(logs);
                        return;
                    }
                }
                // No cache, fetch from API
                self.workflows.log_content = LoadingState::Loading;
                let result = self
                    .github_client
                    .as_mut()
                    .unwrap()
                    .get_job_logs(&owner, &repo, job_id)
                    .await;
                match result {
                    Ok(logs) => {
                        if let Some(path) =
                            cache::job_log_path(&owner, &repo, workflow_id, run_id, job_id)
                        {
                            let _ = cache::write_text(&path, &logs);
                        }
                        self.workflows.log_content = LoadingState::Loaded(logs);
                    }
                    Err(e) => {
                        self.workflows.log_content = LoadingState::Error(e.to_string());
                        self.log_error(format!("Failed to load logs: {}", e));
                    }
                }
            }
        }
    }

    /// Fetch owners (current user + their orgs).
    async fn fetch_owners(
        client: &mut GitHubClient,
    ) -> crate::error::Result<(Vec<crate::github::Owner>, u64)> {
        let mut owners = Vec::new();

        // Get current user
        let user = client.get_current_user().await?;
        owners.push(user);

        // Get user's organizations
        let orgs = client.get_user_orgs().await?;
        owners.extend(orgs);

        let count = owners.len() as u64;
        Ok((owners, count))
    }

    /// Fetch repositories for an owner.
    async fn fetch_repositories(
        client: &mut GitHubClient,
        owner: &str,
    ) -> crate::error::Result<(Vec<crate::github::Repository>, u64)> {
        // Try as user repos first, then org repos
        let repos = client.get_user_repos(1, 30).await?;

        // Filter to repos owned by this owner
        let filtered: Vec<_> = repos
            .into_iter()
            .filter(|r| r.owner.login.eq_ignore_ascii_case(owner))
            .collect();

        let count = filtered.len() as u64;
        Ok((filtered, count))
    }

    /// Load data for the runners tab current view level.
    async fn load_runners_view(&mut self) {
        if self.github_client.is_none() {
            self.log_error("No GitHub token configured");
            return;
        }

        let current_view = self.runners.nav.current().clone();

        match current_view {
            RunnersViewLevel::Repositories => {
                if self.runners.repositories.data.is_loaded() {
                    return;
                }
                // Try to load from cache first
                if let Some(path) = cache::runners_repos_path() {
                    if let Ok(Some(cached)) =
                        cache::read_cached::<Vec<crate::github::Repository>>(&path)
                    {
                        if cached.is_valid(cache::DEFAULT_TTL) {
                            let count = cached.data.len() as u64;
                            self.runners.repositories.set_loaded(cached.data, count);
                            return;
                        }
                    }
                }
                // No valid cache, fetch from API
                self.runners.repositories.set_loading();
                let result = self
                    .github_client
                    .as_mut()
                    .unwrap()
                    .get_user_repos(1, 30)
                    .await;
                match result {
                    Ok(repos) => {
                        if let Some(path) = cache::runners_repos_path() {
                            let _ = cache::write_cached(&path, &repos, false);
                        }
                        let count = repos.len() as u64;
                        self.runners.repositories.set_loaded(repos, count);
                    }
                    Err(e) => {
                        self.runners.repositories.set_error(e.to_string());
                        self.log_error(format!("Failed to load repositories: {}", e));
                    }
                }
            }
            RunnersViewLevel::Runners {
                ref owner,
                ref repo,
            } => {
                // Start timer when entering runners view
                if self.runners.runners_view_entered_at.is_none() {
                    let now = std::time::Instant::now();
                    self.runners.runners_view_entered_at = Some(now);
                    self.runners.runners_next_refresh =
                        Some(now + std::time::Duration::from_secs(60));
                }

                if !self.runners.runners.data.is_loaded() {
                    self.runners.runners.set_loading();
                    let owner = owner.clone();
                    let repo = repo.clone();
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_enriched_runners(&owner, &repo, 1, 30)
                        .await;
                    match result {
                        Ok((runners, count)) => {
                            self.runners.runners.set_loaded(runners, count);
                        }
                        Err(e) => {
                            self.runners.runners.set_error(e.to_string());
                            self.log_error(format!("Failed to load runners: {}", e));
                        }
                    }
                }
            }
            RunnersViewLevel::Runs {
                ref owner,
                ref repo,
                ..
            } => {
                if !self.runners.runs.data.is_loaded() {
                    self.runners.runs.set_loading();
                    let owner = owner.clone();
                    let repo = repo.clone();
                    // Get all workflow runs for the repo
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_workflow_runs(&owner, &repo, 1, 30)
                        .await;
                    match result {
                        Ok((runs, count)) => {
                            self.runners.runs.set_loaded(runs, count);
                        }
                        Err(e) => {
                            self.runners.runs.set_error(e.to_string());
                            self.log_error(format!("Failed to load runs: {}", e));
                        }
                    }
                }
            }
            RunnersViewLevel::Jobs {
                ref owner,
                ref repo,
                run_id,
                ..
            } => {
                if !self.runners.jobs.data.is_loaded() {
                    self.runners.jobs.set_loading();
                    let owner = owner.clone();
                    let repo = repo.clone();
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_jobs(&owner, &repo, run_id, 1, 30)
                        .await;
                    match result {
                        Ok((jobs, count)) => {
                            self.runners.jobs.set_loaded(jobs.clone(), count);
                            // Group jobs by name and create flattened list
                            self.runners.job_groups = crate::github::JobGroup::group_by_name(jobs);
                            self.runners.job_list_items =
                                crate::github::JobListItem::flatten(&self.runners.job_groups);
                        }
                        Err(e) => {
                            self.runners.jobs.set_error(e.to_string());
                            self.log_error(format!("Failed to load jobs: {}", e));
                        }
                    }
                }
            }
            RunnersViewLevel::Logs {
                ref owner,
                ref repo,
                job_id,
                ..
            } => {
                if !self.runners.log_content.is_loaded() {
                    self.runners.log_content = LoadingState::Loading;
                    let owner = owner.clone();
                    let repo = repo.clone();
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_job_logs(&owner, &repo, job_id)
                        .await;
                    match result {
                        Ok(logs) => {
                            self.runners.log_content = LoadingState::Loaded(logs);
                        }
                        Err(e) => {
                            self.runners.log_content = LoadingState::Error(e.to_string());
                            self.log_error(format!("Failed to load logs: {}", e));
                        }
                    }
                }
            }
        }
    }

    /// Open branch selection modal.
    fn handle_branch_modal_open(&mut self) {
        // Only open modal in Workflows tab when viewing workflows
        if self.active_tab == Tab::Workflows {
            if matches!(self.workflows.nav.current(), ViewLevel::Workflows { .. }) {
                self.workflows.branch_modal_visible = true;
                self.workflows.branch_input.clear();
                self.workflows.branch_history_selection = 0;
            }
        }
    }

    /// Handle branch switch from modal.
    async fn handle_branch_switch(&mut self) {
        // Determine branch to switch to
        let branch = if self.workflows.branch_input.is_empty() {
            // Use selection from history
            if !self.workflows.branch_history.is_empty() {
                self.workflows.branch_history[self.workflows.branch_history_selection].clone()
            } else {
                return; // Nothing to switch to
            }
        } else {
            // Use typed input
            self.workflows.branch_input.clone()
        };

        // Close modal
        self.workflows.branch_modal_visible = false;
        self.workflows.branch_input.clear();

        // Update current branch
        self.workflows.current_branch = Some(branch.clone());

        // Add to history if not already present
        if !self.workflows.branch_history.contains(&branch) {
            self.workflows.branch_history.insert(0, branch.clone());
            // Keep history to max 10 items
            if self.workflows.branch_history.len() > 10 {
                self.workflows.branch_history.truncate(10);
            }
        }

        // Clear workflows list to force reload with new branch
        self.workflows.workflows = crate::state::workflows::SelectableList::new();

        // Reload workflows for the new branch
        self.load_current_view().await;
    }

    /// Log an error to the sync activity log.
    fn log_error(&mut self, message: impl Into<String>) {
        self.sync.log_error(message);
    }

    /// Log a warning to the sync activity log.
    #[allow(dead_code)]
    fn log_warn(&mut self, message: impl Into<String>) {
        self.sync.log_warn(message);
    }

    /// Log info to the sync activity log.
    #[allow(dead_code)]
    fn log_info(&mut self, message: impl Into<String>) {
        self.sync.log_info(message);
    }
}
