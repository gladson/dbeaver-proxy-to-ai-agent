//! HTTP router setup for the DBeaver proxy.
//!
//! Mounts all handlers with middleware and configures graceful shutdown.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use tower_http::decompression::RequestDecompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::Config;
use crate::handlers;
use crate::metrics::Metrics;

/// Combined application state shared across all handlers.
pub struct AppState {
    pub config: Config,
    pub metrics: Metrics,
}

/// Build the axum Router with all routes, state, and middleware.
pub fn build_router(config: Config, metrics: Metrics) -> Router {
    let state = Arc::new(AppState { config, metrics });

    let routes = Router::new()
        // GET /models (with and without /v1/ prefix)
        .route("/models", get(handlers::models::list_models))
        .route("/v1/models", get(handlers::models::list_models))
        // POST /responses (main translation endpoint)
        .route("/responses", post(handlers::responses::handle_response))
        .route("/v1/responses", post(handlers::responses::handle_response))
        // POST /chat/completions (legacy passthrough)
        .route(
            "/chat/completions",
            post(handlers::responses::handle_passthrough),
        )
        .route(
            "/v1/chat/completions",
            post(handlers::responses::handle_passthrough),
        )
        // GET /health
        .route("/health", get(handlers::responses::health_check));

    // Apply middleware (outermost = applies first)
    routes
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            metrics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(RequestDecompressionLayer::new())
        .with_state(state)
}

/// Middleware that records request metrics.
async fn metrics_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    if Metrics::is_enabled() {
        state.metrics.request_started();
    }

    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed().as_millis() as u64;

    if Metrics::is_enabled() {
        let is_error = response.status().is_server_error() || response.status().is_client_error();
        state.metrics.record(duration, is_error);
    }

    response
}

#[allow(dead_code)]
/// Convenience extractor for AppState.
pub type AppStateRef = Arc<AppState>;

/// Extractor accessors (used by handlers).
impl AppState {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }
}

/// Start the HTTP server and run until a shutdown signal is received.
pub async fn run(config: Config) {
    let metrics = Metrics::new();
    let app = build_router(config, metrics);

    let addr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        std::env::var("PORT").unwrap_or_else(|_| "60916".to_string())
    );

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("❌ Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    info!("DBeaver Proxy listening on http://{}", addr);
    info!("  Models:       GET  /v1/models");
    info!("  Responses:    POST /v1/responses");
    info!("  Passthrough:  POST /v1/chat/completions");
    info!("  Health:       GET  /health");
    if Metrics::is_enabled() {
        info!("  Metrics:      enabled");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| {
            eprintln!("❌ Server error: {}", e);
        });
}

/// Wait for SIGTERM or SIGINT to trigger graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = terminate => {
            info!("Received SIGTERM, shutting down...");
        }
    }
}
