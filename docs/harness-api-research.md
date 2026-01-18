# Harness API Research

Research findings for integrating Harness CI/CD into jolt.

## Authentication

### API Key Authentication

Harness uses API keys (Personal Access Tokens/PAT) for authentication.

**Environment Variables:**
- `HARNESS_API_KEY` - The API token itself
- `HARNESS_ACCOUNT_ID` - Account identifier (required for all API calls)
- `HARNESS_BASE_URL` - Base URL (optional, defaults to SaaS)
  - SaaS: `https://app.harness.io/gateway/`
  - Self-managed: `https://<your-domain>/gateway/`

**Authentication Methods:**
- Header-based: `x-api-key: <your-api-token>`
- Bearer Token: `Authorization: Bearer <your-api-token>`

**API Key Generation:**
- Generated from Harness UI: Profile → Personal Access Tokens
- Can be scoped to specific resources and permissions
- Support for both Account-level and Project-level tokens

## Organizational Structure

Harness uses a three-level hierarchy:

```
Account (Top-level)
  └── Organization
      └── Project
          ├── Pipelines
          ├── Services
          ├── Environments
          ├── Connectors
          └── Resources (including Runners)
```

**Key Identifiers:**
- `accountIdentifier` - Unique account ID (required for all API calls)
- `orgIdentifier` - Organization identifier within account
- `projectIdentifier` - Project identifier within organization

**Scope Levels:**
- **Account Level**: Resources available across all orgs/projects
- **Organization Level**: Shared within org across projects
- **Project Level**: Isolated to specific project

**API Endpoints:**

List Organizations:
```
GET /ng/api/organizations?accountIdentifier={accountId}
```

List Projects:
```
GET /ng/api/projects?accountIdentifier={accountId}&orgIdentifier={orgId}
```

## Runners

**List Runners:**
```
GET /ng/api/runner/list
```

**Query Parameters:**
- `accountIdentifier` - Account ID (required)
- `orgIdentifier` - Organization ID (optional)
- `projectIdentifier` - Project ID (optional)
- `status` - Filter by status (ACTIVE, INACTIVE, etc.)
- `page` - Page number
- `size` - Page size

**Runner Status Values:**
- `ACTIVE` - Runner is online and available
- `INACTIVE` - Runner is offline
- `UNHEALTHY` - Runner has health issues
- `CONNECTED` - Runner connected to platform
- `DISCONNECTED` - Runner lost connection

**Response Structure:**
```json
{
  "data": {
    "content": [
      {
        "identifier": "runner-id",
        "name": "Runner Name",
        "status": "ACTIVE",
        "lastHeartbeat": "timestamp",
        "ipAddress": "x.x.x.x",
        "capacity": 10,
        "runningBuilds": 3
      }
    ],
    "totalElements": 100,
    "totalPages": 10
  }
}
```

**Get Runner Details:**
```
GET /ng/api/runner/{runnerId}
```

## Pipeline Executions

**List Pipeline Executions:**
```
POST /pipeline/api/pipelines/execution/v2/list
```

**Alternative (Summary):**
```
GET /pipeline/api/pipelines/execution/summary
```

**Query Parameters:**
- `accountIdentifier` - Account ID (required)
- `orgIdentifier` - Organization ID (required)
- `projectIdentifier` - Project ID (required)
- `pipelineIdentifier` - Specific pipeline (optional)
- `status` - Filter by status (optional)
- `page`, `size` - Pagination

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
GET /pipeline/api/pipelines/execution/{planExecutionId}
```

**Response Structure:**
```json
{
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
          "status": "Running",
          "startTs": 1234567890
        }
      ]
    }
  }
}
```

## Logs

### HTTP Log Endpoints

**Get Execution Logs:**
```
GET /log-service/blob/{accountId}/{key}
GET /pipeline/api/pipelines/execution/{planExecutionId}/logs
```

**Get Logs for Specific Step/Stage:**
```
GET /log-service/log-stream
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

Harness supports WebSocket connections for real-time log streaming.

**WebSocket Endpoint:**
```
wss://app.harness.io/log-service/stream
```

**Features:**
- Real-time log streaming as logs are generated
- Subscribe to specific execution/step logs
- Requires authentication via query params or headers
- Receives log events as they're generated

## Common API Patterns

### Pagination

```json
{
  "page": 0,
  "size": 50,
  "sort": ["createdAt,DESC"]
}
```

### Filtering

```json
{
  "filterType": "PipelineExecution",
  "pipelineIdentifiers": ["pipeline-1"],
  "status": ["Running", "Queued"]
}
```

### Standard Response Envelope

```json
{
  "status": "SUCCESS",
  "data": { /* actual data */ },
  "metaData": null,
  "correlationId": "correlation-id"
}
```

## Error Handling

**Common HTTP Status Codes:**
- `200` - Success
- `400` - Bad Request (invalid parameters)
- `401` - Unauthorized (invalid/missing API key)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `429` - Rate Limited
- `500` - Internal Server Error

**Error Response:**
```json
{
  "status": "ERROR",
  "code": "INVALID_REQUEST",
  "message": "Error description",
  "correlationId": "correlation-id"
}
```

## Rate Limiting

- Harness enforces rate limits on API requests
- Handle with exponential backoff when encountering 429 responses
- Cache responses aggressively to reduce API calls

## Verification Needed

The information above is based on general Harness API knowledge as of January 2025. The following should be verified against live API documentation:

1. Exact endpoint paths at https://apidocs.harness.io
2. Current response structures and field names
3. WebSocket connection details and authentication
4. Rate limiting policies and best practices
5. Latest API version and deprecation notices

## References

- Harness Developer Hub: https://developer.harness.io
- API Reference: https://apidocs.harness.io
- API Quickstart: https://developer.harness.io/docs/platform/automation/api/api-quickstart
