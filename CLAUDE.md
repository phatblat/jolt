# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**jolt** is a Rust TUI application for browsing GitHub Actions workflow runs, jobs, logs, and runners. Built with ratatui for terminal rendering.

See `docs/ratatui-plan.md` for the full implementation plan.

## Architecture

```
src/
├── main.rs              # Entry point, terminal setup/cleanup
├── app.rs               # App state, event loop, tab management, persisted state
├── error.rs             # Error types (JoltError enum)
├── ui/
│   ├── mod.rs           # Main draw function, layout
│   ├── tabs.rs          # Tab bar rendering with badge support
│   ├── breadcrumb.rs    # Breadcrumb navigation
│   ├── list.rs          # Generic list widget
│   └── modal.rs         # Modal overlay widget
├── github/
│   ├── client.rs        # Authenticated HTTP client with rate limiting
│   ├── endpoints.rs     # API endpoint methods (runners, workflows, runs, jobs)
│   └── types.rs         # Data models (Owner, Workflow, Run, Job, Runner, etc.)
├── cache/
│   ├── paths.rs         # XDG-compliant cache path helpers
│   └── store.rs         # Serialized cache with TTL and invalidation
└── state/
    ├── navigation.rs    # Breadcrumb stack, ViewLevel enum
    ├── runners.rs       # Runner fetching and state (org + repo level)
    ├── workflows.rs     # Workflow/run/job fetching and state
    ├── sync.rs          # Background data sync
    └── analyze.rs       # Runner analysis and enrichment
```

## Common Development Tasks

### Building & Running

```bash
# Install Rust via mise
just tools

# Build debug binary
just build

# Run the TUI
just run

# Build release binary
just build-release
```

### Quality & Formatting

```bash
# Check formatting and linting
just lint

# Format code
just fmt
```

### Testing

```bash
# Run tests
just test
```

## Dependencies

- **ratatui** - Terminal UI framework
- **crossterm** - Cross-platform terminal manipulation
- **tokio** - Async runtime for non-blocking API calls
- **reqwest** - HTTP client for GitHub API
- **serde/serde_json** - JSON serialization
- **directories** - XDG-compliant cache paths
- **thiserror** - Error handling
- **chrono** - Date/time handling

## Key Implementation Details

- GitHub token from `GITHUB_TOKEN` environment variable
- Tab navigation: `Tab`/`Shift+Tab` to switch, arrow keys to navigate lists
- Breadcrumb navigation: `Enter` drills down, `Esc` goes back
- Local cache at `~/.cache/jolt/` with immutable log storage
- Fixed color palette for status indicators (see plan)

## Current Status (v1.0.0)

- [x] Phase 1: Scaffold — ratatui app loop, tab bar, quit on `q`
- [x] Phase 2: GitHub Client — authenticated API client with rate limiting, paginated fetches
- [x] Phase 3: Data & State — cache layer with TTL, navigation stack, runner/workflow state
- [x] Phase 4: UI — breadcrumb nav, generic list widget, modal overlay, runner analysis
- [x] Runners tab — org + repo-level runners with enrichment
- [x] Workflows tab — workflow/run/job browsing
- [x] Console tab — error/message display
- [x] Persisted state across sessions
- [x] `--version` / `-V` flag
- [x] Release binary via `just install`
