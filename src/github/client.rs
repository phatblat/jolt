// GitHub API HTTP client.
// Handles authentication, rate limiting, and request/response processing.

use reqwest::{
    Client, Response, StatusCode,
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
};

use crate::error::{JoltError, Result};

use super::types::RateLimit;

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";

/// GitHub API client with authentication and rate limit tracking.
pub struct GitHubClient {
    client: Client,
    rate_limit: RateLimit,
}

impl GitHubClient {
    /// Create a new GitHub client with the given token.
    pub fn new(token: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|e| JoltError::Other(e.to_string()))?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static(GITHUB_API_VERSION),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("jolt-tui"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(JoltError::Api)?;

        Ok(Self {
            client,
            rate_limit: RateLimit::default(),
        })
    }

    /// Create a client from the GITHUB_TOKEN environment variable.
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| JoltError::MissingToken)?;
        Self::new(&token)
    }

    /// Get the current rate limit information.
    pub fn rate_limit(&self) -> &RateLimit {
        &self.rate_limit
    }

    /// Make a GET request to the GitHub API.
    pub async fn get(&mut self, endpoint: &str) -> Result<Response> {
        let url = format!("{}{}", GITHUB_API_BASE, endpoint);
        let response = self.client.get(&url).send().await.map_err(JoltError::Api)?;

        self.update_rate_limit(&response);
        self.check_response(response).await
    }

    /// Make a GET request with query parameters.
    pub async fn get_with_params<T: serde::Serialize + ?Sized>(
        &mut self,
        endpoint: &str,
        params: &T,
    ) -> Result<Response> {
        let url = format!("{}{}", GITHUB_API_BASE, endpoint);
        let response = self
            .client
            .get(&url)
            .query(params)
            .send()
            .await
            .map_err(JoltError::Api)?;

        self.update_rate_limit(&response);
        self.check_response(response).await
    }

    /// Update rate limit from response headers.
    fn update_rate_limit(&mut self, response: &Response) {
        if let Some(limit) = response
            .headers()
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
        {
            self.rate_limit.limit = limit;
        }

        if let Some(remaining) = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
        {
            self.rate_limit.remaining = remaining;
        }

        if let Some(reset) = response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
        {
            self.rate_limit.reset = reset;
        }
    }

    /// Check response status and convert errors.
    async fn check_response(&self, response: Response) -> Result<Response> {
        match response.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED => Ok(response),
            StatusCode::UNAUTHORIZED => Err(JoltError::Unauthorized),
            StatusCode::NOT_FOUND => {
                let url = response.url().to_string();
                Err(JoltError::NotFound(url))
            }
            StatusCode::FORBIDDEN => {
                // Check if rate limited
                if self.rate_limit.remaining == 0 {
                    let reset_at =
                        chrono::DateTime::from_timestamp(self.rate_limit.reset as i64, 0)
                            .map(|dt| dt.format("%H:%M:%S").to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                    Err(JoltError::RateLimited { reset_at })
                } else {
                    Err(JoltError::Other(format!(
                        "Forbidden: {}",
                        response.text().await.unwrap_or_default()
                    )))
                }
            }
            status => Err(JoltError::Other(format!(
                "HTTP {}: {}",
                status,
                response.text().await.unwrap_or_default()
            ))),
        }
    }
}
