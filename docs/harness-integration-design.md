# Harness Integration Design

Design document for integrating Harness CI/CD support into jolt alongside GitHub Actions.

## Goals

1. Support multiple CI/CD platforms (GitHub Actions + Harness)
2. Provide unified TUI for viewing runners, executions, and logs
3. Allow users to switch between platforms or view both
4. Maintain similar navigation patterns across platforms
5. Share common UI components where possible

## Architecture Overview

### Multi-Platform Support

```
src/
├── main.rs
├── app.rs                    # Platform-agnostic app state
├── platform/
│   ├── mod.rs                # Platform trait definition
│   ├── github.rs             # GitHub Actions implementation
│   └── harness.rs            # Harness implementation
├── github/                   # GitHub-specific client
├── harness/                  # Harness-specific client
├── cache/                    # Shared caching layer
├── ui/                       # Platform-agnostic UI components
└── state/                    # Shared navigation state
```

### Platform Trait

Define a common interface for CI/CD platforms:

```rust
#[async_trait]
pub trait Platform {
    // Authentication
    async fn authenticate(&mut self) -> Result<()>;
    fn is_authenticated(&self) -> bool;

    // Organizational structure
    async fn list_organizations(&self) -> Result<Vec<Organization>>;
    async fn list_projects(&self, org: &str) -> Result<Vec<Project>>;

    // Runners
    async fn list_runners(&self, scope: Scope) -> Result<Vec<Runner>>;
    async fn get_runner_details(&self, id: &str) -> Result<RunnerDetails>;

    // Executions
    async fn list_executions(&self, scope: Scope, filter: ExecutionFilter) -> Result<Vec<Execution>>;
    async fn get_execution_details(&self, id: &str) -> Result<ExecutionDetails>;

    // Logs
    async fn fetch_logs(&self, execution_id: &str, step_id: Option<&str>) -> Result<Vec<LogLine>>;
    async fn stream_logs(&self, execution_id: &str, step_id: Option<&str>) -> Result<LogStream>;
}

pub enum Scope {
    Account(String),
    Organization { account: String, org: String },
    Project { account: String, org: String, project: String },
}
```

## Configuration

### Environment Variables

**GitHub:**
- `GITHUB_TOKEN` - GitHub personal access token

**Harness:**
- `HARNESS_API_KEY` - Harness API token
- `HARNESS_ACCOUNT_ID` - Harness account identifier
- `HARNESS_BASE_URL` - Base URL (optional, defaults to `https://app.harness.io/gateway/`)

### Configuration File

Optional config file at `~/.config/jolt/config.toml`:

```toml
[platforms.github]
enabled = true
token_env = "GITHUB_TOKEN"

[platforms.harness]
enabled = true
api_key_env = "HARNESS_API_KEY"
account_id_env = "HARNESS_ACCOUNT_ID"
base_url = "https://app.harness.io/gateway/"

[ui]
default_platform = "github"  # or "harness"
show_platform_tabs = true    # Show both platforms in separate tabs
```

### Platform Detection

On startup:
1. Read config file if present
2. Check environment variables for available platforms
3. Enable platforms that have valid credentials
4. If both available, use `default_platform` or prompt user
5. If only one available, use it automatically

## Harness Client Implementation

### Client Structure

```rust
pub struct HarnessClient {
    base_url: String,
    api_key: String,
    account_id: String,
    http_client: reqwest::Client,
}

impl HarnessClient {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("HARNESS_API_KEY")
            .map_err(|_| Error::MissingEnvVar("HARNESS_API_KEY"))?;
        let account_id = std::env::var("HARNESS_ACCOUNT_ID")
            .map_err(|_| Error::MissingEnvVar("HARNESS_ACCOUNT_ID"))?;
        let base_url = std::env::var("HARNESS_BASE_URL")
            .unwrap_or_else(|_| "https://app.harness.io/gateway/".to_string());

        Ok(Self::new(base_url, api_key, account_id))
    }

    pub fn new(base_url: String, api_key: String, account_id: String) -> Self {
        let http_client = reqwest::Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("x-api-key", api_key.parse().unwrap());
                headers
            })
            .build()
            .unwrap();

        Self { base_url, api_key, account_id, http_client }
    }
}
```

### API Methods

```rust
impl HarnessClient {
    // Organizations
    pub async fn list_organizations(&self) -> Result<Vec<Organization>> {
        let url = format!("{}/ng/api/organizations", self.base_url);
        let response = self.http_client
            .get(&url)
            .query(&[("accountIdentifier", &self.account_id)])
            .send()
            .await?;

        self.parse_response(response).await
    }

    // Projects
    pub async fn list_projects(&self, org_id: &str) -> Result<Vec<Project>> {
        let url = format!("{}/ng/api/projects", self.base_url);
        let response = self.http_client
            .get(&url)
            .query(&[
                ("accountIdentifier", &self.account_id),
                ("orgIdentifier", org_id),
            ])
            .send()
            .await?;

        self.parse_response(response).await
    }

    // Runners
    pub async fn list_runners(&self, org_id: Option<&str>, project_id: Option<&str>) -> Result<Vec<Runner>> {
        let url = format!("{}/ng/api/runner/list", self.base_url);
        let mut params = vec![("accountIdentifier", self.account_id.as_str())];
        if let Some(org) = org_id {
            params.push(("orgIdentifier", org));
        }
        if let Some(project) = project_id {
            params.push(("projectIdentifier", project));
        }

        let response = self.http_client
            .get(&url)
            .query(&params)
            .send()
            .await?;

        self.parse_response(response).await
    }

    // Executions
    pub async fn list_executions(&self, org_id: &str, project_id: &str, filter: ExecutionFilter) -> Result<Vec<Execution>> {
        let url = format!("{}/pipeline/api/pipelines/execution/v2/list", self.base_url);

        let body = json!({
            "filterType": "PipelineExecution",
            "status": filter.statuses,
            "pipelineIdentifiers": filter.pipeline_ids,
        });

        let response = self.http_client
            .post(&url)
            .query(&[
                ("accountIdentifier", &self.account_id),
                ("orgIdentifier", org_id),
                ("projectIdentifier", project_id),
            ])
            .json(&body)
            .send()
            .await?;

        self.parse_response(response).await
    }

    // Logs
    pub async fn fetch_logs(&self, execution_id: &str) -> Result<Vec<LogLine>> {
        let url = format!("{}/pipeline/api/pipelines/execution/{}/logs", self.base_url, execution_id);
        let response = self.http_client
            .get(&url)
            .send()
            .await?;

        self.parse_response(response).await
    }
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("API returned error: {code} - {message}")]
    ApiError { code: String, message: String },

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid response format: {0}")]
    ParseError(String),
}
```

## Caching Strategy

### Cache Organization

```
~/.cache/jolt/
├── github/
│   ├── workflows/
│   ├── runs/
│   └── logs/
└── harness/
    ├── organizations/      # Cache org list (1 hour TTL)
    ├── projects/           # Cache project list (1 hour TTL)
    ├── runners/            # Cache runner list (30s TTL)
    ├── executions/         # Cache execution list (10s TTL)
    └── logs/               # Immutable logs (no TTL)
```

### Cache TTL Strategy

| Resource | TTL | Rationale |
|----------|-----|-----------|
| Organizations | 1 hour | Rarely change |
| Projects | 1 hour | Rarely change |
| Runners | 30 seconds | Status changes frequently |
| Executions | 10 seconds | Status changes very frequently |
| Logs | Infinite | Immutable once written |

### Polling vs. Caching

- **Runners**: Poll every 30s when tab is active
- **Executions**: Poll every 10s when tab is active
- **Logs**: Use WebSocket streaming when viewing, cache completed logs

## Navigation Structure

### Breadcrumb Navigation

**Harness:**
```
Account → Organization → Project → Runners/Executions
```

**GitHub:**
```
Repository → Workflows → Runs
```

### Tab Structure

Two approaches (configurable):

**Option 1: Platform Tabs**
```
[GitHub] [Harness] [Console]
```
Within each platform tab, show runners/workflows as sub-views.

**Option 2: Unified View**
```
[Runners] [Workflows] [Console]
```
Show both platforms within each tab, with platform filter/selector.

Initial implementation: Option 1 (simpler)

## UI Components

### Platform-Specific Adaptations

**Runner Status Colors:**
- GitHub: `idle` (green), `offline` (gray), `busy` (yellow)
- Harness: `ACTIVE` (green), `INACTIVE` (gray), `UNHEALTHY` (red), `CONNECTED` (green), `DISCONNECTED` (gray)

Map Harness statuses to GitHub equivalents for consistent coloring:
- `ACTIVE`/`CONNECTED` → green (idle)
- `INACTIVE`/`DISCONNECTED` → gray (offline)
- `UNHEALTHY` → red (custom for Harness)

**Execution Status Colors:**
- GitHub: `success` (green), `failure` (red), `in_progress` (yellow), `queued` (blue)
- Harness: `Success` (green), `Failed` (red), `Running` (yellow), `Queued` (blue), `Paused` (magenta), `Aborted` (gray)

### Shared Components

Reuse existing UI components:
- `ui/tabs.rs` - Tab bar (add platform badge)
- `ui/breadcrumb.rs` - Breadcrumb navigation
- `ui/list.rs` - Generic list widget
- `ui/log_viewer.rs` - Log display
- `ui/console.rs` - Console messages

## Implementation Phases

### Phase 1: Harness Client Foundation
- Create `src/harness/` module
- Implement `HarnessClient` with authentication
- Add environment variable loading
- Basic error handling

### Phase 2: Harness API Integration
- Implement organization/project listing
- Implement runner listing
- Implement execution listing
- Implement log fetching (HTTP only)

### Phase 3: Platform Abstraction
- Define `Platform` trait
- Refactor GitHub client to implement `Platform`
- Implement `Platform` for Harness
- Update app state to work with `Platform` trait

### Phase 4: UI Integration
- Add Harness tab to tab bar
- Implement breadcrumb navigation for Harness
- Display runners and executions
- Show logs

### Phase 5: Advanced Features
- WebSocket log streaming for Harness
- Improved caching with background refresh
- Multi-platform support (view both simultaneously)
- Configuration file support

## Dependencies to Add

```toml
[dependencies]
# Existing
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
chrono = "0.4"
directories = "6.0"

# New for Harness
async-trait = "0.1"           # For Platform trait
tokio-tungstenite = "0.24"    # WebSocket support for log streaming
toml = "0.8"                  # Config file parsing
```

## Testing Strategy

### Unit Tests
- Test Harness API response parsing
- Test error handling
- Test cache TTL logic
- Test platform trait implementations

### Integration Tests
- Mock Harness API server
- Test full request/response cycle
- Test authentication flows
- Test WebSocket log streaming

### Manual Testing
- Test against real Harness account
- Verify API endpoints match documentation
- Test error scenarios (invalid token, rate limits)
- Test with different org/project structures

## Security Considerations

1. **Token Storage**: Never log or cache API tokens
2. **Token Validation**: Validate token format before use
3. **HTTPS Only**: Enforce HTTPS for all API requests
4. **Rate Limiting**: Implement exponential backoff
5. **Error Messages**: Don't expose tokens in error messages
6. **Cache Permissions**: Restrict cache directory permissions (0700)

## Open Questions

1. How to handle Harness self-managed installations with different base URLs?
2. Should we support both GitHub and Harness simultaneously in one view?
3. How to handle long-running WebSocket connections on slow networks?
4. Should we cache organization/project structure or always fetch fresh?
5. How to handle different API versions if Harness updates their API?

## Future Enhancements

- Support for additional platforms (GitLab CI, CircleCI, etc.)
- Graphical view of pipeline stages
- Ability to trigger new executions
- Ability to cancel running executions
- Export logs to file
- Search across executions
- Custom filters and saved views
