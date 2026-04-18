# Justfile for jolt
# Rust TUI for GitHub Actions workflow browsing

set unstable := true

#
# aliases
#

alias fmt := format
alias ls := list
alias up := upgrade

#
# recipes
#

# List available recipes
_default:
    @just --list

# List mise tools
list:
    mise ls --local

# Install dependencies
tools:
    mise install
    cargo fetch

# Upgrade mise tools
upgrade:
    mise upgrade --bump

# Check formatting and linting
lint:
    mise fmt --check
    just --fmt --check
    cargo fmt --check
    cargo clippy -- -D warnings

# Format code
format:
    mise fmt
    just --fmt
    cargo fmt

# Build debug binary
build:
    cargo build

# Build release binary
build-release:
    cargo build --release

# Build and install the release binary via cargo install
install: build-release
    cargo install --path .

# Run tests
test:
    cargo test

# Run the TUI
run:
    cargo run

# Clean build artifacts
clean:
    cargo clean
