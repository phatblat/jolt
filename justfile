# Justfile for jolt
#
# settings

set unstable := true

#
# aliases
#

alias fmt := format
alias ls := list

# alias od := outdated

alias up := upgrade

#
# recipes
#

# List available recipes
_default:
    @just --list

list:
    mise ls --local

# Install dependencies using uv
install:
    mise install
    uv sync

upgrade:
    mise upgrade --bump

lint:
    mise fmt --check
    just --fmt --check
    uv run ruff check .

format:
    mise fmt
    just --fmt
    uv run ruff format .

# Lint and fix auto-fixable issues
lint-fix:
    uv run ruff check --fix .

# Build the project (install in editable mode)
build:
    uv pip install -e .

# Run tests (placeholder for when tests are added)
test:
    @echo "No tests configured yet"
    @echo "Run 'just test-run' to test the CLI with a real repo"

# Test the CLI with a sample command (requires GITHUB_TOKEN)
test-run:
    @echo "Testing jolt CLI..."
    uv run python jolt.py --help

# Run the jolt CLI with arguments

# Usage: just run --repo owner/repo [--workflow NAME] [--pr NUMBER]
[positional-arguments]
run *ARGS='':
    uv run python jolt.py "$@"

# Clean build artifacts and cache
clean:
    rm -rf .venv __pycache__ *.egg-info build dist .pytest_cache
    find . -type d -name __pycache__ -exec rm -rf {} +
    find . -type f -name "*.pyc" -delete
