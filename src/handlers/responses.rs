//! Handlers for `POST /v1/responses` and related endpoints.
//!
//! Contains:
//! - `handle_response` — main DBeaver translation endpoint
//! - `handle_passthrough` — legacy chat/completions passthrough
//! - `health_check` — optional metrics endpoint

use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::client::{BackendClient, BackendError, error_response_body};
use crate::models::{DBeaverRequest, ErrorDetail, ErrorResponse};
use crate::router::AppState;
use crate::sse::{json_response, stream_response};
use crate::translation::request::translate_request;
use crate::translation::response::translate_response;

/// Handle `POST /v1/responses` (and `POST /responses`).
pub async fn handle_response(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> impl IntoResponse {
    let config = state.config();

    // Validate API key from request headers
    if !config.api_key.is_empty()
        && let Err(resp) = validate_auth(request.headers(), config)
    {
        return resp;
    }

    // Parse request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return error_400("Failed to read request body"),
    };

    // Deserialize DBeaver request
    let dbeaver_req: DBeaverRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            return error_400(format!("Invalid request body: {}", e).as_str());
        }
    };

    // Check for empty input
    if dbeaver_req.input.is_empty() {
        return error_400("Request must contain at least one message");
    }

    // Determine if DBeaver wants streaming
    let wants_stream = dbeaver_req.stream.unwrap_or(false);

    // Translate to Chat Completions format
    let chat_req = translate_request(dbeaver_req, config);

    // Send to backend
    let client = BackendClient::new(config);
    let backend_result = client.chat_completions(chat_req).await;

    match backend_result {
        Ok(chat_resp) => {
            let model = chat_resp.model.clone();
            let dbeaver_resp = translate_response(chat_resp, &model);

            if wants_stream {
                (
                    StatusCode::OK,
                    stream_response(dbeaver_resp).into_response(),
                )
            } else {
                (StatusCode::OK, json_response(dbeaver_resp).into_response())
            }
        }
        Err(err) => {
            let status = backend_error_status(&err);
            let body = error_response_body(&err);
            (status, json_body(body))
        }
    }
}

/// Handle `POST /v1/chat/completions` — legacy passthrough.
pub async fn handle_passthrough(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> impl IntoResponse {
    let config = state.config();

    if !config.api_key.is_empty()
        && let Err(resp) = validate_auth(request.headers(), config)
    {
        return resp;
    }

    let body_bytes = match axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return error_400("Failed to read request body"),
    };

    let client = BackendClient::new(config);
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let backend_resp = match client.raw_post(&url, &body_bytes).await {
        Ok(resp) => resp,
        Err(err) => {
            let status = backend_error_status(&err);
            let body = error_response_body(&err);
            return (status, json_body(body));
        }
    };

    (StatusCode::OK, json_body(backend_resp))
}

/// Handle `GET /health` — health check with optional metrics.
pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if crate::metrics::Metrics::is_enabled() {
        let snapshot = state.metrics().snapshot();
        (
            StatusCode::OK,
            Json(serde_json::to_value(&snapshot).unwrap_or_default()),
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "service": "dbeaver-proxy"
            })),
        )
            .into_response()
    }
}

// ────────── Helpers ──────────

/// Validate the Authorization header from DBeaver against the configured API key.
///
/// DBeaver sends the configured token as `Authorization: Bearer <token>`.
/// Returns `Ok(())` if valid, or `Err(response)` with 401 if invalid/missing.
#[allow(clippy::result_large_err)]
fn validate_auth(
    headers: &HeaderMap,
    config: &crate::config::Config,
) -> Result<(), (StatusCode, Response)> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Extract Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .unwrap_or("");

    if token.is_empty() || token != config.api_key {
        return Err(error_401(
            "Invalid API key. Configure the same API key in DBeaver and in the proxy config.",
        ));
    }

    Ok(())
}

/// Wrap raw JSON bytes with proper content-type header.
fn json_body(body: Vec<u8>) -> Response {
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// Build error response body bytes.
fn error_body(message: &str) -> Vec<u8> {
    serde_json::to_vec(&ErrorResponse {
        error: ErrorDetail {
            message: message.to_string(),
        },
    })
    .unwrap_or_default()
}

fn error_401(message: &str) -> (StatusCode, Response) {
    (StatusCode::UNAUTHORIZED, json_body(error_body(message)))
}

fn error_400(message: &str) -> (StatusCode, Response) {
    (StatusCode::BAD_REQUEST, json_body(error_body(message)))
}

fn backend_error_status(err: &BackendError) -> StatusCode {
    match err {
        BackendError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        BackendError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        BackendError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        BackendError::ServerError(_, _) => StatusCode::BAD_GATEWAY,
        BackendError::NetworkError(_) => StatusCode::BAD_GATEWAY,
        BackendError::ParseError(_) => StatusCode::BAD_GATEWAY,
    }
}
