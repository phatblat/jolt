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

### Testing
- All 21 tests passing
- Clippy clean with `-D warnings`
- Builds successfully

## Remaining Work 🚧

### Data Integration (Major Refactoring Required)

#### 1. Convert UI Components to Unified Types
Currently all UI components use GitHub-specific types. Need to:
- **Update `src/ui/list.rs`**:
  - Convert from `crate::github::*` to `crate::types::*`
  - `EnrichedRunner` → `Runner` (unified)
  - `WorkflowRun` → `Execution` (unified)
  - `Job` → `Job` (unified)
  - Update all render functions to accept unified types

- **Update `src/state/workflows.rs`**:
  - Change `SelectableList<Owner>` → `SelectableList<Organization>`
  - Change `SelectableList<Repository>` → `SelectableList<Project>`
  - Update all data structures to use unified types

- **Update `src/state/runners.rs`**:
  - Same conversion to unified types
  - Update navigation to handle multi-platform data

#### 2. Multi-Platform Data Fetching
Update `load_current_view()` and `load_runners_view()` in `src/app.rs`:

```rust
// Current (GitHub only):
let result = self
    .github_client
    .as_mut()
    .unwrap()
    .get_enriched_runners(&owner, &repo, 1, 30)
    .await;

// Target (Multi-platform):
let mut all_runners = Vec::new();
for platform in self.platform_manager.platforms_mut() {
    if let Ok(runners) = platform.list_runners(scope).await {
        all_runners.extend(runners);
    }
}
// Sort and deduplicate
all_runners.sort_by(|a, b| a.name.cmp(&b.name));
```

#### 3. Display Platform Badges in Lists
Once unified types are used, add platform badges to all list items:

```rust
// In render function:
let badge = platform_badge::render_badge(item.platform);
Line::from(vec![
    badge,
    Span::raw(" "),
    Span::styled(&item.name, style),
])
```

#### 4. Handle Platform-Specific Behavior
- GitHub: Runners are repository-scoped
- Harness: Runners can be org-scoped, project-scoped, or account-scoped
- Navigation needs to handle different scoping models
- May need to add "scope" field to runner display

#### 5. Add Platform Filtering
- Add `platform_filter: Option<Platform>` to `App` state
- Press 'f' to cycle through: All → GitHub → Harness → All
- Filter data before rendering based on `platform_filter`
- Show current filter state in UI (e.g., in status bar)

### Steps Navigation Level (Deferred)
Adding the 7th level (Steps between Jobs and Logs) was started but reverted due to scope:
- Requires updating `ViewLevel` and `RunnersViewLevel` enums
- Requires updating ~15+ match statements across codebase
- Affects workflows tab navigation, UI rendering, breadcrumbs
- Should be done as a separate focused task after multi-platform integration

## Implementation Strategy

### Recommended Approach: Incremental Migration

**Phase 4a: Runners Tab Only** (Smaller Scope)
1. Convert Runners tab state to use unified types
2. Update `load_runners_view()` to query multiple platforms
3. Add platform badges to runner lists
4. Test with both GitHub and Harness
5. **Milestone**: Runners tab shows mixed GitHub + Harness runners

**Phase 4b: Workflows Tab** (After 4a works)
1. Convert Workflows tab state to use unified types
2. Update `load_current_view()` for multi-platform
3. Handle organization/project navigation
4. Add platform badges throughout
5. **Milestone**: Workflows tab shows unified navigation

**Phase 4c: Polish** (Final touches)
1. Add platform filtering ('f' key)
2. Error handling for platform-specific issues
3. Loading indicators for parallel queries
4. Performance optimization
5. **Milestone**: Phase 4 complete

### Alternative Approach: All-at-Once Refactor

Convert everything to unified types in one large PR:
- More risky (many changes, harder to test incrementally)
- Requires careful coordination
- But once done, everything is consistent

## Testing Plan

For each increment:
1. **Unit Tests**: Unified type conversions
2. **Integration Tests**: Multi-platform queries
3. **Manual Testing**:
   - Set both `GITHUB_TOKEN` and `HARNESS_*` env vars
   - Navigate through all levels
   - Verify badges show correctly
   - Verify data from both platforms appears
   - Test with only GitHub (Harness unavailable)
   - Test with only Harness (GitHub unavailable)
   - Test with neither (graceful degradation)

## Risk Assessment

### High Risk Items
- **Type conversions**: Easy to miss a field or get mapping wrong
- **Parallel queries**: Error handling when one platform fails
- **Performance**: Two API calls instead of one (mitigate with caching)
- **Navigation differences**: GitHub repo-centric vs Harness org/project-centric

### Mitigation Strategies
- Start with Runners tab (simpler than Workflows)
- Add extensive logging during development
- Test with real Harness account, not just mocks
- Cache aggressively to reduce API calls
- Handle errors gracefully (show data from available platforms)

## Estimated Effort

- **Phase 4a (Runners Only)**: 4-6 hours
- **Phase 4b (Workflows)**: 6-8 hours
- **Phase 4c (Polish)**: 2-3 hours
- **Total**: 12-17 hours of focused development

## Current State

**Infrastructure**: Complete and committed ✅
**Data Integration**: Not started 🚧
**Next Step**: Begin Phase 4a by converting Runners tab to unified types

## Files That Need Changes

### Core Data Structures
- `src/state/runners.rs` - Change to unified types
- `src/state/workflows.rs` - Change to unified types
- `src/app.rs` - Update data fetching logic

### UI Rendering
- `src/ui/list.rs` - Update all render functions
- `src/ui/mod.rs` - Update view rendering to use platform badges

### Testing
- Add integration tests for multi-platform queries
- Add tests for platform filtering
- Add tests for error handling
