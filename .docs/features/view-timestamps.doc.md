# View Timestamps

## Overview
Displays last data load timestamps in the breadcrumb area for Workflows and Runners tabs, helping users understand when data was refreshed from cache or API.

## User Perspective
Users see an ISO 8601 formatted timestamp (YYYY-MM-DD HH:MM:SS ±HHMM) in dark gray text on the right side of the breadcrumb line. The timestamp updates whenever data is loaded at that navigation level. For example, when viewing the list of workflows for a repository, the timestamp shows when that workflow list was last fetched. When drilling down to runs, the timestamp updates to show when that runs list was loaded.

## Data Flow
1. User navigates to a view level (e.g., owners, repositories, workflows, runs, jobs)
2. App loads data from cache or GitHub API
3. `SelectableList::set_loaded()` is called with the data
4. Timestamp field `last_updated` is set to current UTC time
5. UI reads `last_updated` from the current view's SelectableList
6. Breadcrumb rendering function formats timestamp to local timezone
7. Timestamp is rendered on the right side of the breadcrumb line

## Implementation

### Key Files
- `src/state/workflows.rs` - Added `last_updated: Option<DateTime<Utc>>` field to `SelectableList<T>`, updates in `set_loaded()`
- `src/ui/breadcrumb.rs` - Modified `draw_breadcrumb()` and `draw_runners_breadcrumb()` to accept and display timestamp parameter
- `src/ui/mod.rs` - Extracts appropriate timestamp from current view level and passes to breadcrumb functions

### Data Structure
- `SelectableList<T>` struct now includes:
  - `last_updated: Option<chrono::DateTime<chrono::Utc>>` - Timestamp when data was last loaded
- Updated in `set_loaded()` method with `chrono::Utc::now()`

### Timestamp Formatting
- Uses `format_timestamp()` helper function in `breadcrumb.rs`
- Converts UTC to local timezone using `chrono::Local`
- Formats as: `%Y-%m-%d %H:%M:%S %z` (ISO 8601 with timezone offset)
- Example: "2025-11-28 14:30:45 -0800"

### Rendering Strategy
- Breadcrumb paragraph is left-aligned
- Timestamp paragraph is right-aligned on same line
- Both use dark gray color (Color::DarkGray) for consistency with key help text
- Log viewer levels show no timestamp (they don't use SelectableList)

## Configuration
- Requires `chrono` crate (already in dependencies)
- No environment variables or feature flags

## Usage Example
```rust
// In state/workflows.rs
pub fn set_loaded(&mut self, items: Vec<T>, total_count: u64) {
    self.data = LoadingState::Loaded(PaginatedList::new(items, total_count));
    self.last_updated = Some(chrono::Utc::now());
    self.reset_selection();
}

// In ui/mod.rs
let timestamp = match app.workflows.nav.current() {
    ViewLevel::Owners => app.workflows.owners.last_updated,
    ViewLevel::Repositories { .. } => app.workflows.repositories.last_updated,
    ViewLevel::Workflows { .. } => app.workflows.workflows.last_updated,
    ViewLevel::Runs { .. } => app.workflows.runs.last_updated,
    ViewLevel::Jobs { .. } => app.workflows.jobs.last_updated,
    ViewLevel::Logs { .. } => None,
};
breadcrumb::draw_breadcrumb(frame, &breadcrumbs, chunks[1], timestamp);
```

## Testing
- Manual test: 
  1. Run `just run` to start jolt TUI
  2. Navigate through Workflows tab: owners -> repositories -> workflows -> runs -> jobs
  3. Verify timestamp appears on right side of breadcrumb at each level
  4. Verify timestamp updates when navigating to different levels
  5. Repeat for Runners tab: repositories -> runners -> runs -> jobs
  6. Verify no timestamp shown in log viewer
- Expected behavior: 
  - Timestamp shows current time in local timezone when data loads
  - Format matches ISO 8601 standard with timezone offset
  - Timestamp updates independently for each navigation level
  - Dark gray color matches surrounding UI elements

## Related Documentation
- Architecture: Breadcrumb navigation system
- State management: `SelectableList<T>` pattern for paginated data
