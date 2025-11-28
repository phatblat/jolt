# Runners Auto-Refresh Timer

## Overview
Automatically refreshes the Self-Hosted Runners list every 60 seconds when viewing the runners list, ensuring users see up-to-date runner status without manual intervention.

## User Perspective
When users navigate to the Self-Hosted Runners list view (owner/repo level), the application automatically starts a 60-second timer. After 60 seconds, the runners list refreshes in-place, showing updated status, labels, and availability. The timer continues to refresh every 60 seconds while the user remains on that view. Navigating away (drilling down, going back, or switching tabs) immediately stops the timer.

## Data Flow
1. User navigates to Runners tab > owner/repo > Self-Hosted Runners list
2. App detects view entry and records timestamp in `runners_view_entered_at`
3. App schedules first refresh at `now + 60 seconds` in `runners_next_refresh`
4. Event loop checks timer on each iteration
5. When timer expires, app calls `github_client.get_runners()` with force reload
6. Received runner data updates `runners.set_loaded()` with new items and timestamp
7. App schedules next refresh at `now + 60 seconds`
8. On navigation away, both timer fields are cleared (`None`)

## Implementation

### Key Files
- `src/state/runners.rs` - Added timer state fields to `RunnersTabState`:
  - `runners_view_entered_at: Option<std::time::Instant>` - When view was entered
  - `runners_next_refresh: Option<std::time::Instant>` - When next refresh should occur
- `src/app.rs` - Timer logic implementation:
  - `load_runners_view()` - Starts timer when entering runners list view
  - `tick()` - Checks timer expiration and triggers refresh
  - `handle_runners_enter()` - Clears timer when drilling down
  - `go_back()` - Clears timer when navigating back from runners list
  - `on_tab_change()` - Clears timer when switching away from Runners tab
- `src/ui/list.rs` - Updated title from "Runners" to "Self-Hosted Runners"

### Timer Management
The timer uses `std::time::Instant` for monotonic time measurements, avoiding issues with system clock changes. Timer lifecycle:
- **Start**: Set both fields when entering `RunnersViewLevel::Runners`
- **Check**: Every event loop iteration compares `now >= runners_next_refresh`
- **Refresh**: On expiration, reload runners data and reschedule
- **Clear**: Set both fields to `None` on any navigation away from runners list

### Data Loading
When timer expires:
1. Call `runners.set_loading()` to show loading state
2. Request fresh data: `github_client.get_runners(owner, repo, 1, 30)`
3. On success: `runners.set_loaded(runners, count)` updates state and timestamp
4. On error: `runners.set_error(e.to_string())` shows error message
5. Schedule next refresh regardless of success/failure

## Configuration
- Refresh interval: 60 seconds (hardcoded in `std::time::Duration::from_secs(60)`)
- Page size: 30 runners per request
- No environment variables or feature flags

## Usage Example
```rust
// Timer initialization when entering runners view
if self.runners.runners_view_entered_at.is_none() {
    let now = std::time::Instant::now();
    self.runners.runners_view_entered_at = Some(now);
    self.runners.runners_next_refresh = Some(now + std::time::Duration::from_secs(60));
}

// Timer check in event loop
if let Some(next_refresh) = self.runners.runners_next_refresh {
    if std::time::Instant::now() >= next_refresh {
        // Trigger refresh and reschedule
        self.runners.runners_next_refresh = 
            Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
    }
}

// Timer cleanup on navigation
if self.active_tab != Tab::Runners {
    self.runners.runners_view_entered_at = None;
    self.runners.runners_next_refresh = None;
}
```

## Testing
- **Manual test**: 
  1. Start jolt: `just run`
  2. Navigate to Runners tab
  3. Select a repository to view Self-Hosted Runners
  4. Wait 60 seconds and observe automatic refresh
  5. Verify list updates with fresh data
  6. Navigate to a runner detail or switch tabs
  7. Return to runners list and verify timer restarts
  
- **Expected behavior**: 
  - First refresh occurs exactly 60 seconds after entering view
  - Subsequent refreshes occur every 60 seconds
  - Timer stops immediately on navigation away
  - Timer restarts when returning to runners list view
  - Loading state briefly appears during refresh
  - Selection is preserved across refreshes

## Related Documentation
- Architecture: `docs/ratatui-plan.md` - Phase 2 (GitHub Client)
- State management: `src/state/runners.rs` - Runner tab navigation state
- GitHub API client: `src/github/client.rs` - Runner data fetching
