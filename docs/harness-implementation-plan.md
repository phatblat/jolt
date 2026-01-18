# Harness Implementation Plan

Phased implementation plan for adding Harness CI/CD support to jolt with unified navigation.

## Overview

This plan outlines the steps to add Harness support alongside existing GitHub Actions functionality using a **unified navigation approach**. Since GitHub Actions and Harness have identical 7-level hierarchies, we can integrate both platforms into a single navigation flow with platform badges for distinction.

The implementation is divided into 5 phases, each with specific deliverables and testing criteria.

## Key Architecture Decision

**Unified Navigation**: Both platforms share the same UI and navigation structure since their hierarchies map 1:1:
- GitHub: Owner → Repository → Workflow → Run → Job → Step → Logs
- Harness: Organization → Project → Pipeline → Execution → Stage → Step → Logs

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

## Phase 3: Unified Types and Platform Abstraction

**Goal:** Create unified types that work for both platforms and platform abstraction layer

### Tasks

- [ ] Create `src/types/` module for unified types
  - [ ] `Organization` - Works for both GitHub Owner and Harness Organization
  - [ ] `Project` - Works for both GitHub Repository and Harness Project
  - [ ] `Workflow` - Works for both GitHub Workflow and Harness Pipeline
  - [ ] `Execution` - Works for both GitHub Run and Harness Execution
  - [ ] `Job` - Works for both GitHub Job and Harness Stage
  - [ ] `Step` - Works for both platforms (same concept)
  - [ ] `Runner` - Works for both platforms
  - [ ] `Platform` enum (GitHub, Harness)
  - [ ] Status enums (ExecutionStatus, RunnerStatus)

- [ ] Create `src/platform/` module for platform abstraction
  - [ ] `src/platform/mod.rs` - Platform trait definition
  - [ ] `src/platform/github.rs` - GitHub implementation
  - [ ] `src/platform/harness.rs` - Harness implementation

- [ ] Define `Platform` trait with methods returning unified types
  - [ ] `list_organizations()` → `Vec<Organization>`
  - [ ] `list_projects(org)` → `Vec<Project>`
  - [ ] `list_workflows(project)` → `Vec<Workflow>`
  - [ ] `list_executions(workflow)` → `Vec<Execution>`
  - [ ] `list_jobs(execution)` → `Vec<Job>`
  - [ ] `list_steps(job)` → `Vec<Step>`
  - [ ] `fetch_logs(step)` → `Vec<LogLine>`
  - [ ] `list_runners(scope)` → `Vec<Runner>`

- [ ] Implement mappers
  - [ ] GitHub types → Unified types
  - [ ] Harness types → Unified types
  - [ ] Status mapping (both directions)

- [ ] Update `App` state
  - [ ] Store `Vec<Box<dyn Platform>>` for multi-platform support
  - [ ] Merge and sort results from multiple platforms
  - [ ] Handle platform-specific features gracefully

### Success Criteria

- Unified types compile and cover all necessary fields
- Platform trait compiles and is usable
- Both GitHub and Harness implement Platform correctly
- Mappers correctly transform platform-specific types to unified types
- App can query multiple platforms and merge results
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

## Phase 4: Unified UI Integration

**Goal:** Integrate both platforms into unified TUI with platform badges

### Tasks

- [ ] Add platform badge rendering
  - [ ] Create `render_platform_badge()` function
  - [ ] Show [GH] or [HR] prefix on list items
  - [ ] Use color coding (blue for GitHub, orange for Harness)

- [ ] Update existing UI components for unified types
  - [ ] Update list rendering to use unified types
  - [ ] Update breadcrumb to show platform badge
  - [ ] Update status colors to work with unified statuses

- [ ] Enhance navigation for 7-level hierarchy
  - [ ] Organizations/Owners list
  - [ ] Projects/Repositories list
  - [ ] Workflows/Pipelines list
  - [ ] Executions/Runs list
  - [ ] Jobs/Stages list
  - [ ] Steps list (NEW level)
  - [ ] Logs viewer

- [ ] Update Runners tab
  - [ ] Fetch runners from all platforms
  - [ ] Merge and sort runner lists
  - [ ] Show platform badge for each runner
  - [ ] Show scope (repo/org/project)

- [ ] Update Workflows tab
  - [ ] Fetch from all platforms
  - [ ] Merge and sort at each navigation level
  - [ ] Maintain separate breadcrumbs per platform

- [ ] Add platform filtering (optional)
  - [ ] Press 'f' to toggle filter
  - [ ] Show only GitHub or only Harness
  - [ ] Show filter state in UI

- [ ] Update app event loop
  - [ ] Query all active platforms in parallel
  - [ ] Merge results from multiple platforms
  - [ ] Handle platform-specific errors gracefully

### Success Criteria

- Platform badges render correctly
- Can navigate unified hierarchy with both platforms
- Lists show mixed GitHub and Harness items
- Platform filtering works
- Colors and status indicators are consistent
- Navigation is intuitive across both platforms

### Testing

Manual testing checklist:
- [ ] Platform badges show on all list items
- [ ] Can navigate org → project → workflow → execution → job → step → logs
- [ ] Mixed lists (GitHub + Harness) display correctly
- [ ] Runner list shows runners from both platforms
- [ ] Status colors are correct for both platforms
- [ ] Can view logs for GitHub and Harness executions
- [ ] Platform filter toggles correctly
- [ ] Can switch platforms mid-navigation
- [ ] Breadcrumb shows current platform
- [ ] Error messages are clear and indicate platform

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
  - [ ] Support custom Harness base URLs
  - [ ] Support cache TTL customization

- [ ] Polish
  - [ ] Add help text for platform-specific features
  - [ ] Improve error messages with platform context
  - [ ] Add loading indicators for slow API calls
  - [ ] Add retry logic with exponential backoff
  - [ ] Handle rate limiting gracefully for both platforms
  - [ ] Add keyboard shortcuts for platform filtering

### Success Criteria

- Logs stream in real-time via WebSocket
- Caching reduces API calls significantly
- Config file works correctly
- Both GitHub and Harness work seamlessly in unified view
- Error handling is robust and platform-aware
- Performance is good even with slow networks
- Platform filtering is smooth and intuitive

### Testing

- [ ] WebSocket streaming works without drops
- [ ] Cache TTL works correctly for both platforms
- [ ] Cache invalidation works
- [ ] Config file is parsed correctly
- [ ] Platform filtering toggles correctly
- [ ] Rate limiting is handled gracefully for both platforms
- [ ] Long-running connections are stable
- [ ] Performance is acceptable with both platforms enabled

---

## Milestones

### M1: Basic Harness Client (Phases 1-2)
- Harness client can authenticate and make API calls
- All core API methods implemented (orgs, projects, pipelines, executions, stages, steps, logs, runners)
- Unit tests passing

### M2: Unified Types and Platform Abstraction (Phase 3)
- Unified types defined for both platforms
- Platform trait defined and implemented
- Both GitHub and Harness implement Platform trait
- Mappers correctly transform platform-specific types
- App can query multiple platforms

### M3: Unified UI Integration (Phase 4)
- Platform badges render correctly
- Both platforms visible in unified lists
- 7-level navigation works (including new Steps level)
- Can view runners, executions, jobs, steps, logs from both platforms
- Platform filtering works

### M4: Production Ready (Phase 5)
- WebSocket streaming works for Harness
- Caching improves performance for both platforms
- Configuration file support
- Error handling is robust
- Performance is good with both platforms
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
