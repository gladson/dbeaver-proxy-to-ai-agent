//! Handler for `GET /v1/models` — lists available models.
//!
//! This endpoint works without an API key so DBeaver can discover
//! available models before authentication.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::models::ModelListResponse;
use crate::router::AppState;

/// Handle `GET /v1/models` (and `GET /models`).
pub async fn list_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let models = vec![state.config().model.clone()];
    (StatusCode::OK, Json(ModelListResponse::new(&models)))
}
