# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**jolt** is a Rust TUI application for browsing GitHub Actions workflow runs, jobs, logs, and runners. Built with ratatui for terminal rendering.

See `docs/ratatui-plan.md` for the full implementation plan.

## Architecture

```
src/
├── main.rs              # Entry point, terminal setup/cleanup
├── app.rs               # App state, event loop, tab management
├── ui/
│   ├── mod.rs           # Main draw function, layout
│   ├── tabs.rs          # Tab bar rendering with badge support
│   ├── breadcrumb.rs    # Breadcrumb navigation (planned)
│   ├── list.rs          # Generic list widget (planned)
│   ├── log_viewer.rs    # Log display with search (planned)
│   ├── console.rs       # Console message list (planned)
│   └── help.rs          # Help overlay (planned)
├── github/              # GitHub API client (planned)
├── cache/               # Local filesystem cache (planned)
├── state/               # Navigation and tab state (planned)
└── error.rs             # Error types (planned)
```

## Common Development Tasks

### Building & Running

```bash
# Install Rust via mise
just install

# Build debug binary
just build

# Run the TUI
just run

# Build release binary
just release
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

## Current Status

Phase 1 (Scaffold) complete:
- [x] Basic ratatui app loop
- [x] Tab bar with Runners/Workflows/Console
- [x] Placeholder content per tab
- [x] Quit on `q`

Next: Phase 2 (GitHub Client)
