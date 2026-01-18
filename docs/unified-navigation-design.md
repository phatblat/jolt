# Unified Navigation Design

Analysis of GitHub Actions vs Harness models and strategy for unified navigation.

## Model Comparison

### GitHub Actions Hierarchy

```
(Implicit Account)
  └── Owner (user/org)
      └── Repository
          └── Workflow
              └── Workflow Run
                  └── Job
                      └── Step
                          └── Logs
```

**Current Navigation:**
- Breadcrumb: `owner > repo-name > workflow > run #123`
- Navigation levels shown in UI: Owners → Repositories → Workflows → Runs → Jobs → Logs

**Runners:**
- Scoped to: Repository, Organization, or Enterprise
- Current UI: Shows repos with runners, then runners for that repo

### Harness Hierarchy

```
Account (explicit)
  └── Organization
      └── Project
          └── Pipeline
              └── Pipeline Execution
                  └── Stage
                      └── Step
                          └── Logs
```

**Natural Navigation:**
- Breadcrumb: `account > org > project > pipeline > execution #123`
- Navigation levels: Organizations → Projects → Pipelines → Executions → Stages → Steps → Logs

**Runners:**
- Scoped to: Account, Organization, or Project
- Can inherit from parent scopes

## Key Observations

### Similarities

1. **Identical Hierarchy Depth**: Both are exactly 7 levels deep (including logs)
2. **Perfect 1:1 Conceptual Mapping**:
   - *(Implicit Account)* (GitHub) ↔ Account (Harness)
   - Owner (GitHub) ↔ Organization (Harness)
   - Repository (GitHub) ↔ Project (Harness)
   - Workflow (GitHub) ↔ Pipeline (Harness)
   - Workflow Run (GitHub) ↔ Pipeline Execution (Harness)
   - Job (GitHub) ↔ Stage (Harness)
   - Step (GitHub) ↔ Step (Harness)
3. **Runner Scoping**: Both support runners at multiple scope levels
4. **Status Model**: Both have similar execution status (running, success, failed, queued)
5. **Execution Subdivision**: Both use Jobs/Stages that contain Steps

### Differences

1. **Top Level Visibility**:
   - GitHub: User/Org is the top level (account is implicit, tied to authentication)
   - Harness: Account is explicit and must be specified in all API calls

2. **Container Semantics**:
   - GitHub: Repository is a code repository with CI/CD workflows
   - Harness: Project is a CI/CD container (may span multiple repos or have no repo)

3. **Runner Inheritance**:
   - GitHub: Explicit association (repo → org → enterprise)
   - Harness: Can list runners from parent scopes (project inherits org/account runners)

4. **Terminology**:
   - GitHub uses "Job" for the stage level
   - Harness uses "Stage" for the same concept
   - Both use "Step" for the subdivision within Jobs/Stages

## Unified Navigation Strategy

### Option 1: Parallel Hierarchies (Current Design)

Keep separate tabs per platform, as proposed in `harness-integration-design.md`.

**Pros:**
- Clear separation of concerns
- No model impedance mismatch
- Platform-specific features don't interfere

**Cons:**
- Duplicated UI code
- Cannot view both platforms side-by-side
- More tabs/complexity

### Option 2: Unified Hierarchy with Platform Indicator

Treat Organization (Harness) and Owner (GitHub) as equivalent top-level entities, then unify downward.

**Navigation Structure:**
```
Organizations/Owners (mixed list)
  └── Projects/Repositories (mixed list)
      └── Pipelines/Workflows (mixed list)
          └── Executions/Runs (mixed list)
              └── Stages/Jobs (mixed list)
                  └── Steps (mixed list)
                      └── Logs
```

This provides a perfect 1:1 mapping since both platforms have identical hierarchy depth and structure.

**Implementation:**
```rust
pub struct Organization {
    pub name: String,
    pub platform: Platform,  // GitHub or Harness
    pub identifier: String,  // login (GitHub) or org ID (Harness)
}

pub struct Project {
    pub name: String,
    pub platform: Platform,
    pub org: String,
    pub full_path: String,  // "owner/repo" or "org/project"
}

pub struct Workflow {
    pub name: String,
    pub platform: Platform,
    pub project_path: String,
    pub identifier: String,
}
```

**Breadcrumb Examples:**
- GitHub: `phatblat > jolt > CI > run #123`
- Harness: `my-org > frontend-proj > build-pipeline > exec #456`

**Visual Platform Indicator:**
- Prefix each item with platform badge: `[GH] jolt` or `[HR] frontend-proj`
- Use color coding (blue for GitHub, orange for Harness)
- Show platform icon/glyph

**Pros:**
- Single navigation flow
- Can see all resources from both platforms together
- Shared UI code
- Simpler mental model for users with both platforms

**Cons:**
- Need to normalize differences (especially Account vs Owner)
- Platform-specific fields may not align perfectly
- Requires careful abstraction layer

### Option 3: Workspace-Centric Unified View

Flatten to a "workspace" concept where the primary view is Projects/Repositories, hiding the organization level by default.

**Navigation Structure:**
```
Workspaces (Projects/Repos with full path shown)
  ├── phatblat/jolt [GH]
  ├── my-org/frontend-proj [HR]
  ├── acme-corp/api-service [HR]
  └── google/guava [GH]
      └── Workflows/Pipelines
          └── Runs/Executions
              └── Jobs/Stages
                  └── Logs
```

**Breadcrumb:**
- Show full path in breadcrumb: `phatblat/jolt > CI > run #123`
- Organization/Owner is embedded in the workspace name

**Pros:**
- Simplest unified view
- Matches how developers think ("I'm working on project X")
- Organization level is preserved in the path but not a separate navigation step

**Cons:**
- Loses ability to browse at org level first
- Long paths for Harness (might need to show `org/project` and hide account)

## Recommended Approach: Option 2 (Unified Hierarchy)

**Rationale:**
1. Preserves full navigational flexibility
2. Allows browsing by organization first (common workflow)
3. Maintains clear data model mapping
4. Platform badges provide clear visual distinction
5. Can be implemented incrementally

### Handling the Account Level (Harness)

**Problem:** Harness has Account above Organization, GitHub doesn't.

**Solution:** Hide Account level in navigation, treat it as implicit:
- Account is stored in config/env (`HARNESS_ACCOUNT_ID`)
- All Harness orgs shown are for that account
- If user needs multi-account, they switch accounts (future feature)

This mirrors GitHub's model where the authenticated user's account is implicit.

### Unified Data Model

```rust
// Platform-agnostic types
pub enum Platform {
    GitHub,
    Harness,
}

pub struct Organization {
    pub name: String,
    pub display_name: String,
    pub platform: Platform,
    pub id: String,
}

pub struct Project {
    pub name: String,
    pub display_name: String,  // "owner/repo" or "org/project"
    pub platform: Platform,
    pub org_id: String,
    pub id: String,
}

pub struct Workflow {
    pub name: String,
    pub platform: Platform,
    pub project_id: String,
    pub id: String,
    pub state: WorkflowState,
}

pub struct Execution {
    pub id: String,
    pub number: i64,
    pub platform: Platform,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub duration: Option<Duration>,
}

pub enum ExecutionStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
    Paused,  // Harness-only
}

pub struct Job {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
}

pub struct Step {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub job_id: String,
    pub status: ExecutionStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
}

pub struct Runner {
    pub name: String,
    pub platform: Platform,
    pub status: RunnerStatus,
    pub scope: RunnerScope,
    pub current_job: Option<String>,
}

pub enum RunnerScope {
    Repository { org: String, repo: String },
    Organization { org: String },
    Project { org: String, project: String },
}

pub enum RunnerStatus {
    Online,
    Offline,
    Busy,
    Unhealthy,  // Harness-specific, but useful to show
}
```

### Status Mapping

**Execution Status:**
| GitHub | Harness | Unified |
|--------|---------|---------|
| `queued` | `Queued` | `Queued` |
| `in_progress` | `Running` | `Running` |
| `completed` (success) | `Success` | `Success` |
| `completed` (failure) | `Failed` | `Failed` |
| `completed` (cancelled) | `Aborted` | `Cancelled` |
| - | `Paused` | `Paused` (Harness-only) |
| - | `Expired` | `Failed` (map to failed) |

**Runner Status:**
| GitHub | Harness | Unified |
|--------|---------|---------|
| `online` (idle) | `ACTIVE` | `Online` |
| `offline` | `INACTIVE` / `DISCONNECTED` | `Offline` |
| `online` (busy) | - | `Busy` |
| - | `UNHEALTHY` | `Unhealthy` |

### UI Representation

**Organization List:**
```
┌─────────────────────────────────────────────┐
│ Organizations                               │
├─────────────────────────────────────────────┤
│ [GH] phatblat (User)                        │
│ [GH] getditto (Organization)                │
│ [HR] acme-corp                              │
│ [HR] customer-success                       │
└─────────────────────────────────────────────┘
```

**Project List:**
```
┌─────────────────────────────────────────────┐
│ phatblat > Projects                         │
├─────────────────────────────────────────────┤
│ [GH] dotfiles         Updated 2h ago        │
│ [GH] jolt             Updated 5h ago        │
│ [HR] api-gateway      Updated 1d ago        │
└─────────────────────────────────────────────┘
```

**Breadcrumb:**
```
┌─────────────────────────────────────────────┐
│ [GH] phatblat > jolt > CI > run #123        │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ [HR] acme-corp > api-gateway > build > #456 │
└─────────────────────────────────────────────┘
```

### Runners Tab

Show all runners from both platforms in a unified list:

```
┌─────────────────────────────────────────────────────────────┐
│ Runners                                                     │
├─────────────────────────────────────────────────────────────┤
│ [GH] runner-1           Online    phatblat/jolt            │
│ [GH] runner-2           Busy      getditto/common (Job #3) │
│ [HR] prod-runner-1      Online    acme-corp (Org level)    │
│ [HR] prod-runner-2      Unhealthy acme-corp/api-gateway    │
│ [GH] macos-self-hosted  Offline   phatblat                 │
└─────────────────────────────────────────────────────────────┘
```

Show scope in the third column to indicate where the runner belongs.

## Implementation Impact

### Changes to Current Design

1. **No separate platform tabs**: Instead of `[GitHub]` and `[Harness]` tabs, we have:
   - `[Runners]` - Unified runner view
   - `[Workflows]` - Unified workflow/pipeline view
   - `[Console]` - Unified console

2. **Platform abstraction layer**: Create `Platform` trait as planned, but use it for unified views rather than separate views

3. **Visual platform indicators**: Add badge/icon rendering in UI components

4. **Scope display**: Show runner and resource scope clearly (especially for Harness multi-level scoping)

### Benefits

1. **Simpler UX**: One navigation flow regardless of platform
2. **Better comparison**: See all resources side-by-side
3. **Less duplication**: Shared UI code for both platforms
4. **Scalable**: Easy to add more platforms (GitLab, CircleCI, etc.)

### Challenges

1. **API Differences**: Need to normalize different response structures
2. **Platform-specific Features**: Some features may not have equivalents (e.g., Harness `Paused` status)
3. **Performance**: Fetching from multiple platforms in parallel
4. **Error Handling**: Platform-specific errors in a unified view

## Migration Path

1. **Phase 1**: Keep current GitHub implementation, implement Harness with Platform trait
2. **Phase 2**: Create unified types and mappers from platform-specific types
3. **Phase 3**: Update UI to use unified types, add platform badges
4. **Phase 4**: Remove platform-specific tabs, consolidate into unified views
5. **Phase 5**: Add multi-platform filtering and advanced features

## Open Questions

1. **Filtering**: Should users be able to filter by platform in the unified view?
2. **Configuration**: Should users be able to disable platforms in the config?
3. **Default view**: When app starts, show GitHub first, Harness first, or both?
4. **Sorting**: When mixing platforms, how to sort? (alphabetical, by update time, by platform?)
5. **Runner scope**: How to best represent Harness's inherited runners from parent scopes?

## Conclusion

**The models are virtually identical!** Both platforms use the exact same 7-level hierarchy.

**Perfect 1:1 Mapping:**
- Account (implicit/explicit)
- Owner/Organization
- Repository/Project
- Workflow/Pipeline
- Run/Execution
- Job/Stage
- Step/Step

Recommendation:
- **Use Option 2 (Unified Hierarchy)** - This is the clear winner
- Hide Harness Account level (treat as implicit like GitHub's account)
- Use platform badges for visual distinction ([GH] vs [HR])
- Map platform-specific statuses to common statuses
- Show scope explicitly for runners and resources
- Unify Job (GitHub) and Stage (Harness) under a common `Job` type
- Both use Steps as subdivisions within Jobs/Stages

This provides the best user experience with minimal duplication while preserving the full capabilities of both platforms. The identical hierarchies make unified navigation straightforward to implement.
