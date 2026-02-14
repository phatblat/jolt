// Harness API response types.
// Defines structures for deserializing Harness API responses.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Standard Harness API response envelope.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: Option<T>,
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
}

/// Error response from Harness API.
/// Status is typically "FAILURE" for error responses.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiError {
    pub status: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
}

/// Field-level error detail in API error responses.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldError {
    pub field: Option<String>,
    pub message: Option<String>,
}

/// Wrapper for organization in list response (content[].organization).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrganizationResponse {
    pub organization: Organization,
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    #[serde(rename = "lastModifiedAt")]
    pub last_modified_at: Option<i64>,
}

/// Organization in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Organization {
    pub identifier: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Wrapper for project in list response (content[].project).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectResponse {
    pub project: Project,
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    #[serde(rename = "lastModifiedAt")]
    pub last_modified_at: Option<i64>,
}

/// Project in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Project {
    pub identifier: String,
    pub name: String,
    #[serde(rename = "orgIdentifier")]
    pub org_identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<String>>,
}

/// Runner status in Harness.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RunnerStatus {
    Active,
    Inactive,
    Unhealthy,
    Connected,
    Disconnected,
}

/// Pipeline execution status in Harness.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Running,
    Success,
    Failed,
    Aborted,
    Expired,
    Queued,
    Paused,
}

/// Paginated response wrapper (NG API).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageResponse<T> {
    pub content: Vec<T>,
    #[serde(rename = "totalItems")]
    pub total_items: Option<i64>,
    #[serde(rename = "totalPages")]
    pub total_pages: Option<i64>,
    #[serde(rename = "pageIndex")]
    pub page_index: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
}

/// Runner in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Runner {
    pub identifier: String,
    pub name: String,
    pub status: RunnerStatus,
    #[serde(rename = "lastHeartbeat")]
    pub last_heartbeat: Option<i64>,
    #[serde(rename = "ipAddress")]
    pub ip_address: Option<String>,
    pub capacity: Option<i32>,
    #[serde(rename = "runningBuilds")]
    pub running_builds: Option<i32>,
}

/// Pipeline in Harness.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pipeline {
    pub identifier: String,
    pub name: String,
    #[serde(rename = "projectIdentifier")]
    pub project_identifier: String,
    #[serde(rename = "orgIdentifier")]
    pub org_identifier: String,
}

/// Pipeline execution summary.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Execution {
    #[serde(rename = "planExecutionId")]
    pub plan_execution_id: String,
    #[serde(rename = "pipelineIdentifier")]
    pub pipeline_identifier: String,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
    pub name: Option<String>,
}

/// Pipeline execution details with stages and steps.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionDetails {
    #[serde(rename = "planExecutionId")]
    pub plan_execution_id: String,
    #[serde(rename = "pipelineIdentifier")]
    pub pipeline_identifier: String,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
    #[serde(rename = "stageExecutions")]
    pub stage_executions: Vec<StageExecution>,
}

/// Stage execution within a pipeline execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StageExecution {
    #[serde(rename = "stageIdentifier")]
    pub stage_identifier: String,
    #[serde(rename = "stageName")]
    pub stage_name: Option<String>,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
    #[serde(rename = "stepExecutions")]
    pub step_executions: Option<Vec<StepExecution>>,
}

/// Step execution within a stage.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepExecution {
    #[serde(rename = "stepIdentifier")]
    pub step_identifier: String,
    #[serde(rename = "stepName")]
    pub step_name: Option<String>,
    pub status: ExecutionStatus,
    #[serde(rename = "startTs")]
    pub start_ts: i64,
    #[serde(rename = "endTs")]
    pub end_ts: Option<i64>,
}

/// Log line from execution.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogLine {
    pub level: Option<String>,
    pub time: Option<String>,
    pub message: String,
    pub pos: Option<i64>,
}

/// Log response with pagination.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogResponse {
    #[serde(rename = "logLines")]
    pub log_lines: Vec<LogLine>,
    pub more: Option<bool>,
    #[serde(rename = "nextToken")]
    pub next_token: Option<String>,
}

/// Execution filter for listing executions.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExecutionFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<ExecutionStatus>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "pipelineIdentifiers"
    )]
    pub pipeline_ids: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_organization_list_response() {
        let json = r#"{
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
            "correlationId": "abc-123"
        }"#;

        let response: ApiResponse<PageResponse<OrganizationResponse>> =
            serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "SUCCESS");

        let page = response.data.unwrap();
        assert_eq!(page.total_items, Some(1));
        assert_eq!(page.page_index, Some(0));
        assert_eq!(page.content.len(), 1);

        let org = &page.content[0].organization;
        assert_eq!(org.identifier, "default");
        assert_eq!(org.name, "Default Organization");
        assert_eq!(org.description, Some("Default organization".to_string()));
    }

    #[test]
    fn test_deserialize_project_list_response() {
        let json = r#"{
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
                        "lastModifiedAt": 1234567890000
                    }
                ],
                "pageIndex": 0,
                "pageSize": 50,
                "totalPages": 1,
                "totalItems": 1
            },
            "metaData": {},
            "correlationId": "abc-123"
        }"#;

        let response: ApiResponse<PageResponse<ProjectResponse>> =
            serde_json::from_str(json).unwrap();
        let page = response.data.unwrap();
        assert_eq!(page.content.len(), 1);

        let project = &page.content[0].project;
        assert_eq!(project.identifier, "project1");
        assert_eq!(project.org_identifier, "default");
        assert_eq!(
            project.modules,
            Some(vec!["CD".to_string(), "CI".to_string()])
        );
    }

    #[test]
    fn test_deserialize_runner_list_response() {
        let json = r#"{
            "status": "SUCCESS",
            "data": {
                "content": [
                    {
                        "identifier": "runner-1",
                        "name": "Build Runner",
                        "status": "ACTIVE",
                        "lastHeartbeat": 1234567890000,
                        "ipAddress": "10.0.0.1",
                        "capacity": 10,
                        "runningBuilds": 3
                    }
                ],
                "totalItems": 1,
                "totalPages": 1,
                "pageIndex": 0,
                "pageSize": 50
            }
        }"#;

        let response: ApiResponse<PageResponse<Runner>> = serde_json::from_str(json).unwrap();
        let page = response.data.unwrap();

        let runner = &page.content[0];
        assert_eq!(runner.identifier, "runner-1");
        assert_eq!(runner.status, RunnerStatus::Active);
        assert_eq!(runner.capacity, Some(10));
        assert_eq!(runner.running_builds, Some(3));
    }

    #[test]
    fn test_deserialize_error_response() {
        let json = r#"{
            "status": "FAILURE",
            "code": "INVALID_REQUEST",
            "message": "Invalid account identifier",
            "errors": [
                {
                    "field": "accountIdentifier",
                    "message": "must not be empty"
                }
            ],
            "correlationId": "abc-123"
        }"#;

        let error: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.status, "FAILURE");
        assert_eq!(error.code, "INVALID_REQUEST");
        assert!(error.errors.is_some());
        assert_eq!(error.errors.unwrap().len(), 1);
    }

    #[test]
    fn test_deserialize_execution_status_variants() {
        for (json_val, expected) in [
            ("\"Running\"", ExecutionStatus::Running),
            ("\"Success\"", ExecutionStatus::Success),
            ("\"Failed\"", ExecutionStatus::Failed),
            ("\"Aborted\"", ExecutionStatus::Aborted),
            ("\"Expired\"", ExecutionStatus::Expired),
            ("\"Queued\"", ExecutionStatus::Queued),
            ("\"Paused\"", ExecutionStatus::Paused),
        ] {
            let status: ExecutionStatus = serde_json::from_str(json_val).unwrap();
            assert_eq!(status, expected);
        }
    }

    #[test]
    fn test_deserialize_runner_status_variants() {
        for (json_val, expected) in [
            ("\"ACTIVE\"", RunnerStatus::Active),
            ("\"INACTIVE\"", RunnerStatus::Inactive),
            ("\"UNHEALTHY\"", RunnerStatus::Unhealthy),
            ("\"CONNECTED\"", RunnerStatus::Connected),
            ("\"DISCONNECTED\"", RunnerStatus::Disconnected),
        ] {
            let status: RunnerStatus = serde_json::from_str(json_val).unwrap();
            assert_eq!(status, expected);
        }
    }

    #[test]
    fn test_deserialize_log_response() {
        let json = r#"{
            "logLines": [
                {
                    "level": "INFO",
                    "time": "2026-01-18T10:30:00Z",
                    "message": "Build started",
                    "pos": 1
                },
                {
                    "level": "ERROR",
                    "time": "2026-01-18T10:31:00Z",
                    "message": "Test failed",
                    "pos": 2
                }
            ],
            "more": true,
            "nextToken": "page-2-token"
        }"#;

        let log_response: LogResponse = serde_json::from_str(json).unwrap();
        assert_eq!(log_response.log_lines.len(), 2);
        assert_eq!(log_response.more, Some(true));
        assert_eq!(log_response.next_token, Some("page-2-token".to_string()));
        assert_eq!(log_response.log_lines[0].message, "Build started");
        assert_eq!(log_response.log_lines[1].level, Some("ERROR".to_string()));
    }

    #[test]
    fn test_deserialize_execution_details_response() {
        let json = r#"{
            "status": "SUCCESS",
            "data": {
                "pipelineExecutionSummary": {
                    "planExecutionId": "exec-123",
                    "pipelineIdentifier": "build-pipeline",
                    "status": "Running",
                    "startTs": 1234567890,
                    "endTs": null,
                    "stageExecutions": [
                        {
                            "stageIdentifier": "build-stage",
                            "stageName": "Build",
                            "status": "Success",
                            "startTs": 1234567890,
                            "endTs": 1234567900,
                            "stepExecutions": [
                                {
                                    "stepIdentifier": "compile-step",
                                    "stepName": "Compile",
                                    "status": "Success",
                                    "startTs": 1234567890,
                                    "endTs": 1234567895
                                }
                            ]
                        }
                    ]
                }
            }
        }"#;

        // The execution details are nested under pipelineExecutionSummary.
        // Our parse_response extracts data, so we test the inner structure.
        let response: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        let data = response.data.unwrap();
        let summary = &data["pipelineExecutionSummary"];

        let details: ExecutionDetails = serde_json::from_value(summary.clone()).unwrap();
        assert_eq!(details.plan_execution_id, "exec-123");
        assert_eq!(details.status, ExecutionStatus::Running);
        assert_eq!(details.stage_executions.len(), 1);

        let stage = &details.stage_executions[0];
        assert_eq!(stage.stage_name, Some("Build".to_string()));
        assert_eq!(stage.status, ExecutionStatus::Success);

        let steps = stage.step_executions.as_ref().unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].step_name, Some("Compile".to_string()));
    }
}
