# Harness Implementation Plan

Phased implementation plan for adding Harness CI/CD support to jolt.

## Overview

This plan outlines the steps to add Harness support alongside existing GitHub Actions functionality. The implementation is divided into 5 phases, each with specific deliverables and testing criteria.

## Phase 1: Harness Client Foundation

**Goal:** Establish basic Harness API client with authentication

### Tasks

- [ ] Create `src/harness/` module structure
  - [ ] `src/harness/mod.rs` - Module exports
  - [ ] `src/harness/client.rs` - Client struct and constructor
  - [ ] `src/harness/types.rs` - Response type definitions
  - [ ] `src/harness/error.rs` - Error types

- [ ] Implement `HarnessClient` struct
  - [ ] Constructor with `base_url`, `api_key`, `account_id`
  - [ ] `from_env()` method to read environment variables
  - [ ] HTTP client setup with default headers

- [ ] Environment variable handling
  - [ ] Read `HARNESS_API_KEY`
  - [ ] Read `HARNESS_ACCOUNT_ID`
  - [ ] Read `HARNESS_BASE_URL` (with default)

- [ ] Basic error handling
  - [ ] Define `HarnessError` enum
  - [ ] Implement conversions from `reqwest::Error`
  - [ ] Parse API error responses

- [ ] Add dependencies to `Cargo.toml`
  - [ ] `async-trait = "0.1"`
  - [ ] Ensure `reqwest` has `json` feature

### Success Criteria

- Client can be constructed from environment variables
- Authentication headers are set correctly
- Basic error handling works
- Unit tests pass

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_from_env() {
        std::env::set_var("HARNESS_API_KEY", "test-key");
        std::env::set_var("HARNESS_ACCOUNT_ID", "test-account");

        let client = HarnessClient::from_env().unwrap();
        assert_eq!(client.account_id, "test-account");
    }

    #[test]
    fn test_missing_env_vars() {
        std::env::remove_var("HARNESS_API_KEY");
        std::env::remove_var("HARNESS_ACCOUNT_ID");

        let result = HarnessClient::from_env();
        assert!(result.is_err());
    }
}
```

---

## Phase 2: Harness API Integration

**Goal:** Implement core API methods for organizations, projects, runners, executions, and logs

### Tasks

- [ ] Organization API
  - [ ] `list_organizations()` - List all orgs
  - [ ] Define `Organization` type
  - [ ] Parse response envelope
  - [ ] Handle pagination

- [ ] Project API
  - [ ] `list_projects(org_id)` - List projects in org
  - [ ] Define `Project` type
  - [ ] Handle empty results

- [ ] Runner API
  - [ ] `list_runners(org_id, project_id)` - List runners
  - [ ] `get_runner_details(runner_id)` - Get runner details
  - [ ] Define `Runner` and `RunnerDetails` types
  - [ ] Parse status enum (ACTIVE, INACTIVE, etc.)

- [ ] Execution API
  - [ ] `list_executions(org_id, project_id, filter)` - List executions
  - [ ] `get_execution_details(execution_id)` - Get execution details
  - [ ] Define `Execution`, `ExecutionDetails`, `ExecutionFilter` types
  - [ ] Parse status enum (Running, Success, Failed, etc.)
  - [ ] Handle POST request with JSON body

- [ ] Log API (HTTP only)
  - [ ] `fetch_logs(execution_id)` - Get execution logs
  - [ ] Define `LogLine` type
  - [ ] Handle pagination with `nextToken`
  - [ ] Parse log levels and timestamps

- [ ] Response parsing
  - [ ] Implement `parse_response()` helper
  - [ ] Handle standard response envelope
  - [ ] Extract data from nested structure
  - [ ] Handle API errors in envelope

### Success Criteria

- All API methods work against real Harness account
- Response types correctly deserialize
- Pagination works correctly
- Error responses are handled properly
- Unit and integration tests pass

### Testing

Create integration tests with mock server:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
    async fn test_list_organizations() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/ng/api/organizations"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS",
                "data": {
                    "content": [
                        {"identifier": "org1", "name": "Organization 1"}
                    ]
                }
            })))
            .mount(&mock_server)
            .await;

        let client = HarnessClient::new(
            mock_server.uri(),
            "test-key".to_string(),
            "test-account".to_string(),
        );

        let orgs = client.list_organizations().await.unwrap();
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].identifier, "org1");
    }
}
```

---

## Phase 3: Platform Abstraction

**Goal:** Create platform-agnostic abstraction layer

### Tasks

- [ ] Create `src/platform/` module
  - [ ] `src/platform/mod.rs` - Platform trait definition
  - [ ] `src/platform/types.rs` - Common types
  - [ ] `src/platform/github.rs` - GitHub implementation
  - [ ] `src/platform/harness.rs` - Harness implementation

- [ ] Define `Platform` trait
  - [ ] Authentication methods
  - [ ] Organizational structure methods
  - [ ] Runner methods
  - [ ] Execution methods
  - [ ] Log methods

- [ ] Define common types
  - [ ] `Scope` enum (Account, Organization, Project)
  - [ ] `Runner` struct (normalized)
  - [ ] `Execution` struct (normalized)
  - [ ] `LogLine` struct (normalized)

- [ ] Implement `Platform` for GitHub
  - [ ] Wrap existing GitHub client
  - [ ] Map GitHub types to common types
  - [ ] Map GitHub scopes (repo-based) to `Scope`

- [ ] Implement `Platform` for Harness
  - [ ] Wrap `HarnessClient`
  - [ ] Map Harness types to common types
  - [ ] Map Harness hierarchy to `Scope`

- [ ] Update `App` state
  - [ ] Store `Box<dyn Platform>` instead of concrete type
  - [ ] Add platform selector/switcher
  - [ ] Handle multi-platform scenarios

### Success Criteria

- Platform trait compiles and is usable
- Both GitHub and Harness implement Platform correctly
- App state works with trait objects
- Type conversions are correct
- Tests pass for both implementations

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    async fn test_platform_impl<P: Platform>(platform: P) {
        // Test authentication
        let authed = platform.is_authenticated();
        assert!(authed);

        // Test listing runners
        let scope = Scope::Account("test".to_string());
        let runners = platform.list_runners(scope).await.unwrap();
        assert!(!runners.is_empty());
    }

    #[tokio::test]
    async fn test_github_platform() {
        let github = GitHubPlatform::new("test-token");
        test_platform_impl(github).await;
    }

    #[tokio::test]
    async fn test_harness_platform() {
        let harness = HarnessPlatform::new("test-key", "test-account");
        test_platform_impl(harness).await;
    }
}
```

---

## Phase 4: UI Integration

**Goal:** Add Harness tab and navigation to TUI

### Tasks

- [ ] Update tab structure
  - [ ] Add `HarnessRunners` tab
  - [ ] Add `HarnessWorkflows` tab (or rename to Executions)
  - [ ] Update `Tab` enum
  - [ ] Add platform badge to tabs

- [ ] Implement breadcrumb navigation
  - [ ] Account → Org → Project hierarchy
  - [ ] Update `ui/breadcrumb.rs` for Harness
  - [ ] Handle navigation state

- [ ] Display runners
  - [ ] List runners in current scope
  - [ ] Show runner status with colors
  - [ ] Show runner details (capacity, running builds)
  - [ ] Handle empty state

- [ ] Display executions
  - [ ] List executions in current scope
  - [ ] Show execution status with colors
  - [ ] Show pipeline name, start time, duration
  - [ ] Handle filtering (all, running, queued)

- [ ] Display logs
  - [ ] Show logs for selected execution
  - [ ] Handle pagination
  - [ ] Add scroll support
  - [ ] Show loading state

- [ ] Update app event loop
  - [ ] Handle navigation events (Enter, Esc)
  - [ ] Handle tab switching
  - [ ] Handle scroll events
  - [ ] Trigger background data fetching

### Success Criteria

- Harness tabs render correctly
- Breadcrumb navigation works
- Runners display with correct status
- Executions display with correct status
- Logs display correctly
- Navigation is intuitive

### Testing

Manual testing checklist:
- [ ] Can switch to Harness tabs
- [ ] Can navigate org → project → runners
- [ ] Can navigate org → project → executions
- [ ] Runner status colors are correct
- [ ] Execution status colors are correct
- [ ] Can view logs for an execution
- [ ] Can scroll through long logs
- [ ] Can navigate back up hierarchy
- [ ] Error messages are clear

---

## Phase 5: Advanced Features

**Goal:** Add WebSocket streaming, improved caching, and configuration

### Tasks

- [ ] WebSocket log streaming
  - [ ] Add `tokio-tungstenite` dependency
  - [ ] Implement WebSocket connection
  - [ ] Authenticate WebSocket connection
  - [ ] Subscribe to log stream
  - [ ] Handle incoming log events
  - [ ] Update UI in real-time
  - [ ] Handle connection errors/reconnection

- [ ] Improved caching
  - [ ] Implement cache module for Harness
  - [ ] Create cache directory structure
  - [ ] Implement TTL-based cache invalidation
  - [ ] Cache organizations (1 hour TTL)
  - [ ] Cache projects (1 hour TTL)
  - [ ] Cache runners (30s TTL)
  - [ ] Cache executions (10s TTL)
  - [ ] Cache logs (immutable)
  - [ ] Background refresh for active views

- [ ] Configuration file
  - [ ] Add `toml` dependency
  - [ ] Define config structure
  - [ ] Read from `~/.config/jolt/config.toml`
  - [ ] Support platform enable/disable
  - [ ] Support default platform selection
  - [ ] Support custom base URLs

- [ ] Multi-platform support
  - [ ] Show both platforms in UI (configurable)
  - [ ] Allow switching between platforms
  - [ ] Unified view option (both platforms in one tab)
  - [ ] Platform filter/selector

- [ ] Polish
  - [ ] Add help text for Harness-specific keys
  - [ ] Improve error messages
  - [ ] Add loading indicators
  - [ ] Add retry logic with exponential backoff
  - [ ] Handle rate limiting gracefully

### Success Criteria

- Logs stream in real-time via WebSocket
- Caching reduces API calls significantly
- Config file works correctly
- Can use both GitHub and Harness simultaneously
- Error handling is robust
- Performance is good even with slow networks

### Testing

- [ ] WebSocket streaming works without drops
- [ ] Cache TTL works correctly
- [ ] Cache invalidation works
- [ ] Config file is parsed correctly
- [ ] Multi-platform view works
- [ ] Rate limiting is handled gracefully
- [ ] Long-running connections are stable

---

## Milestones

### M1: Basic Harness Client (Phases 1-2)
- Harness client can authenticate and make API calls
- All core API methods implemented
- Unit tests passing

### M2: Platform Abstraction (Phase 3)
- Platform trait defined and implemented
- Both GitHub and Harness use common interface
- App state refactored to use trait

### M3: UI Integration (Phase 4)
- Harness tabs functional
- Can view runners, executions, logs
- Navigation works correctly

### M4: Production Ready (Phase 5)
- WebSocket streaming works
- Caching improves performance
- Configuration file support
- Multi-platform support
- Ready for release

## Dependencies

### New Crates to Add

```toml
[dependencies]
async-trait = "0.1"              # Phase 1
tokio-tungstenite = "0.24"       # Phase 5
toml = "0.8"                     # Phase 5
```

### Dev Dependencies

```toml
[dev-dependencies]
wiremock = "0.6"                 # Phase 2 (integration tests)
```

## Risk Mitigation

### Risk: Harness API changes
- **Mitigation**: Version API requests, handle errors gracefully, log API responses for debugging

### Risk: WebSocket connection instability
- **Mitigation**: Implement reconnection logic, fall back to HTTP polling if WebSocket fails

### Risk: Rate limiting
- **Mitigation**: Implement exponential backoff, cache aggressively, warn user if approaching limits

### Risk: Complex organizational hierarchies
- **Mitigation**: Lazy-load projects only when navigated to, cache org/project structure

### Risk: Performance with large datasets
- **Mitigation**: Pagination, virtual scrolling, background loading, smart caching

## Documentation Updates

- [ ] Update README with Harness setup instructions
- [ ] Document environment variables
- [ ] Add configuration file example
- [ ] Add screenshots of Harness tabs
- [ ] Document keyboard shortcuts
- [ ] Add troubleshooting section

## Future Work (Post-Release)

- Support for additional platforms (GitLab CI, CircleCI, Jenkins)
- Graphical pipeline visualization
- Ability to trigger executions from TUI
- Ability to cancel/abort executions
- Export logs to file
- Search/filter across executions
- Custom views and saved filters
- Notifications for execution status changes
- Dashboard view with metrics
