# ci-failures

CLI tool to view GitHub Actions job failures.

## Setup

This project uses [mise](https://mise.jdx.dev/) for tool management and [uv](https://docs.astral.sh/uv/) for Python package management.

### Install Dependencies

```bash
# Install dependencies
just install

# Or manually with uv
uv sync
```

## Usage

### Using Just

```bash
# List available recipes
just

# Run the CLI
just run --repo owner/repo
just run --repo owner/repo --workflow "CI"
just run --repo owner/repo --pr 1234
just run --help

# Test the CLI
just test-run
```

### Direct Usage

```bash
# Using uv run
uv run python ci_failures.py --repo owner/repo

# Or activate the venv
source .venv/bin/activate
python ci_failures.py --repo owner/repo
```

## Configuration

Set your GitHub token as an environment variable:

```bash
export GITHUB_TOKEN="ghp_your_token_here"
```

Or pass it via the `--token` flag:

```bash
just run --repo owner/repo --token ghp_your_token
```

## Examples

```bash
# View recent failures for a repo
just run --repo launchdarkly/android-client-sdk

# Filter by workflow name (partial match)
just run --repo launchdarkly/android-client-sdk --workflow "CI"

# Filter by PR number
just run --repo launchdarkly/android-client-sdk --pr 1234

# Limit results
just run --repo launchdarkly/android-client-sdk --limit 5
```

## Development

```bash
# Install dependencies (including dev tools)
just install

# Build (install in editable mode)
just build

# Run tests
just test

# Lint code
just lint

# Lint and auto-fix issues
just lint-fix

# Format code
just fmt

# Clean build artifacts
just clean
```
