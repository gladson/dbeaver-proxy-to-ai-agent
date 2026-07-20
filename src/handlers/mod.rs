//! HTTP handlers for the DBeaver proxy server.
//!
//! This module provides handler functions for:
//! - `GET /v1/models` — List available models
//! - `POST /v1/responses` — Main translation endpoint
//! - `POST /v1/chat/completions` — Legacy passthrough
//! - `GET /health` — Health check

pub mod models;
pub mod responses;
