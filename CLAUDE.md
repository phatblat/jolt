# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**jolt** (JOb Log Ticket) is a CLI tool that queries GitHub Actions workflow logs to find and display failed jobs, making it easier to extract failure information for tickets and debugging.

## Architecture

The project is single-file Python application (`jolt.py`) with a clear separation of concerns:

1. **GitHubClient** - HTTP wrapper around the GitHub REST API
   - Handles authentication via Bearer token
   - Methods: `get_workflow_runs()`, `get_pr_workflow_runs()`, `get_failed_jobs()`
   - Uses requests library with persistent session

2. **Display & Formatting** - Rich library integration for CLI output
   - `display_failures()` - Main output function rendering workflow failures in panels/tables
   - `format_time_ago()` - Helper for human-readable timestamps

3. **CLI Interface** - Click command with options for repo, workflow, PR filtering
   - Entry point: `main()` function
   - Command-line args: `--repo`, `--workflow`, `--pr`, `--limit`, `--token`

The application follows a simple pipeline: parse args → create API client → fetch runs → display results

## Common Development Tasks

### Building & Running

```bash
# Install dependencies (uv + mise)
just install

# Run with arguments
just run --repo owner/repo [--workflow NAME] [--pr NUMBER]

# Quick test
just test-run  # Runs with --help

# Editable install
just build
```

### Quality & Formatting

```bash
# Check linting (ruff, justfile format, mise format)
just lint

# Auto-fix issues
just lint-fix

# Format code
just fmt
```

### Testing

Tests are not yet implemented. When adding tests, ensure they:
- Cover new functionality
- Use uv/pytest infrastructure
- Have pristine output

### Dependencies

- **requests** - GitHub API HTTP client
- **click** - CLI argument parsing and decoration
- **rich** - Terminal output formatting (panels, tables, colors)
- **ruff** - Linting and formatting (dev only)

## Key Implementation Details

- GitHub token sourced from `GITHUB_TOKEN` env var or `--token` flag
- API version pinned to `2022-11-28`
- Handles pagination with `per_page` parameter (default 20-50)
- PR filtering matches on commit SHA or pull_requests references
- Failed job detection checks job `conclusion` field
- No persistent state or caching between runs

## Development Guidelines

- The justfile uses `uv run` to execute Python with proper virtual environment
- Arguments passed to `just run` use `"$@"` to preserve quoted strings
- Rich library provides styled console output (colors, underlines, panels)
- HTTP errors include user-friendly messages for 404/401/other failures
