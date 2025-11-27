// Sync tab state management.
// Handles background sync control, metrics, and error tracking.

use chrono::{DateTime, Utc};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Instant;

/// Sync status indicator.
#[derive(Debug, Clone, Default)]
pub enum SyncStatus {
    #[default]
    Idle,
    Running,
    Paused {
        reason: PauseReason,
    },
}

/// Reason for sync being paused.
#[derive(Debug, Clone)]
pub enum PauseReason {
    UserDisabled,
    RateLimited { reset_at: DateTime<Utc> },
    ErrorThreshold,
}

/// Phase of the sync process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SyncPhase {
    #[default]
    Idle,
    FetchingFavorites,
    FetchingRuns,
    FetchingJobs,
    DownloadingLogs,
}

impl SyncPhase {
    pub fn display(&self) -> &'static str {
        match self {
            SyncPhase::Idle => "Idle",
            SyncPhase::FetchingFavorites => "Fetching favorites",
            SyncPhase::FetchingRuns => "Fetching runs",
            SyncPhase::FetchingJobs => "Fetching jobs",
            SyncPhase::DownloadingLogs => "Downloading logs",
        }
    }
}

/// Progress of the current sync cycle.
#[derive(Debug, Clone, Default)]
pub struct SyncProgress {
    /// Current phase of sync.
    pub phase: SyncPhase,
    /// Description of current activity.
    pub current_item: Option<String>,
    /// Pending job metadata fetches.
    pub pending_jobs: usize,
    /// Pending log downloads.
    pub pending_logs: usize,
}

/// Metrics collected during sync.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncMetrics {
    /// Total jobs synced across all sessions.
    pub jobs_synced_total: u64,
    /// Jobs synced this session.
    #[serde(skip)]
    pub jobs_synced_session: u64,
    /// Total logs cached across all sessions.
    pub logs_cached_total: u64,
    /// Logs cached this session.
    #[serde(skip)]
    pub logs_cached_session: u64,
    /// Total errors encountered.
    pub errors_total: u64,
}

/// Console message level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLevel {
    Info,
    Warn,
    Error,
}

/// A console message for the activity log.
#[derive(Debug, Clone)]
pub struct ConsoleMessage {
    pub level: ConsoleLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl ConsoleMessage {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: ConsoleLevel::Info,
            message: message.into(),
            timestamp: Utc::now(),
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            level: ConsoleLevel::Warn,
            message: message.into(),
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: ConsoleLevel::Error,
            message: message.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Tracks recent errors for threshold-based pausing.
#[derive(Debug, Default)]
pub struct ErrorTracker {
    /// Timestamps of recent errors (rolling window).
    errors: VecDeque<Instant>,
}

impl ErrorTracker {
    const WINDOW_SECS: u64 = 60;
    const THRESHOLD: usize = 5;

    pub fn new() -> Self {
        Self::default()
    }

    /// Record an error. Returns true if threshold exceeded.
    pub fn record_error(&mut self) -> bool {
        let now = Instant::now();
        self.errors.push_back(now);
        self.prune_old();
        self.errors.len() >= Self::THRESHOLD
    }

    /// Get count of errors in the current window.
    pub fn count_in_window(&self) -> usize {
        self.errors.len()
    }

    /// Remove errors outside the 1-minute window.
    fn prune_old(&mut self) {
        let cutoff = Instant::now()
            .checked_sub(std::time::Duration::from_secs(Self::WINDOW_SECS))
            .unwrap_or_else(Instant::now);
        while self.errors.front().is_some_and(|&t| t < cutoff) {
            self.errors.pop_front();
        }
    }

    /// Reset error tracking (after user resumes).
    pub fn reset(&mut self) {
        self.errors.clear();
    }
}

/// Complete state for the Sync tab.
#[derive(Debug)]
pub struct SyncTabState {
    /// Whether sync is enabled by user.
    pub enabled: bool,
    /// Current sync status.
    pub status: SyncStatus,
    /// Progress of current sync cycle.
    pub progress: SyncProgress,
    /// Metrics for sync operations.
    pub metrics: SyncMetrics,
    /// Error tracker for threshold detection.
    pub error_tracker: ErrorTracker,
    /// Console messages (activity log).
    pub messages: Vec<ConsoleMessage>,
    /// List state for message scrolling.
    pub list_state: ListState,
}

impl Default for SyncTabState {
    fn default() -> Self {
        Self {
            enabled: false,
            status: SyncStatus::Idle,
            progress: SyncProgress::default(),
            metrics: SyncMetrics::default(),
            error_tracker: ErrorTracker::new(),
            messages: Vec::new(),
            list_state: ListState::default(),
        }
    }
}

impl SyncTabState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle sync enabled state.
    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        if self.enabled {
            self.status = SyncStatus::Running;
            self.error_tracker.reset();
        } else {
            self.status = SyncStatus::Paused {
                reason: PauseReason::UserDisabled,
            };
        }
        self.enabled
    }

    /// Add an info message.
    pub fn log_info(&mut self, message: impl Into<String>) {
        self.messages.push(ConsoleMessage::info(message));
        self.scroll_to_bottom();
    }

    /// Add a warning message.
    pub fn log_warn(&mut self, message: impl Into<String>) {
        self.messages.push(ConsoleMessage::warn(message));
        self.scroll_to_bottom();
    }

    /// Add an error message and track for threshold.
    pub fn log_error(&mut self, message: impl Into<String>) {
        self.messages.push(ConsoleMessage::error(message));
        self.metrics.errors_total += 1;

        // Check if we should pause due to error threshold
        if self.error_tracker.record_error() {
            self.status = SyncStatus::Paused {
                reason: PauseReason::ErrorThreshold,
            };
        }

        self.scroll_to_bottom();
    }

    /// Scroll message list to bottom.
    fn scroll_to_bottom(&mut self) {
        if !self.messages.is_empty() {
            self.list_state.select(Some(self.messages.len() - 1));
        }
    }

    /// Select previous message in list.
    pub fn select_prev(&mut self) {
        if self.messages.is_empty() {
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
            None => self.messages.len().saturating_sub(1),
        };
        self.list_state.select(Some(i));
    }

    /// Select next message in list.
    pub fn select_next(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    i
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Check if sync is actively running.
    pub fn is_running(&self) -> bool {
        self.enabled && matches!(self.status, SyncStatus::Running)
    }

    /// Get status display string.
    pub fn status_display(&self) -> (&'static str, &'static str) {
        if !self.enabled {
            ("OFF", "gray")
        } else {
            match &self.status {
                SyncStatus::Idle => ("IDLE", "gray"),
                SyncStatus::Running => ("ON", "green"),
                SyncStatus::Paused { reason } => match reason {
                    PauseReason::UserDisabled => ("OFF", "gray"),
                    PauseReason::RateLimited { .. } => ("RATE LIMITED", "yellow"),
                    PauseReason::ErrorThreshold => ("PAUSED", "red"),
                },
            }
        }
    }
}
