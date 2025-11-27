// App state and main event loop.
// Manages tabs, navigation state, and keyboard input handling.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;

use crate::github::GitHubClient;
use crate::state::{LoadingState, ViewLevel, WorkflowsTabState};
use crate::ui;

/// Active tab in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// Main application state.
pub struct App {
    /// Currently active tab.
    pub active_tab: Tab,
    /// Number of unread console errors (for badge).
    pub console_unread: usize,
    /// Console messages.
    pub console_messages: Vec<ConsoleMessage>,
    /// Whether the app should exit.
    pub should_quit: bool,
    /// GitHub API client (None if no token).
    pub github_client: Option<GitHubClient>,
    /// Workflows tab state.
    pub workflows: WorkflowsTabState,
}

impl App {
    pub fn new() -> Self {
        // Try to create GitHub client from env
        let github_client = match GitHubClient::from_env() {
            Ok(client) => Some(client),
            Err(e) => {
                // Will show error in console tab
                eprintln!("GitHub client error: {}", e);
                None
            }
        };

        Self {
            active_tab: Tab::default(),
            console_unread: 0,
            console_messages: Vec::new(),
            should_quit: false,
            github_client,
            workflows: WorkflowsTabState::new(),
        }
    }

    /// Main event loop.
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        // Initial data load for workflows tab
        self.load_current_view().await;

        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, self))?;
            self.handle_events().await?;
        }
        Ok(())
    }

    /// Handle keyboard and other events.
    #[allow(clippy::collapsible_if)]
    async fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
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
                        KeyCode::Up => self.handle_up(),
                        KeyCode::Down => self.handle_down(),
                        KeyCode::Left => self.handle_left(),
                        KeyCode::Right => self.handle_right(),
                        KeyCode::PageUp => self.handle_page_up(),
                        KeyCode::PageDown => self.handle_page_down(),
                        KeyCode::Home => self.handle_home(),
                        KeyCode::End => self.handle_end(),
                        KeyCode::Enter => self.handle_enter().await,
                        KeyCode::Esc => self.handle_escape(),
                        KeyCode::Char('r') => self.handle_refresh().await,
                        KeyCode::Char('/') => self.handle_search_start(),
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
            Tab::Runners => {} // TODO: Implement runners tab
            Tab::Console => {} // TODO: Scroll console
        }
    }

    /// Handle down arrow key.
    fn handle_down(&mut self) {
        match self.active_tab {
            Tab::Workflows => self.workflows.select_next(),
            Tab::Runners => {}
            Tab::Console => {}
        }
    }

    /// Handle left arrow key.
    fn handle_left(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.scroll_left();
        }
    }

    /// Handle right arrow key.
    fn handle_right(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.scroll_right();
        }
    }

    /// Handle Page Up key.
    fn handle_page_up(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.page_up();
        }
    }

    /// Handle Page Down key.
    fn handle_page_down(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.page_down();
        }
    }

    /// Handle Home key.
    fn handle_home(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.scroll_to_start();
        }
    }

    /// Handle End key.
    fn handle_end(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.scroll_to_end();
        }
    }

    /// Handle search start (/ key).
    fn handle_search_start(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.search_active = true;
        }
    }

    /// Handle Enter key (drill down).
    async fn handle_enter(&mut self) {
        if self.active_tab != Tab::Workflows {
            return;
        }

        // Get the next navigation level based on current selection
        let next_level =
            match self.workflows.nav.current().clone() {
                ViewLevel::Owners => {
                    self.workflows
                        .owners
                        .selected_item()
                        .map(|owner| ViewLevel::Repositories {
                            owner: owner.login.clone(),
                        })
                }
                ViewLevel::Repositories { owner } => self
                    .workflows
                    .repositories
                    .selected_item()
                    .map(|repo| ViewLevel::Workflows {
                        owner,
                        repo: repo.name.clone(),
                    }),
                ViewLevel::Workflows { owner, repo } => self
                    .workflows
                    .workflows
                    .selected_item()
                    .map(|workflow| ViewLevel::Runs {
                        owner,
                        repo,
                        workflow_id: workflow.id,
                        workflow_name: workflow.name.clone(),
                    }),
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
                    }),
                ViewLevel::Logs { .. } => None, // Can't drill down further
            };

        if let Some(level) = next_level {
            self.workflows.nav.push(level);
            self.load_current_view().await;
        }
    }

    /// Handle Escape key (go back).
    fn handle_escape(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.go_back();
        }
    }

    /// Handle refresh key.
    async fn handle_refresh(&mut self) {
        if self.active_tab == Tab::Workflows {
            self.workflows.clear_current();
            self.load_current_view().await;
        }
    }

    /// Called when switching tabs.
    async fn on_tab_change(&mut self) {
        // Load data for the new tab if needed
        if self.active_tab == Tab::Workflows {
            self.load_current_view().await;
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
                if !self.workflows.owners.data.is_loaded() {
                    self.workflows.owners.set_loading();
                    let result = Self::fetch_owners(self.github_client.as_mut().unwrap()).await;
                    match result {
                        Ok((owners, count)) => {
                            self.workflows.owners.set_loaded(owners, count);
                        }
                        Err(e) => {
                            self.workflows.owners.set_error(e.to_string());
                            self.log_error(format!("Failed to load owners: {}", e));
                        }
                    }
                }
            }
            ViewLevel::Repositories { ref owner } => {
                if !self.workflows.repositories.data.is_loaded() {
                    self.workflows.repositories.set_loading();
                    let owner = owner.clone();
                    let result =
                        Self::fetch_repositories(self.github_client.as_mut().unwrap(), &owner)
                            .await;
                    match result {
                        Ok((repos, count)) => {
                            self.workflows.repositories.set_loaded(repos, count);
                        }
                        Err(e) => {
                            self.workflows.repositories.set_error(e.to_string());
                            self.log_error(format!("Failed to load repositories: {}", e));
                        }
                    }
                }
            }
            ViewLevel::Workflows {
                ref owner,
                ref repo,
            } => {
                if !self.workflows.workflows.data.is_loaded() {
                    self.workflows.workflows.set_loading();
                    let owner = owner.clone();
                    let repo = repo.clone();
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_workflows(&owner, &repo, 1, 30)
                        .await;
                    match result {
                        Ok((workflows, count)) => {
                            self.workflows.workflows.set_loaded(workflows, count);
                        }
                        Err(e) => {
                            self.workflows.workflows.set_error(e.to_string());
                            self.log_error(format!("Failed to load workflows: {}", e));
                        }
                    }
                }
            }
            ViewLevel::Runs {
                ref owner,
                ref repo,
                workflow_id,
                ..
            } => {
                if !self.workflows.runs.data.is_loaded() {
                    self.workflows.runs.set_loading();
                    let owner = owner.clone();
                    let repo = repo.clone();
                    let result = self
                        .github_client
                        .as_mut()
                        .unwrap()
                        .get_workflow_runs_for_workflow(&owner, &repo, workflow_id, 1, 30)
                        .await;
                    match result {
                        Ok((runs, count)) => {
                            self.workflows.runs.set_loaded(runs, count);
                        }
                        Err(e) => {
                            self.workflows.runs.set_error(e.to_string());
                            self.log_error(format!("Failed to load runs: {}", e));
                        }
                    }
                }
            }
            ViewLevel::Jobs {
                ref owner,
                ref repo,
                run_id,
                ..
            } => {
                if !self.workflows.jobs.data.is_loaded() {
                    self.workflows.jobs.set_loading();
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
                            self.workflows.jobs.set_loaded(jobs, count);
                        }
                        Err(e) => {
                            self.workflows.jobs.set_error(e.to_string());
                            self.log_error(format!("Failed to load jobs: {}", e));
                        }
                    }
                }
            }
            ViewLevel::Logs {
                ref owner,
                ref repo,
                job_id,
                ..
            } => {
                if !self.workflows.log_content.is_loaded() {
                    self.workflows.log_content = LoadingState::Loading;
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

    /// Clear console badge when viewing console tab.
    fn clear_console_badge_if_viewing(&mut self) {
        if self.active_tab == Tab::Console {
            self.console_unread = 0;
        }
    }
}
