# jolt

Interactive TUI for browsing GitHub Actions workflows, runs, jobs, and logs.

## Features

- **Tab-based Navigation**: Workflows tab (Owners → Repos → Workflows → Runs → Jobs → Logs) and Runners tab (Repos → Runners → Runs → Jobs → Logs)
- **Log Viewer**: Full log content display with horizontal/vertical scrolling, page up/down, and jump to start/end
- **Job Status Display**: Visual status indicators with colors (green ✓, red ✗, yellow ⏳) and step-by-step breakdown for in-progress jobs
- **Performance**: Cache-first loading pattern with 5-minute TTL for responsive navigation
- **State Persistence**: Saves active tab, navigation position, and favorites across sessions
- **Console**: Error messages with timestamps and badges

## Installation

### Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- GitHub personal access token

### Install

```bash
# Install from local source
cargo install --path .

# Or build without installing
cargo build --release
```

## Usage

### Configuration

Set your GitHub token as an environment variable:

```bash
export GITHUB_TOKEN="ghp_your_token_here"
```

### Running

```bash
# Run the TUI
jolt
```

### Keyboard Shortcuts

| Key       | Action                       |
| --------- | ---------------------------- |
| Tab       | Switch tabs                  |
| ↑/↓       | Navigate lists / Scroll logs |
| ←/→       | Horizontal scroll in logs    |
| Enter     | Drill down / Select          |
| Esc       | Go back                      |
| PgUp/PgDn | Page scroll in logs          |
| Home/End  | Jump to start/end of logs    |
| r         | Refresh current view         |
| ?         | Show help                    |
| q         | Quit                         |

## Development

```bash
# Run tests
cargo test

# Lint code
just lint

# Format code
just fmt

# Clean build artifacts
just clean
```

## Architecture

Cache-first data loading for all levels of navigation:

| View Level   | Cache Path                                                                                   |
| ------------ | -------------------------------------------------------------------------------------------- |
| Owners       | ~/Library/Caches/jolt/owners.json                                                            |
| Repositories | ~/Library/Caches/jolt/owners/{owner}/repos.json                                              |
| Workflows    | ~/Library/Caches/jolt/owners/{owner}/repos/{repo}/workflows.json                             |
| Runs         | ~/Library/Caches/jolt/owners/{owner}/repos/{repo}/workflows/{id}/runs.json                   |
| Jobs         | ~/Library/Caches/jolt/owners/{owner}/repos/{repo}/workflows/{id}/runs/{id}/jobs.json         |
| Logs         | ~/Library/Caches/jolt/owners/{owner}/repos/{repo}/workflows/{id}/runs/{id}/jobs/{id}/log.txt |

Each view:

1. Shows cached data instantly on navigation (no loading spinner)
2. Fetches fresh data in the background and updates the display
3. Only shows errors if there's no cached data to fall back to

```
src/
├── main.rs           # Entry point
├── app.rs            # App state, event loop
├── ui/               # TUI rendering
│   ├── tabs.rs       # Tab bar
│   ├── breadcrumb.rs # Navigation breadcrumb
│   └── list.rs       # List widgets
├── github/           # GitHub API client
│   ├── client.rs     # HTTP client
│   ├── types.rs      # API types
│   └── endpoints.rs  # API endpoints
├── cache/            # Local filesystem cache
│   ├── store.rs      # Cache operations
│   └── paths.rs      # Cache paths
├── state/            # Tab state management
│   ├── navigation.rs # Nav stack
│   ├── workflows.rs  # Workflows tab
│   └── runners.rs    # Runners tab
└── error.rs          # Error types
```
