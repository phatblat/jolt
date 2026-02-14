# Harness API Research

Research findings for integrating Harness CI/CD into jolt.
Verified against official Harness documentation (February 2026).

## API Generations

Harness has two API generations with different conventions:

| Aspect | NG API (stable) | V1 Beta API (newer) |
|--------|----------------|---------------------|
| Base URL | `https://app.harness.io/ng/api/` | `https://app.harness.io/v1/` |
| Scoping | Query parameters (`?accountIdentifier=...`) | Path segments (`/orgs/{org}/projects/{proj}/...`) |
| Account ID | `accountIdentifier` query param | `Harness-Account` header |
| Auth header | `x-api-key` | `x-api-key` + `Harness-Account` |
| Pagination | `pageIndex` / `pageSize` (default 50) | `page` / `limit` (default 30) |
| Response | Wrapped in `{"status":"SUCCESS","data":{...}}` | Direct content (no envelope) |
| Max page | 100 | 100 |

**Our approach:** Use NG API for orgs/projects (better list support), V1 for pipelines (cleaner paths).

## Authentication

### API Key Authentication

Harness uses Personal Access Tokens (PATs) for authentication.

**Token format:** `pat.xxxx.yyyy.zzzz` (PAT prefix is standard)

**Token generation:**
1. Login to Harness UI
2. Navigate to Profile (bottom-left) -> My API Keys
3. Click "+ API Key", enter a name
4. Click "+ Token", set expiration, click Generate
5. Copy immediately - **token is only shown once**

**Token types:**
- **Personal API Keys** - Account-level only, inherit user's permissions
- **Service Account Tokens** - Can be scoped to account/org/project

**Environment Variables (for jolt):**
- `HARNESS_API_KEY` - The PAT token (required)
- `HARNESS_ACCOUNT_ID` - Account identifier (required)
- `HARNESS_BASE_URL` - Base URL (optional, defaults to `https://app.harness.io/`)

**Account ID location:** Found in every Harness URL after `/account/`, e.g.:
```
https://app.harness.io/ng/#/account/6_vVHzo9Qeu9fXvj-AcQCb/settings/overview
                                     ^^^^^^^^^^^^^^^^^^^^^^^^
```
Also visible in Profile -> Account Settings.

**Required headers:**

NG API:
```http
Content-Type: application/json
x-api-key: pat.xxxx.yyyy.zzzz
```

V1 API (additional header):
```http
Content-Type: application/json
x-api-key: pat.xxxx.yyyy.zzzz
Harness-Account: YOUR_ACCOUNT_ID
```

## Organizational Structure

```
Account (Top-level)
  └── Organization
      └── Project
          ├── Pipelines
          │   └── Pipeline Execution
          │       └── Stage
          │           └── Step
          │               └── Logs
          ├── Services
          ├── Environments
          ├── Connectors
          └── Resources (including Runners)
```

**Full Execution Hierarchy:**
```
Pipeline → Execution → Stage → Step → Logs
```

Maps to GitHub Actions:
- **Pipeline** <-> Workflow
- **Execution** <-> Run
- **Stage** <-> Job
- **Step** <-> Step

**Key Identifiers:**
- `accountIdentifier` - Unique account ID (required for all NG API calls)
- `orgIdentifier` - Organization identifier within account
- `projectIdentifier` - Project identifier within organization

## Organizations API (NG)

**List Organizations:**
```
GET https://app.harness.io/ng/api/organizations
    ?accountIdentifier={accountId}
    &pageIndex=0
    &pageSize=50
    &searchTerm={optional}
```

**Response Structure:**
```json
{
  "status": "SUCCESS",
  "data": {
    "content": [
      {
        "organization": {
          "identifier": "default",
          "name": "Default Organization",
          "description": "Default organization",
          "tags": {}
        },
        "createdAt": 1234567890000,
        "lastModifiedAt": 1234567890000
      }
    ],
    "pageIndex": 0,
    "pageSize": 50,
    "totalPages": 1,
    "totalItems": 1
  },
  "metaData": {},
  "correlationId": "uuid-string"
}
```

**IMPORTANT:** Organization data is nested inside `content[].organization`, not flat.

**Get Organization:**
```
GET https://app.harness.io/ng/api/organizations/{orgId}
    ?accountIdentifier={accountId}
```

## Projects API (NG)

**List Projects:**
```
GET https://app.harness.io/ng/api/projects
    ?accountIdentifier={accountId}
    &orgIdentifier={orgId}
    &pageIndex=0
    &pageSize=50
    &searchTerm={optional}
    &moduleType={optional: CD, CI, CF, CV}
    &sortOrders={optional}
```

**Response Structure:**
```json
{
  "status": "SUCCESS",
  "data": {
    "content": [
      {
        "project": {
          "orgIdentifier": "default",
          "identifier": "project1",
          "name": "Project One",
          "description": "First project",
          "tags": {},
          "modules": ["CD", "CI"]
        },
        "createdAt": 1234567890000,
        "lastModifiedAt": 1234567890000,
        "isFavorite": false
      }
    ],
    "pageIndex": 0,
    "pageSize": 50,
    "totalPages": 1,
    "totalItems": 1
  },
  "metaData": {},
  "correlationId": "uuid-string"
}
```

**IMPORTANT:** Project data is nested inside `content[].project`, not flat.

**Get Project:**
```
GET https://app.harness.io/ng/api/projects/{projectId}
    ?accountIdentifier={accountId}
    &orgIdentifier={orgId}
```

## Pipelines API (V1 Beta)

**List Pipelines:**
```
GET https://app.harness.io/v1/orgs/{org}/projects/{project}/pipelines
    ?page=0
    &limit=30
    &searchTerm={optional}
    &sort={optional: name, createdAt}
    &order={optional: asc, desc}
```

Requires `Harness-Account` header.

**Response Structure (V1 - no envelope):**
```json
[
  {
    "identifier": "pipeline1",
    "name": "Pipeline One",
    "description": "First pipeline",
    "tags": {},
    "createdAt": 1234567890000,
    "lastModifiedAt": 1234567890000,
    "recentExecutions": []
  }
]
```

**Alternative NG API:**
```
GET https://app.harness.io/ng/api/pipelines
    ?accountIdentifier={accountId}
    &orgIdentifier={orgId}
    &projectIdentifier={projectId}
    &pageIndex=0
    &pageSize=50
```

**Get Pipeline (V1):**
```
GET https://app.harness.io/v1/orgs/{org}/projects/{project}/pipelines/{pipeline}
```

## Runners API (NG)

**List Runners:**
```
GET https://app.harness.io/ng/api/runner/list
    ?accountIdentifier={accountId}
    &orgIdentifier={optional}
    &projectIdentifier={optional}
    &status={optional: ACTIVE, INACTIVE, etc.}
    &pageIndex=0
    &pageSize=50
```

**Runner Status Values:**
- `ACTIVE` - Runner is online and available
- `INACTIVE` - Runner is offline
- `UNHEALTHY` - Runner has health issues
- `CONNECTED` - Runner connected to platform
- `DISCONNECTED` - Runner lost connection

**Response Structure:**
```json
{
  "status": "SUCCESS",
  "data": {
    "content": [
      {
        "identifier": "runner-id",
        "name": "Runner Name",
        "status": "ACTIVE",
        "lastHeartbeat": 1234567890000,
        "ipAddress": "x.x.x.x",
        "capacity": 10,
        "runningBuilds": 3
      }
    ],
    "totalItems": 100,
    "totalPages": 10,
    "pageIndex": 0,
    "pageSize": 50
  }
}
```

**Get Runner:**
```
GET https://app.harness.io/ng/api/runner/{runnerId}
    ?accountIdentifier={accountId}
```

## Pipeline Executions API (NG)

**List Pipeline Executions:**
```
POST https://app.harness.io/pipeline/api/pipelines/execution/v2/list
     ?accountIdentifier={accountId}
     &orgIdentifier={orgId}
     &projectIdentifier={projectId}
     &pipelineIdentifier={optional}
     &status={optional}
     &pageIndex=0
     &pageSize=50
```

**POST body (filter):**
```json
{
  "filterType": "PipelineExecution",
  "pipelineIdentifiers": ["pipeline-1"],
  "status": ["Running", "Queued"]
}
```

**Status Values:**
- `Running` - Currently executing
- `Success` - Completed successfully
- `Failed` - Execution failed
- `Aborted` - Manually stopped
- `Expired` - Timed out
- `Queued` - Waiting to start
- `Paused` - Waiting for approval/intervention

**Get Execution Details:**
```
GET https://app.harness.io/pipeline/api/pipelines/execution/{planExecutionId}
    ?accountIdentifier={accountId}
    &orgIdentifier={orgId}
    &projectIdentifier={projectId}
```

**Response Structure:**
```json
{
  "status": "SUCCESS",
  "data": {
    "pipelineExecutionSummary": {
      "planExecutionId": "exec-id",
      "pipelineIdentifier": "pipeline-id",
      "status": "Running",
      "startTs": 1234567890,
      "endTs": null,
      "stageExecutions": [
        {
          "stageIdentifier": "stage-id",
          "stageName": "Build",
          "status": "Running",
          "startTs": 1234567890,
          "stepExecutions": [
            {
              "stepIdentifier": "step-id",
              "stepName": "Run Tests",
              "status": "Success",
              "startTs": 1234567890,
              "endTs": 1234567900
            }
          ]
        }
      ]
    }
  }
}
```

## Logs API

### HTTP Log Endpoints

**Get Execution Logs:**
```
GET https://app.harness.io/log-service/blob/{accountId}/{key}
GET https://app.harness.io/pipeline/api/pipelines/execution/{planExecutionId}/logs
```

**Get Logs for Specific Step/Stage:**
```
GET https://app.harness.io/log-service/log-stream
    ?accountID={accountId}
    &key={logKey}
```

**Log Key Format:**
```
accountId/orgId/projectId/pipelineId/planExecutionId/stageId/stepId
```

**Log Response Structure:**
```json
{
  "logLines": [
    {
      "level": "INFO",
      "time": "2026-01-18T10:30:00Z",
      "message": "Log message content",
      "pos": 1
    }
  ],
  "more": true,
  "nextToken": "token-for-next-page"
}
```

### WebSocket Log Streaming

```
wss://app.harness.io/log-service/stream
```

- Real-time log streaming as logs are generated
- Subscribe to specific execution/step logs
- Requires authentication via query params or headers

## Common API Patterns

### Pagination (NG API)

```
?pageIndex=0&pageSize=50
```

Response includes:
```json
{
  "pageIndex": 0,
  "pageSize": 50,
  "totalPages": 10,
  "totalItems": 487
}
```

Response headers:
- `X-Total-Elements` - Total number of entries
- `X-Page-Number` - Current page number
- `X-Page-Size` - Number of entries per page

### Pagination (V1 API)

```
?page=0&limit=30
```

### Standard Response Envelope (NG API only)

```json
{
  "status": "SUCCESS",
  "data": { ... },
  "metaData": {},
  "correlationId": "uuid-string"
}
```

V1 API returns data directly without the envelope.

## Error Handling

**Common HTTP Status Codes:**
- `200` - Success
- `400` - Bad Request (invalid parameters)
- `401` - Unauthorized (invalid/missing API key)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `429` - Rate Limited
- `500` - Internal Server Error

**Error Response (NG):**
```json
{
  "status": "FAILURE",
  "code": "INVALID_REQUEST",
  "message": "Error description",
  "errors": [
    {
      "field": "fieldName",
      "message": "Field-specific error"
    }
  ],
  "correlationId": "uuid-string"
}
```

## Rate Limiting

| Scope | Limit |
|-------|-------|
| Per API key | 1,000 requests/minute |
| Per IP address | 5,000 requests/10 seconds (30,000/minute) |

Handle with exponential backoff when encountering 429 responses.

## Known Discrepancies from Initial Research

Issues found when comparing initial (speculative) research against official docs:

1. **Base URL**: Was `https://app.harness.io/gateway/`, should be `https://app.harness.io/`
2. **Org response nesting**: Orgs are at `content[].organization`, not `content[]` directly
3. **Project response nesting**: Projects are at `content[].project`, not `content[]` directly
4. **Pagination fields**: NG uses `totalItems`/`pageIndex`/`pageSize`, not `totalElements`/`page`/`size`
5. **Error status**: Real API returns `"FAILURE"`, not `"ERROR"`
6. **V1 pipeline API**: Has no response envelope, requires `Harness-Account` header
7. **Pipeline endpoint**: V1 path is `/v1/orgs/{org}/projects/{proj}/pipelines`, not `/pipeline/api/pipelines`

## References

- [Harness Developer Hub](https://developer.harness.io)
- [API Reference (OpenAPI)](https://apidocs.harness.io)
- [API Quickstart](https://developer.harness.io/docs/platform/automation/api/api-quickstart/)
- [Manage API Keys](https://developer.harness.io/docs/platform/automation/api/add-and-manage-api-keys/)
- [Platform Rate Limits](https://developer.harness.io/docs/platform/rate-limits/)
- [List Pipelines](https://apidocs.harness.io/pipelines/list-pipelines)
- [List Projects](https://apidocs.harness.io/project/getproject)
- [List Organizations](https://apidocs.harness.io/organization/getorganization)
