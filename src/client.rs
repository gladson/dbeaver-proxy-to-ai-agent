//! Backend HTTP client for communicating with the AI provider.
//!
//! Wraps `reqwest::Client` to provide a typed interface for making
//! Chat Completions requests to any OpenAI-compatible backend
//! (OmniRoute, OpenAI, Mistral, etc.).

use crate::config::Config;
use crate::models::{ChatCompletionsRequest, ChatCompletionsResponse, ErrorDetail, ErrorResponse};
use std::time::Duration;

/// Errors that can occur when communicating with the backend.
#[derive(Debug)]
pub enum BackendError {
    /// Backend returned 401 Unauthorized
    Unauthorized(String),
    /// Request exceeded the configured timeout
    Timeout,
    /// Backend returned 429 Rate Limited
    RateLimited,
    /// Backend returned a 5xx error
    ServerError(u16, String),
    /// Network-level failure (connection refused, DNS, TLS, etc.)
    NetworkError(String),
    /// Response wasn't valid JSON
    ParseError(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            BackendError::Timeout => write!(f, "Backend request timed out"),
            BackendError::RateLimited => write!(f, "Rate limited by backend"),
            BackendError::ServerError(code, msg) => {
                write!(f, "Backend server error {}: {}", code, msg)
            }
            BackendError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            BackendError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

/// HTTP client for making requests to the AI backend.
pub struct BackendClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    #[allow(dead_code)]
    timeout: Duration,
}

impl BackendClient {
    /// Create a new backend client from the application config.
    pub fn new(config: &Config) -> Self {
        let timeout = Duration::from_secs(60);

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        BackendClient {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
            timeout,
        }
    }

    /// Send a raw POST request to an arbitrary URL (for passthrough).
    ///
    /// Returns the raw response body as bytes on success.
    pub async fn raw_post(&self, url: &str, body: &[u8]) -> Result<Vec<u8>, BackendError> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BackendError::Timeout
                } else {
                    BackendError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();

        if status.is_success() {
            response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| BackendError::NetworkError(format!("Failed to read response: {}", e)))
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(BackendError::Unauthorized(
                response.text().await.unwrap_or_default(),
            ))
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(BackendError::RateLimited)
        } else {
            Err(BackendError::ServerError(
                status.as_u16(),
                response.text().await.unwrap_or_default(),
            ))
        }
    }

    /// Send a Chat Completions request to the backend.
    ///
    /// Returns the parsed `ChatCompletionsResponse` on success,
    /// or a `BackendError` on any failure.
    pub async fn chat_completions(
        &self,
        request: ChatCompletionsRequest,
    ) -> Result<ChatCompletionsResponse, BackendError> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    BackendError::Timeout
                } else {
                    BackendError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();

        if status.is_success() {
            response
                .json::<ChatCompletionsResponse>()
                .await
                .map_err(|e| {
                    BackendError::ParseError(format!("Failed to parse backend response: {}", e))
                })
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            let body = response.text().await.unwrap_or_default();
            Err(BackendError::Unauthorized(body))
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Err(BackendError::RateLimited)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(BackendError::ServerError(status.as_u16(), body))
        }
    }
}

/// Create an error response JSON body for a BackendError.
pub fn error_response_body(error: &BackendError) -> Vec<u8> {
    let message = match error {
        BackendError::Unauthorized(msg) => {
            if msg.is_empty() {
                "Authentication failed. Check your API key.".to_string()
            } else {
                // Try to extract a meaningful message from the backend error body
                // Backend may return JSON like {"error": {"message": "..."}}
                serde_json::from_str::<serde_json::Value>(msg)
                    .ok()
                    .and_then(|v| {
                        v.get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str().map(String::from))
                    })
                    .unwrap_or_else(|| msg.clone())
            }
        }
        BackendError::Timeout => "Backend request timed out.".to_string(),
        BackendError::RateLimited => "Rate limited by backend. Try again later.".to_string(),
        BackendError::ServerError(_code, msg) => {
            if msg.is_empty() {
                format!("Backend server error (status {})", _code)
            } else {
                // Try to extract a meaningful message from the backend error body
                serde_json::from_str::<serde_json::Value>(msg)
                    .ok()
                    .and_then(|v| {
                        v.get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str().map(String::from))
                    })
                    .unwrap_or_else(|| msg.clone())
            }
        }
        BackendError::NetworkError(msg) => format!("Network error: {}", msg),
        BackendError::ParseError(msg) => format!("Parse error: {}", msg),
    };

    let error_resp = ErrorResponse {
        error: ErrorDetail { message },
    };

    serde_json::to_vec(&error_resp).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_error_display() {
        assert_eq!(
            BackendError::Timeout.to_string(),
            "Backend request timed out"
        );
        assert_eq!(
            BackendError::RateLimited.to_string(),
            "Rate limited by backend"
        );
        assert!(
            BackendError::Unauthorized("bad key".to_string())
                .to_string()
                .contains("bad key")
        );
        assert!(
            BackendError::ServerError(502, "bad gateway".to_string())
                .to_string()
                .contains("502")
        );
    }

    #[test]
    fn test_error_response_body_format() {
        let body = error_response_body(&BackendError::Timeout);
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["error"]["message"], "Backend request timed out.");
    }
}
