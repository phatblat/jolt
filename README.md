# jolt

Interactive TUI for browsing GitHub Actions workflows, runs, jobs, and logs.

## Features

- **Three tabs**: Runners, Workflows, Console
- **Hierarchical navigation**: Owners → Repos → Workflows → Runs → Jobs → Logs
- **Breadcrumb trail**: Visual path showing current location
- **Log viewer**: Scrollable logs with line numbers
- **Console**: Error messages with timestamps and badges
- **State persistence**: Remembers last active tab between sessions

## Setup

This project uses [mise](https://mise.jdx.dev/) for tool management and Rust/Cargo for building.

### Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- GitHub personal access token

### Build

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Or using just
just build
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
cargo run

# Or using just
just run
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑`/`↓` | Navigate list / scroll logs |
| `←`/`→` | Horizontal scroll (logs) |
| `Enter` | Select / drill down |
| `Esc` | Go back |
| `Tab` | Switch tabs |
| `PgUp`/`PgDn` | Page scroll (logs) |
| `Home`/`End` | Jump to start/end (logs) |
| `r` | Refresh current view |
| `?` | Show help |
| `q` | Quit |

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
