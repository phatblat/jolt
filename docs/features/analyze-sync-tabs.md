# Analyze and Sync Tabs

## Overview
Two new tabs that replace the Console tab: Analyze for saving and reviewing log excerpts with full context, and Sync for controlling background synchronization with metrics and activity monitoring.

## User Perspective

### Analyze Tab
Users can save interesting log excerpts while reviewing workflow or runner logs. When viewing logs, pressing 'a' saves the selected lines (or current line if no selection) as an "analysis session" that includes:
- The log excerpt itself
- Full navigation context (repo, workflow, run, job)
- Metadata (PR number, branch, commit SHA, author, runner info)
- GitHub URL to the original job
- Timestamp when saved

The Analyze tab shows a list of all saved sessions, sorted newest first. Pressing Enter on a session shows the full detail view with the log excerpt and all metadata. From the detail view, pressing Enter again navigates back to the original log location in the Workflows or Runners tab, restoring the exact scroll position and selection.

Duplicate detection prevents saving the same job/line range twice - if you try to save identical content, the app navigates to the existing session instead.

### Sync Tab
Users can toggle background synchronization on/off with Shift+S (works from any tab). The Sync tab displays:
- Current sync status (ON/OFF/PAUSED)
- Current activity (what's being synced right now)
- Metrics (jobs synced, logs cached, errors)
- Activity log with timestamped messages

Error tracking automatically pauses sync if 5 errors occur within 1 minute, preventing runaway failures. Users can re-enable sync after investigating issues.

## Data Flow

### Saving an Analysis Session (Analyze Tab)
1. User views log in Workflows or Runners tab, selects lines (or uses cursor position)
2. User presses 'a' key
3. App checks for duplicate (same job_id + run_id + line range)
4. If duplicate exists: navigates to existing session in Analyze tab
5. If new: creates AnalysisSession with all context and metadata
6. App switches to Analyze tab and highlights the new/existing session
7. Session is added to in-memory state (persistence planned for future)

### Navigating Back to Source (Analyze Tab)
1. User views session detail in Analyze tab
2. User presses Enter (go to source)
3. App reads NavigationContext from session
4. App switches to source tab (Workflows or Runners)
5. App navigates to correct repo/workflow/run/job
6. App loads log file if not already loaded
7. App restores scroll position and selection range
8. User sees original log with same lines highlighted

### Toggling Sync (Sync Tab)
1. User presses Shift+S from any tab
2. App toggles sync.enabled flag
3. If enabling: status becomes Running, error tracker resets
4. If disabling: status becomes Paused with UserDisabled reason
5. Activity log records the state change
6. UI updates to show new status

### Error Tracking (Sync Tab)
1. Background sync encounters error
2. App calls sync.log_error(message)
3. Error added to activity log, metrics.errors_total incremented
4. ErrorTracker records timestamp in rolling 60-second window
5. If 5+ errors in window: status becomes Paused with ErrorThreshold reason
6. User investigates in Sync tab, then re-enables with Shift+S

## Implementation

### Key Files
- `src/state/analyze.rs` - AnalysisSession, NavigationContext, RunMetadata, AnalyzeTabState
  - Session data structure with full context for saved log excerpts
  - View state management (list vs detail)
  - Selection tracking and duplicate detection
- `src/state/sync.rs` - SyncTabState, SyncStatus, SyncProgress, SyncMetrics, ErrorTracker, ConsoleMessage
  - Sync control state and status tracking
  - Progress tracking with phases (fetching favorites/runs/jobs, downloading logs)
  - Error tracking with 5-errors-in-1-minute threshold
  - Activity log with timestamped messages
- `src/app.rs` - Tab enum, App state
  - Changed Tab enum: Console → Analyze + Sync (4 tabs total)
  - Added analyze and sync state fields to App
  - 'a' key handler for saving log selections to Analyze
  - Shift+S handler for sync toggle (global, works from any tab)
  - go_to_source() method for navigating from Analyze back to source logs
  - Duplicate detection when saving sessions
- `src/ui/mod.rs` - Tab rendering
  - draw_analyze_tab() with list and detail views
  - draw_analyze_list() for session list
  - draw_analyze_detail() for session detail with metadata and log excerpt
  - draw_sync_tab() with status panel, progress info, metrics, and activity log
  - Updated help overlay with new keybindings
  - Added clipboard flash indicator for log viewer status bar
- `src/ui/tabs.rs` - Tab bar rendering
  - Updated to 4-tab layout (Runners, Workflows, Analyze, Sync)
  - Tab numbers 1-4 for direct selection
- `src/state/mod.rs` - Module exports
  - Exports for analyze and sync modules

### Database
N/A - State is currently in-memory only. Persistence planned for future phase.

## Configuration
- No environment variables required
- Feature is always enabled
- No feature flags

## Usage Example

```rust
// Saving a log excerpt from Workflows log viewer
// User has lines 45-52 selected and presses 'a'
let session = AnalysisSession::new(
    NavigationContext {
        source_tab: SourceTab::Workflows,
        owner: "getditto".to_string(),
        repo: "ditto".to_string(),
        workflow_id: Some(12345),
        workflow_name: Some("CI".to_string()),
        run_id: 67890,
        run_number: 123,
        job_id: 11111,
        job_name: "Build iOS".to_string(),
        job_status: RunStatus::Completed,
        job_conclusion: Some(RunConclusion::Failure),
        scroll_to_line: 45,
        selection_anchor: 45,
        selection_cursor: 52,
    },
    RunMetadata {
        pr_number: Some(456),
        branch_name: Some("feature/new-api".to_string()),
        commit_sha: "abc1234".to_string(),
        author: Some("devuser".to_string()),
        runner_name: Some("runner-1".to_string()),
        runner_labels: vec!["self-hosted".to_string(), "macOS".to_string()],
    },
    "https://github.com/getditto/ditto/runs/11111".to_string(),
    log_excerpt_text,
    500, // total lines
    45,  // start line
    52,  // end line
);

// Check for duplicate before adding
if let Some(existing) = app.analyze.find_duplicate(job_id, run_id, 45, 52) {
    // Navigate to existing session
    app.analyze.enter_detail_by_id(&existing.id);
} else {
    // Add new session
    app.analyze.add_session(session);
}

// Toggle sync from any tab
if key == Shift+S {
    let enabled = app.sync.toggle();
    let msg = if enabled {
        "Background sync enabled"
    } else {
        "Background sync disabled"
    };
    app.sync.log_info(msg);
}

// Navigate back to source from Analyze detail view
if key == Enter && in_detail_view {
    app.go_to_source().await;
}
```

## Testing

### Manual Testing: Analyze Tab

#### Saving Sessions
1. Launch jolt: `just run`
2. Navigate to a workflow log (Tab 2, select repo, select workflow, select run, select job)
3. Wait for log to load
4. Press 'v' to enter selection mode
5. Use arrow keys to select multiple lines
6. Press 'a' to save selection
7. App should switch to Analyze tab showing the new session at top of list
8. Verify session title includes job name, line count, and repo name
9. Try saving again with same lines selected
10. App should navigate to existing session instead of creating duplicate

#### Viewing Sessions
1. In Analyze tab list view, use arrow keys to navigate sessions
2. Press Enter to view session detail
3. Verify detail shows:
   - Session title and timestamp
   - Navigation breadcrumb (source → repo → workflow → run → job)
   - Run metadata (PR, branch, commit, author, runner)
   - GitHub URL
   - Log excerpt with line numbers
4. Verify log excerpt shows correct lines with proper formatting

#### Navigation Back to Source
1. From session detail view, press Enter
2. App should switch to Workflows or Runners tab (depending on source)
3. App should navigate to correct repo/workflow/run/job
4. Log should load (if not already cached)
5. Scroll position should match original location
6. Selection range should be restored
7. Verify lines match the saved excerpt

### Manual Testing: Sync Tab

#### Toggle Sync
1. Press Shift+S from any tab
2. Switch to Sync tab (Tab 4)
3. Verify status shows "ON" in green
4. Verify activity log shows "Background sync enabled" message
5. Press Shift+S again
6. Verify status shows "OFF" in gray
7. Verify activity log shows "Background sync disabled" message

#### Error Tracking
1. Enable sync (Shift+S)
2. Simulate 5 errors quickly (requires code modification or real failures)
3. Verify status changes to "PAUSED" in red
4. Verify activity log shows error messages
5. Verify metrics show error count
6. Re-enable sync with Shift+S
7. Verify status returns to "ON" after manual re-enable

### Expected Behavior
- Sessions persist for duration of app session (lost on quit - persistence planned)
- Duplicate detection based on job_id + run_id + line range
- Navigation back to source works from any session, regardless of current tab state
- Sync toggle works immediately from any tab
- Error threshold pauses sync to prevent runaway failures
- Activity log scrolls to bottom when new messages arrive
- Clipboard indicator (📋) flashes briefly after copying (from previous log viewer feature)

## Related Documentation
- Architecture: `docs/ratatui-plan.md` - Phase 1-3 implementation plan
- State Management: `src/state/mod.rs` - All state module exports
- GitHub API: `src/github/mod.rs` - API client (used by future sync implementation)
