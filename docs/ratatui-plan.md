# Jolt Ratatui TUI Plan

This document outlines the plan to rewrite jolt from a Python CLI to a Rust TUI application using ratatui.

## Overview

**Goal:** Interactive TUI for browsing GitHub Actions workflow runs, jobs, logs, and runners.

**Key Decisions:**
- Platform: macOS only (initially)
- Auth: `GITHUB_TOKEN` environment variable
- Navigation: Arrow keys only
- Refresh: Manual (`r` key)
- Search: Plain text filtering (regex planned for later)
- Caching: Local filesystem cache with manual refresh
- Timestamps: Relative ("2h ago") in UI
- Log viewer: Horizontal scroll (no line wrap)
- Status colors: Fixed palette

## UI Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  [Runners]  [Workflows]  [Console (3)]               ← Tabs     │
├─────────────────────────────────────────────────────────────────┤
│  phatblat > repo-name > workflow > run #123         ← Breadcrumb│
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ > Job 1 - build (failed)                    2m 34s      │   │
│  │   Job 2 - test (success)                    1m 12s      │   │
│  │   Job 3 - deploy (skipped)                  -           │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                     ← List View │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│  ↑↓ Navigate  ↵ Select  Esc Back  r Refresh  / Search  q Quit  │
└─────────────────────────────────────────────────────────────────┘
```

The Console tab shows a badge with unread error count (e.g., `Console (3)`). Badge clears when tab is viewed.

## Data Model

### Owner

Unified concept representing either a GitHub user or organization.

```rust
struct Owner {
    login: String,
    owner_type: OwnerType,  // User or Organization
    avatar_url: Option<String>,
}

enum OwnerType {
    User,
    Organization,
}
```

The current authenticated user's personal repos appear under their username as an owner, alongside any organizations they belong to.

## Navigation Model

### Tab: Runners

Shows repos where user has admin access AND repo has runners configured.

```
Repos with Runners
    └── Runners for Repo
            └── Workflow Runs (for runner)
                    └── Jobs
                            └── Log Viewer
```

| Level | Columns | Sort |
|-------|---------|------|
| Repos | name, owner, runner count, last activity | alphabetical |
| Runners | name, status, current job | status, name |
| Workflow Runs | state, timestamp, workflow, run #, PR # | most recent first |
| Jobs | name, status, duration | run order |
| Logs | scrollable, searchable text | - |

### Tab: Workflows

Hierarchical navigation through GitHub's object model.

```
Owners (users/orgs)
    └── Repositories
            └── Workflows
                    └── Workflow Runs
                            └── Jobs
                                    └── Log Viewer
```

| Level | Columns | Sort |
|-------|---------|------|
| Owners | login, type (user/org) | alphabetical |
| Repositories | name, visibility, last updated | most recently updated |
| Workflows | name, state, path | alphabetical |
| Workflow Runs | status, timestamp, run #, PR #, branch | descending (newest first) |
| Jobs | name, status, duration, runner | run order |
| Logs | scrollable, searchable text | - |

### Tab: Console

Displays errors, warnings, and diagnostic messages. No drill-down navigation.

```
Console Messages (scrollable list)
    - [ERROR] 2h ago: 401 Unauthorized - GET /repos/foo/bar/actions/runners
    - [ERROR] 3h ago: Rate limit exceeded (resets in 42m)
    - [WARN]  5h ago: Cache miss for workflow 12345
```

| Column | Description |
|--------|-------------|
| Level | ERROR, WARN, INFO |
| Timestamp | Relative ("2h ago") |
| Message | Error description with context |

**Badge behavior:**
- Badge shows count of unread errors (ERROR level only)
- Badge clears when Console tab is selected
- Tab title changes color (red) when errors exist

## Empty States

When a list has no items, display a centered placeholder:

| View | Placeholder Text |
|------|------------------|
| Owners | "No accessible owners found" |
| Repositories | "No repositories found" |
| Workflows | "No workflows in this repository" |
| Workflow Runs | "No workflow runs found" |
| Jobs | "No jobs in this run" |
| Runners | "No runners configured" |
| Repos with Runners | "No repositories with runners" |
| Console | "No messages" |

## Pagination

Lists load more data automatically when scrolling near the bottom.

**Behavior:**
- Trigger: Cursor within 5 items of list end
- Indicator: "Loading more..." row appended to list
- Page size: 30 items per request (GitHub default)
- End state: No indicator when all pages loaded

**Cached pagination:**
- Store page cursors in cache for resumable fetching
- On refresh (`r`), invalidate and reload from first page

## Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate list |
| `←` / `→` | Horizontal scroll (log viewer) |
| `Enter` | Drill down / select |
| `Esc` | Navigate back up / clear search |
| `Tab` | Switch tabs |
| `r` | Refresh current view |
| `/` | Start search (filters lists, searches logs) |
| `n` / `N` | Next/prev search match (log viewer) |
| `q` | Quit |
| `?` | Show help |

## Local Cache Structure

Base directory: `~/.cache/jolt/`

```
~/.cache/jolt/
├── state.json                           # Last selected tab, positions
├── owners/
│   └── {owner}/
│       ├── owner.json                   # Owner metadata (type, avatar, etc.)
│       └── repos/
│           └── {repo}/
│               ├── repo.json            # Repository metadata
│               ├── runners/
│               │   └── {runner_id}.json # Runner metadata
│               └── workflows/
│                   └── {workflow_id}/
│                       ├── workflow.json
│                       └── runs/
│                           └── {run_id}/
│                               ├── run.json
│                               └── jobs/
│                                   └── {job_id}/
│                                       ├── job.json
│                                       └── log.txt
```

### Cache Behavior

- **Mutable data** (runners, active runs): Cache with TTL, manual refresh invalidates
- **Immutable data** (completed runs, logs): Cache permanently
- **Run state tracking**: Cache run `status`; if `completed`, logs are immutable

## GitHub API Endpoints

### Authentication
- Token via `GITHUB_TOKEN` env var
- Header: `Authorization: Bearer {token}`

### Endpoints Required

| Endpoint | Purpose |
|----------|---------|
| `GET /user` | Current authenticated user |
| `GET /user/orgs` | List user's organizations |
| `GET /user/repos` | List user's accessible repos |
| `GET /orgs/{org}/repos` | List org repos |
| `GET /repos/{owner}/{repo}` | Repo details |
| `GET /repos/{owner}/{repo}/actions/runners` | Repo runners (admin required) |
| `GET /repos/{owner}/{repo}/actions/workflows` | List workflows |
| `GET /repos/{owner}/{repo}/actions/runs` | List workflow runs |
| `GET /repos/{owner}/{repo}/actions/runs/{run_id}` | Run details |
| `GET /repos/{owner}/{repo}/actions/runs/{run_id}/jobs` | Jobs for run |
| `GET /repos/{owner}/{repo}/actions/jobs/{job_id}/logs` | Download job logs |

### Rate Limiting
- Display remaining quota in status bar
- Graceful handling when rate limited (show message, don't crash)

## Architecture

### Crate Structure

```
jolt/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, arg parsing
│   ├── app.rs               # App state, event loop
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── tabs.rs          # Tab bar rendering (with badge support)
│   │   ├── breadcrumb.rs    # Breadcrumb rendering
│   │   ├── list.rs          # Generic list widget
│   │   ├── log_viewer.rs    # Log display with search
│   │   ├── console.rs       # Console message list
│   │   └── help.rs          # Help overlay
│   ├── github/
│   │   ├── mod.rs
│   │   ├── client.rs        # HTTP client wrapper
│   │   ├── types.rs         # API response types
│   │   └── endpoints.rs     # API endpoint functions
│   ├── cache/
│   │   ├── mod.rs
│   │   ├── store.rs         # File-based cache
│   │   └── paths.rs         # Cache path utilities
│   ├── state/
│   │   ├── mod.rs
│   │   ├── navigation.rs    # Nav stack, breadcrumbs
│   │   ├── runners.rs       # Runners tab state
│   │   ├── workflows.rs     # Workflows tab state
│   │   └── console.rs       # Console messages, badge state
│   └── error.rs             # Error types
```

### Dependencies

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"           # XDG paths
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
```

### Key Patterns

1. **Navigation Stack**: Each tab maintains a stack of views. `Esc` pops, `Enter` pushes.

2. **Async Data Loading**: Use tokio channels to fetch data without blocking UI.

3. **State Machine**: Each view has states: `Loading`, `Loaded(data)`, `Error(msg)`.

4. **List Selection**: Generic `StatefulList<T>` wrapping ratatui's `ListState`.

## Implementation Phases

### Phase 1: Scaffold
- [ ] Initialize Rust project with dependencies
- [ ] Basic ratatui app loop (quit on `q`)
- [ ] Tab bar rendering (non-functional)
- [ ] Placeholder content per tab

### Phase 2: GitHub Client
- [ ] HTTP client with auth
- [ ] Implement core endpoints
- [ ] Response type definitions (Owner, Repo, Workflow, Run, Job)
- [ ] Error handling for 401/404/rate-limit

### Phase 3: Cache Layer
- [ ] Directory structure creation
- [ ] JSON serialization/deserialization
- [ ] Cache read/write operations
- [ ] TTL checking for mutable data

### Phase 4: Workflows Tab
- [ ] Owners list view (user + orgs combined)
- [ ] Navigation stack implementation
- [ ] Breadcrumb rendering
- [ ] Repositories list
- [ ] Workflows list
- [ ] Workflow runs list
- [ ] Jobs list
- [ ] Pagination (load more on scroll)
- [ ] Loading indicator for pagination

### Phase 5: Log Viewer
- [ ] Log download and caching
- [ ] Scrollable log display
- [ ] Plain text search
- [ ] Search highlighting
- [ ] Next/prev match navigation

### Phase 6: Runners Tab
- [ ] Fetch repos with runner access
- [ ] Filter to repos with runners
- [ ] Runner list per repo
- [ ] Runner detail → workflow runs

### Phase 7: Console Tab
- [ ] Console message data structure (level, timestamp, message)
- [ ] Console tab rendering
- [ ] Error routing (API errors → console)
- [ ] Badge state (unread count)
- [ ] Badge clear on tab view

### Phase 8: Polish
- [ ] Loading indicators
- [ ] Empty state placeholders
- [ ] Rate limit display in status bar
- [ ] Help overlay (`?`)
- [ ] State persistence (last position)

### Phase 9: Testing
- [ ] Unit tests for cache layer
- [ ] Unit tests for state management
- [ ] Integration tests with mock API
- [ ] Manual testing checklist

## Status Colors

Fixed color palette for status indicators:

| Status | Color |
|--------|-------|
| Success / Completed | Green |
| Failed / Error | Red |
| In Progress / Running | Yellow |
| Queued / Waiting | Blue |
| Cancelled / Skipped | Gray |
| Offline (runner) | Gray |
| Online / Idle (runner) | Green |
| Busy (runner) | Yellow |

## Future Enhancements (Out of Scope)

- Linux/Windows support
- GitHub Enterprise
- Org-level runners
- Runner labels display
- Regex search
- Multiple token profiles
- Webhook-based live updates
- Log annotation parsing (error grouping)
