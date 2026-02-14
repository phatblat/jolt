// Harness API HTTP client.
// Handles authentication, request/response processing, and error handling.

use reqwest::{Client, Response, StatusCode, header::HeaderMap, header::HeaderValue};
use serde::de::DeserializeOwned;

use super::error::{HarnessError, Result};
use super::types::{ApiError, ApiResponse};

const DEFAULT_BASE_URL: &str = "https://app.harness.io/";

/// Harness API client with authentication.
#[derive(Debug)]
pub struct HarnessClient {
    base_url: String,
    api_key: String,
    account_id: String,
    client: Client,
}

impl HarnessClient {
    /// Create a new Harness client with the given credentials.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for Harness API (e.g., "https://app.harness.io/gateway/")
    /// * `api_key` - Harness API key (Personal Access Token)
    /// * `account_id` - Harness account identifier
    pub fn new(base_url: String, api_key: String, account_id: String) -> Result<Self> {
        let mut headers = HeaderMap::new();

        // Add API key authentication header
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&api_key)
                .map_err(|e| HarnessError::Auth(format!("Invalid API key format: {}", e)))?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(HarnessError::Http)?;

        Ok(Self {
            base_url,
            api_key,
            account_id,
            client,
        })
    }

    /// Create a client from environment variables.
    ///
    /// Reads:
    /// - `HARNESS_API_KEY` - API token (required)
    /// - `HARNESS_ACCOUNT_ID` - Account identifier (required)
    /// - `HARNESS_BASE_URL` - Base URL (optional, defaults to SaaS)
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("HARNESS_API_KEY")
            .map_err(|_| HarnessError::MissingEnvVar("HARNESS_API_KEY".to_string()))?;

        let account_id = std::env::var("HARNESS_ACCOUNT_ID")
            .map_err(|_| HarnessError::MissingEnvVar("HARNESS_ACCOUNT_ID".to_string()))?;

        let base_url =
            std::env::var("HARNESS_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        Self::new(base_url, api_key, account_id)
    }

    /// Get the account ID.
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Make a GET request to the Harness API.
    pub async fn get(&self, endpoint: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(HarnessError::Http)?;

        self.check_response(response).await
    }

    /// Make a GET request with query parameters.
    pub async fn get_with_params<T: serde::Serialize + ?Sized>(
        &self,
        endpoint: &str,
        params: &T,
    ) -> Result<Response> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await
            .map_err(HarnessError::Http)?;

        self.check_response(response).await
    }

    /// Make a POST request with JSON body.
    pub async fn post<T: serde::Serialize>(&self, endpoint: &str, body: &T) -> Result<Response> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(HarnessError::Http)?;

        self.check_response(response).await
    }

    /// Parse a Harness API response envelope.
    pub async fn parse_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status_code = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| HarnessError::ParseError(format!("Failed to read response: {}", e)))?;

        // Try to parse as error response first
        if !status_code.is_success() {
            if let Ok(error) = serde_json::from_str::<ApiError>(&text) {
                return Err(HarnessError::Api {
                    code: error.code,
                    message: error.message,
                });
            }
        }

        // Parse as success response
        let api_response: ApiResponse<T> = serde_json::from_str(&text).map_err(|e| {
            HarnessError::ParseError(format!("Failed to parse response: {}. Body: {}", e, text))
        })?;

        if api_response.status != "SUCCESS" {
            return Err(HarnessError::Api {
                code: api_response.status.clone(),
                message: format!("API returned non-success status: {}", api_response.status),
            });
        }

        api_response
            .data
            .ok_or_else(|| HarnessError::ParseError("API response missing data field".to_string()))
    }

    /// Check response status and convert errors.
    async fn check_response(&self, response: Response) -> Result<Response> {
        match response.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED => Ok(response),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                let error_text = response.text().await.unwrap_or_default();
                Err(HarnessError::Auth(format!(
                    "Authentication failed: {}",
                    error_text
                )))
            }
            StatusCode::NOT_FOUND => {
                let url = response.url().to_string();
                Err(HarnessError::NotFound(url))
            }
            StatusCode::TOO_MANY_REQUESTS => Err(HarnessError::RateLimited),
            StatusCode::BAD_REQUEST => {
                let error_text = response.text().await.unwrap_or_default();
                Err(HarnessError::InvalidParameter(error_text))
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(HarnessError::Other(format!(
                    "HTTP {}: {}",
                    status, error_text
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = HarnessClient::new(
            "https://app.harness.io/".to_string(),
            "test-key".to_string(),
            "test-account".to_string(),
        )
        .unwrap();

        assert_eq!(client.account_id(), "test-account");
        assert_eq!(client.base_url(), "https://app.harness.io/");
    }

    #[test]
    fn test_from_env_missing_api_key() {
        unsafe {
            std::env::remove_var("HARNESS_API_KEY");
            std::env::remove_var("HARNESS_ACCOUNT_ID");
        }

        let result = HarnessClient::from_env();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HarnessError::MissingEnvVar(_)
        ));
    }

    #[test]
    fn test_from_env_with_vars() {
        unsafe {
            std::env::set_var("HARNESS_API_KEY", "test-key");
            std::env::set_var("HARNESS_ACCOUNT_ID", "test-account");
            std::env::set_var("HARNESS_BASE_URL", "https://custom.harness.io/");
        }

        let client = HarnessClient::from_env().unwrap();
        assert_eq!(client.account_id(), "test-account");
        assert_eq!(client.base_url(), "https://custom.harness.io/");

        // Cleanup
        unsafe {
            std::env::remove_var("HARNESS_API_KEY");
            std::env::remove_var("HARNESS_ACCOUNT_ID");
            std::env::remove_var("HARNESS_BASE_URL");
        }
    }

    #[test]
    fn test_from_env_default_base_url() {
        unsafe {
            std::env::set_var("HARNESS_API_KEY", "test-key");
            std::env::set_var("HARNESS_ACCOUNT_ID", "test-account");
            std::env::remove_var("HARNESS_BASE_URL");
        }

        let client = HarnessClient::from_env().unwrap();
        assert_eq!(client.base_url(), DEFAULT_BASE_URL);

        // Cleanup
        unsafe {
            std::env::remove_var("HARNESS_API_KEY");
            std::env::remove_var("HARNESS_ACCOUNT_ID");
        }
    }
}
