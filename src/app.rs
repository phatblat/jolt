// App state and main event loop.
// Manages tabs, navigation state, and keyboard input handling.

use std::collections::HashSet;
use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};

use crate::cache;
use crate::github::GitHubClient;
use crate::state::{
    LoadingState, NavigationStack, RunnersNavStack, RunnersTabState, RunnersViewLevel, ViewLevel,
    WorkflowsTabState,
};
use crate::ui;

/// Active tab in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tab {
    Runners,
    #[default]
    Workflows,
    Console,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Runners => "Runners",
            Tab::Workflows => "Workflows",
            Tab::Console => "Console",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Tab::Runners => Tab::Workflows,
            Tab::Workflows => Tab::Console,
            Tab::Console => Tab::Runners,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Tab::Runners => Tab::Console,
            Tab::Workflows => Tab::Runners,
            Tab::Console => Tab::Workflows,
        }
    }
}

/// Console message for the Console tab.
#[derive(Debug, Clone)]
pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLevel {
    Info,
    Warn,
    Error,
}

impl ConsoleMessage {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: ConsoleLevel::Error,
            message: message.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            level: ConsoleLevel::Warn,
            message: message.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: ConsoleLevel::Info,
            message: message.into(),
            timestamp: chrono::Utc::now(),
        }
    }
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
    /// Number of unread console errors (for badge).
    pub console_unread: usize,
    /// Console messages.
    pub console_messages: Vec<ConsoleMessage>,
    /// Console list selection state.
    pub console_list_state: ListState,
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
        let mut runners = RunnersTabState::new();
        if let Some(nav) = persisted.runners_nav {
            runners.nav = nav;
        }

        Self {
            active_tab: persisted.active_tab,
            console_unread: 0,
            console_messages: Vec::new(),
            console_list_state: ListState::default(),
            should_quit: false,
            show_help: false,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_index: 0,
            github_client,
            workflows,
            runners,
            favorite_owners: persisted.favorite_owners,
            favorite_repos: persisted.favorite_repos,
            favorite_workflows: persisted.favorite_workflows,
            favorite_runners: persisted.favorite_runners,
        }
    }

    /// Save application state for next session.
    pub fn save_state(&self) {
        let state = PersistedState {
            active_tab: self.active_tab,
            workflows_nav: Some(self.workflows.nav.clone()),
            runners_nav: Some(self.runners.nav.clone()),
            favorite_owners: self.favorite_owners.clone(),
            favorite_repos: self.favorite_repos.clone(),
            favorite_workflows: self.favorite_workflows.clone(),
            favorite_runners: self.favorite_runners.clone(),
        };
        state.save();
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

                    // Handle Ctrl modifier keys first
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        match key.code {
                            KeyCode::Char('d') => self.handle_page_down(),
                            KeyCode::Char('u') => self.handle_page_up(),
                            KeyCode::Char('f') => self.handle_page_down(),
                            KeyCode::Char('b') => self.handle_page_up(),
                            _ => {}
                        }
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('?') => self.show_help = true,
                        KeyCode::Tab => {
                            self.active_tab = self.active_tab.next();
                            self.clear_console_badge_if_viewing();
                            self.on_tab_change().await;
                        }
                        KeyCode::BackTab => {
                            self.active_tab = self.active_tab.prev();
                            self.clear_console_badge_if_viewing();
                            self.on_tab_change().await;
                        }
                        // Direct tab selection
                        KeyCode::Char('1') => {
                            self.active_tab = Tab::Runners;
                            self.clear_console_badge_if_viewing();
                            self.on_tab_change().await;
                        }
                        KeyCode::Char('2') => {
                            self.active_tab = Tab::Workflows;
                            self.clear_console_badge_if_viewing();
                            self.on_tab_change().await;
                        }
                        KeyCode::Char('3') => {
                            self.active_tab = Tab::Console;
                            self.clear_console_badge_if_viewing();
                            self.on_tab_change().await;
                        }
                        // Arrow keys
                        KeyCode::Up => self.handle_up(),
                        KeyCode::Down => self.handle_down(),
                        KeyCode::Left => self.handle_left(),
                        KeyCode::Right => self.handle_right(),
                        // Vim navigation
                        KeyCode::Char('k') => self.handle_up(),
                        KeyCode::Char('j') => self.handle_down(),
                        KeyCode::Char('h') => self.handle_left(),
                        KeyCode::Char('l') => self.handle_right(),
                        // Page navigation
                        KeyCode::PageUp => self.handle_page_up(),
                        KeyCode::PageDown => self.handle_page_down(),
                        // Jump to start/end
                        KeyCode::Home => self.handle_home(),
                        KeyCode::End => self.handle_end(),
                        KeyCode::Char('g') => self.handle_home(),
                        KeyCode::Char('G') => self.handle_end(),
                        // Actions
                        KeyCode::Enter => self.handle_enter().await,
                        KeyCode::Esc => self.handle_escape(),
                        KeyCode::Char('r') => self.handle_refresh().await,
                        KeyCode::Char('/') => self.handle_search_start(),
                        KeyCode::Char('o') => self.handle_open_in_browser(),
                        KeyCode::Char('f') => self.toggle_favorite(),
                        // Search navigation
                        KeyCode::Char('n') => self.search_next(),
                        KeyCode::Char('N') => self.search_prev(),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle up arrow key.
    fn handle_up(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.select_prev(),
            Tab::Runners => self.runners.select_prev(),
            Tab::Console => self.console_select_prev(),
        }
    }

    /// Handle down arrow key.
    fn handle_down(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.select_next(),
            Tab::Runners => self.runners.select_next(),
            Tab::Console => self.console_select_next(),
        }
    }

    /// Handle left arrow key.
    fn handle_left(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.scroll_left(),
            Tab::Runners => self.runners.scroll_left(),
            Tab::Console => {}
        }
    }

    /// Handle right arrow key.
    fn handle_right(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.scroll_right(),
            Tab::Runners => self.runners.scroll_right(),
            Tab::Console => {}
        }
    }

    /// Handle Page Up key.
    fn handle_page_up(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.page_up(),
            Tab::Runners => self.runners.page_up(),
            Tab::Console => {}
        }
    }

    /// Handle Page Down key.
    fn handle_page_down(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.page_down(),
            Tab::Runners => self.runners.page_down(),
            Tab::Console => {}
        }
    }

    /// Handle Home key.
    fn handle_home(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.scroll_to_start(),
            Tab::Runners => self.runners.scroll_to_start(),
            Tab::Console => {}
        }
    }

    /// Handle End key.
    fn handle_end(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.scroll_to_end(),
            Tab::Runners => self.runners.scroll_to_end(),
            Tab::Console => {}
        }
    }

    /// Handle search start (/ key).
    fn handle_search_start(&mut self) {
        // Only activate search when viewing logs
        let in_logs = match self.active_tab {
            Tab::Workflows => matches!(self.workflows.nav.current(), ViewLevel::Logs { .. }),
            Tab::Runners => matches!(self.runners.nav.current(), RunnersViewLevel::Logs { .. }),
            Tab::Console => false,
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
            Tab::Console => return,
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
                Tab::Console => {}
            }
        }
    }

    /// Open the current item in GitHub in the browser.
    fn handle_open_in_browser(&mut self) {
        let url = match self.active_tab {
            Tab::Workflows => self.get_workflows_github_url(),
            Tab::Runners => self.get_runners_github_url(),
            Tab::Console => None,
        };

        #[allow(clippy::collapsible_if)]
        if let Some(url) = url {
            if let Err(e) = std::process::Command::new("open").arg(&url).spawn() {
                self.log_error(format!("Failed to open browser: {}", e));
            }
        }
    }

    /// Toggle favorite status for the currently selected item.
    fn toggle_favorite(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.toggle_workflows_favorite(),
            Tab::Runners => self.toggle_runners_favorite(),
            Tab::Console => {}
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
                    let a_key = format!("{}/{}/{}", owner, repo, a.name);
                    let b_key = format!("{}/{}/{}", owner, repo, b.name);
                    let a_fav = self.favorite_runners.contains(&a_key);
                    let b_fav = self.favorite_runners.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                if let Some(runner) = sorted.get(index) {
                    let key = format!("{}/{}/{}", owner, repo, runner.name);
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
            } => self.workflows.jobs.selected_item().map(|job| {
                format!(
                    "https://github.com/{}/{}/actions/runs/{}/job/{}",
                    owner, repo, run_id, job.id
                )
            }),
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
            Tab::Console => {}
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
            } => self
                .workflows
                .jobs
                .selected_item()
                .map(|job| ViewLevel::Logs {
                    owner,
                    repo,
                    workflow_id,
                    run_id,
                    job_id: job.id,
                    job_name: job.name.clone(),
                    job_status: job.status,
                    job_conclusion: job.conclusion,
                }),
            ViewLevel::Logs { .. } => None, // Can't drill down further
        };

        if let Some(level) = next_level {
            self.workflows.nav.push(level);
            self.load_current_view().await;
        }
    }

    /// Handle Enter in Runners tab.
    async fn handle_runners_enter(&mut self) {
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
                    let a_key = format!("{}/{}/{}", owner, repo, a.name);
                    let b_key = format!("{}/{}/{}", owner, repo, b.name);
                    let a_fav = self.favorite_runners.contains(&a_key);
                    let b_fav = self.favorite_runners.contains(&b_key);
                    match (a_fav, b_fav) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.name.cmp(&b.name),
                    }
                });
                sorted.get(index).map(|runner| RunnersViewLevel::Runs {
                    owner,
                    repo,
                    runner_name: Some(runner.name.clone()),
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
            } => self
                .runners
                .jobs
                .selected_item()
                .map(|job| RunnersViewLevel::Logs {
                    owner,
                    repo,
                    run_id,
                    job_id: job.id,
                    job_name: job.name.clone(),
                    job_status: job.status,
                    job_conclusion: job.conclusion,
                }),
            RunnersViewLevel::Logs { .. } => None,
        };

        if let Some(level) = next_level {
            self.runners.nav.push(level);
            self.load_runners_view().await;
        }
    }

    /// Handle Escape key (go back).
    fn handle_escape(&mut self) {
        match self.active_tab {
            Tab::Workflows => {
                self.workflows.go_back();
            }
            Tab::Runners => {
                self.runners.go_back();
            }
            Tab::Console => {}
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
            Tab::Console => {}
        }
    }

    /// Called when switching tabs.
    async fn on_tab_change(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.load_current_view().await,
            Tab::Runners => self.load_runners_view().await,
            Tab::Console => {}
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
                let result = self
                    .github_client
                    .as_mut()
                    .unwrap()
                    .get_workflow_runs_for_workflow(&owner, &repo, workflow_id, 1, 30)
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
                            self.workflows.jobs.set_loaded(cached.data, count);
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
                        self.workflows.jobs.set_loaded(jobs, count);
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
                if !self.runners.runners.data.is_loaded() {
                    self.runners.runners.set_loading();
                    let owner = owner.clone();
                    let repo = repo.clone();
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_runners(&owner, &repo, 1, 30)
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
                            self.runners.jobs.set_loaded(jobs, count);
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

    /// Log an error to the console tab.
    fn log_error(&mut self, message: impl Into<String>) {
        self.console_messages.push(ConsoleMessage::error(message));
        self.console_unread += 1;
    }

    /// Log a warning to the console tab.
    #[allow(dead_code)]
    fn log_warn(&mut self, message: impl Into<String>) {
        self.console_messages.push(ConsoleMessage::warn(message));
    }

    /// Log info to the console tab.
    #[allow(dead_code)]
    fn log_info(&mut self, message: impl Into<String>) {
        self.console_messages.push(ConsoleMessage::info(message));
    }

    /// Select previous item in console list.
    fn console_select_prev(&mut self) {
        if self.console_messages.is_empty() {
            return;
        }
        let i = match self.console_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => self.console_messages.len().saturating_sub(1),
        };
        self.console_list_state.select(Some(i));
    }

    /// Select next item in console list.
    fn console_select_next(&mut self) {
        if self.console_messages.is_empty() {
            return;
        }
        let i = match self.console_list_state.selected() {
            Some(i) => {
                if i >= self.console_messages.len() - 1 {
                    self.console_messages.len() - 1
                } else {
                    i + 1
                }
            }
            None => self.console_messages.len().saturating_sub(1),
        };
        self.console_list_state.select(Some(i));
    }

    /// Clear console badge when viewing console tab.
    fn clear_console_badge_if_viewing(&mut self) {
        if self.active_tab == Tab::Console {
            self.console_unread = 0;
        }
    }
}
