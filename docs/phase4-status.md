# Phase 4 Implementation Status

## Completed ✅

### Infrastructure (Core Foundation)
- **Platform Badge Rendering** (`src/ui/platform_badge.rs`)
  - `render_badge()` - Creates styled `[GH]` or `[HR]` badges
  - `badge_text()`, `platform_color()` - Helper functions
  - `badge_line()`, `badge_line_styled()` - Convenience builders
  - Full unit test coverage

- **Platform Manager** (`src/app.rs`)
  - `PlatformManager` struct manages multiple CI/CD platforms
  - Auto-initialization from environment variables
  - `GITHUB_TOKEN` → GitHub platform
  - `HARNESS_API_KEY`, `HARNESS_ACCOUNT_ID`, `HARNESS_BASE_URL` → Harness platform
  - Methods: `platforms()`, `get_platform()`, `available_platforms()`, `has_platforms()`
  - Integrated into `App` state

- **Breadcrumb Platform Support** (`src/ui/breadcrumb.rs`)
  - `draw_breadcrumb_with_platform()` - Shows platform badge in navigation
  - `draw_runners_breadcrumb_with_platform()` - Platform badges for runners
  - Backward compatible with existing code

### Type Conversions (`src/types/mod.rs`)
- `Runner::from_github_enriched()` - Convert GitHub EnrichedRunner to unified Runner
  - Maps runner status (Online, Offline, Busy)
  - Extracts current job info (PR number, branch, start time)
  - Handles label extraction
  - Sets appropriate RunnerScope
  - Populates `os` field from GitHub runner data

### Platform Badges in All Lists (`src/ui/list.rs`)
- All list render functions display `[GH]` or `[HR]` platform badge at start of each item:
  - `render_owners_list()` - Owners with platform badge
  - `render_repositories_list()` - Repositories (Workflows tab) with badge
  - `render_runner_repositories_list()` - Repositories (Runners tab, kept for reference)
  - `render_workflows_list()` - Workflows with badge
  - `render_runs_list()` - Workflow runs with badge
  - `render_jobs_list()` - Jobs with badge
  - `render_runners_list()` - Runners with badge (kept for reference)
- **New unified render functions** (dynamic badges from `item.platform`):
  - `render_projects_list()` - Unified projects list with dynamic platform badges
  - `render_unified_runners_list()` - Unified runners list with OS, labels, job info

### Phase 4a: Runners Tab Unified Types ✅

Runners tab fully migrated from GitHub-specific types to unified platform-agnostic types.

#### State Layer (`src/state/runners.rs`)
- `RunnersViewLevel::Repositories` → `RunnersViewLevel::Projects`
- `SelectableList<Repository>` → `SelectableList<Project>` (unified type)
- `SelectableList<EnrichedRunner>` → `SelectableList<Runner>` (unified type)
- `owner`/`repo` fields → `org_id`/`project_id` (platform-agnostic naming)
- `run_id: u64` / `job_id: u64` → `String` (platform-agnostic IDs)
- Platform-aware title and breadcrumb methods:
  - `title_for_platform()` - "Repositories" (GitHub) vs "Projects" (Harness)
  - `breadcrumb_label_for_platform()` - "Repos" (GitHub) vs "Projects" (Harness)
  - "Runs" (GitHub) vs "Executions" (Harness), "Jobs" vs "Stages"
- `current_platform()` method derives platform from selected project

#### App Layer (`src/app.rs`)
- All Runners tab data loading uses unified types with conversion at platform boundaries
- GitHub `EnrichedRunner` → unified `Runner` via `Runner::from_github_enriched()`
- GitHub `Repository` → unified `Project` via inline conversion (cache-compatible)
- String-based IDs with `parse::<u64>()` at GitHub API call sites
- Auto-refresh loop uses unified types

#### UI Layer (`src/ui/mod.rs`, `src/ui/list.rs`)
- `draw_runners_tab()` uses `render_projects_list()` and `render_unified_runners_list()`
- Platform-aware breadcrumbs rendered via `draw_runners_breadcrumb_with_platform()`
- Dynamic platform badges from `item.platform` field (not hardcoded)

#### Platform Layer (`src/platform/github.rs`, `src/platform/harness.rs`)
- `GitHubPlatform::list_organizations()` - Real API calls (user + orgs)
- `GitHubPlatform::list_projects()` - Real API calls (user repos vs org repos)
- `GitHubPlatform::list_runners()` - Real API calls with scope parsing
- `map_repository_to_project()` - New mapper function with tests
- `map_owner_to_organization()` - Tested for both User and Organization types
- Harness `map_runner()` sets `os: None`

### Testing
- All tests passing
- Clippy clean with `-D warnings`
- Builds successfully
- New unit tests: `test_map_owner_org_type`, `test_map_repository_to_project`

### Phase 4b: Workflows Tab Unified Types ✅

Workflows tab fully migrated from GitHub-specific types (Owner, Repository) to unified platform-agnostic types (Organization, Project), matching the same hybrid pattern used for the Runners tab in Phase 4a.

#### State Layer (`src/state/workflows.rs`)
- `SelectableList<Owner>` → `SelectableList<Organization>` (unified type)
- `SelectableList<Repository>` → `SelectableList<Project>` (unified type)
- Added `current_platform()` method deriving platform from selected organization
- All match arms updated: `ViewLevel::Owners` → `Organizations`, `Repositories` → `Projects`

#### Navigation Layer (`src/state/navigation.rs`)
- `ViewLevel::Owners` → `ViewLevel::Organizations` with `#[serde(alias)]` for backward compat
- `ViewLevel::Repositories` → `ViewLevel::Projects` with `#[serde(alias)]` for backward compat
- Field renames: `owner` → `org_id`, `repo` → `project_id` (with serde aliases)
- Platform-aware methods: `title_for_platform()`, `to_breadcrumb_for_platform()`
- `breadcrumbs_for_platform()` on NavigationStack for platform-aware breadcrumb trails
- New test: `test_breadcrumbs_github_platform`

#### Types Layer (`src/types/mod.rs`)
- Added `visibility: Option<bool>` to unified `Project` (maps from GitHub `private` field)
- Added `updated_at: Option<DateTime<Utc>>` to unified `Project` (maps from GitHub `updated_at`)

#### App Layer (`src/app.rs`)
- ~80 match arms updated for ViewLevel renames across all workflows methods
- `load_current_view()` converts GitHub Owner/Repository to unified Organization/Project
  at platform boundaries via `map_owner_to_organization()` and `map_repository_to_project()`
- Cache layer still stores GitHub-native types; conversion happens after cache read
- Updated: `handle_workflows_enter()`, `toggle_workflows_favorite()`,
  `get_workflows_github_url()`, `save_workflows_to_analyze()`, `go_to_workflows_source()`

#### UI Layer (`src/ui/list.rs`, `src/ui/mod.rs`)
- `render_owners_list()` → `render_organizations_list()` using unified `Organization` type
- `render_repositories_list()` → `render_workflow_projects_list()` using unified `Project` type
- Dynamic platform badges from `org.platform` / `project.platform` (not hardcoded)
- `render_workflows_list()`, `render_runs_list()`, `render_jobs_list()` now accept `Platform` parameter
- `draw_workflows_tab()` passes platform from `current_platform()` to all render functions
- `draw_runners_tab()` also updated to pass platform to shared render functions
- Platform-aware breadcrumbs via `breadcrumbs_for_platform()`

#### Platform Layer (`src/platform/github.rs`, `src/platform/harness.rs`)
- `map_repository_to_project()` populates new `visibility` and `updated_at` fields
- `map_project()` (Harness) sets new fields to `None`
- Existing tests updated to verify new fields

## Remaining Work 🚧

### Phase 4c: Polish

#### 1. Add Platform Filtering
- Add `platform_filter: Option<Platform>` to `App` state
- Press 'f' to cycle through: All → GitHub → Harness → All
- Filter data before rendering based on `platform_filter`
- Show current filter state in UI (e.g., in status bar)

#### 2. Handle Platform-Specific Behavior
- GitHub: Runners are repository-scoped
- Harness: Runners can be org-scoped, project-scoped, or account-scoped
- Navigation needs to handle different scoping models

#### 3. Error handling and performance
- Error handling for platform-specific issues
- Loading indicators for parallel queries
- Performance optimization

### Steps Navigation Level (Deferred)
Adding the 7th level (Steps between Jobs and Logs) was started but reverted due to scope:
- Requires updating `ViewLevel` and `RunnersViewLevel` enums
- Requires updating ~15+ match statements across codebase
- Affects workflows tab navigation, UI rendering, breadcrumbs
- Should be done as a separate focused task after multi-platform integration

## Implementation Strategy

### Incremental Migration (In Progress)

**Phase 4a: Runners Tab Only** ✅ Complete
1. ✅ Add `Runner::from_github_enriched()` type conversion
2. ✅ Add platform badges to runner lists (all list renders)
3. ✅ Convert Runners tab state to use unified types
4. ✅ Update `load_runners_view()` with unified type conversions at boundary
5. ✅ Platform-aware breadcrumbs and titles
6. **Milestone**: Runners tab uses unified types with dynamic platform badges

**Phase 4b: Workflows Tab** ✅ Complete
1. ✅ Platform badges already added to all workflow lists
2. ✅ Convert Workflows tab state to use unified types (Organization, Project)
3. ✅ Update `load_current_view()` with type conversions at boundary
4. ✅ Platform-aware breadcrumbs, titles, and dynamic badges
5. **Milestone**: Workflows tab uses unified types with dynamic platform badges

**Phase 4c: Polish** (Final touches)
1. Add platform filtering ('f' key)
2. Error handling for platform-specific issues
3. Loading indicators for parallel queries
4. Performance optimization
5. **Milestone**: Phase 4 complete

## Testing Plan

For each increment:
1. **Unit Tests**: Unified type conversions, mapper functions
2. **Integration Tests**: Multi-platform queries
3. **Manual Testing**:
   - Set both `GITHUB_TOKEN` and `HARNESS_*` env vars
   - Navigate through all levels
   - Verify badges show correctly
   - Verify data from both platforms appears
   - Test with only GitHub (Harness unavailable)
   - Test with only Harness (GitHub unavailable)
   - Test with neither (graceful degradation)

## Current State

**Phase 4a (Runners Tab)**: Complete ✅
**Phase 4b (Workflows Tab)**: Complete ✅
**Phase 4c (Polish)**: Not started 🚧
**Next Step**: Add platform filtering, error handling, and performance optimizations

## Files Changed in Phase 4a

### Types
- `src/types/mod.rs` - Added `os: Option<String>` to unified Runner

### Platform
- `src/platform/github.rs` - Real API implementations, `map_repository_to_project()`, tests
- `src/platform/harness.rs` - Set `os: None` in mapper

### State
- `src/state/runners.rs` - Migrated to unified types, platform-aware methods

### UI
- `src/ui/list.rs` - `render_projects_list()`, `render_unified_runners_list()`
- `src/ui/mod.rs` - Wired unified render functions, platform-aware breadcrumbs
- `src/ui/breadcrumb.rs` - `#[allow(dead_code)]` on unused convenience wrapper

### App
- `src/app.rs` - Unified type conversions at boundaries, string-based IDs

## Files Changed in Phase 4b

### Types
- `src/types/mod.rs` - Added `visibility` and `updated_at` fields to unified Project

### Platform
- `src/platform/github.rs` - Populate new Project fields from GitHub Repository
- `src/platform/harness.rs` - Set new Project fields to `None`

### State
- `src/state/navigation.rs` - ViewLevel renames, serde aliases, platform-aware methods
- `src/state/workflows.rs` - Migrated to unified Organization/Project types

### UI
- `src/ui/list.rs` - Renamed/updated render functions for unified types, dynamic badges
- `src/ui/mod.rs` - Platform-aware breadcrumbs and render function wiring

### App
- `src/app.rs` - ~80 match arm updates, type conversions at cache/API boundaries
