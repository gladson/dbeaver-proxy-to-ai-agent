//! Format translation between OpenAI Responses API and Chat Completions API.
//!
//! This module provides:
//! - `request`: DBeaver (OpenAI Responses) → Backend (Chat Completions)
//! - `response`: Backend (Chat Completions) → DBeaver (OpenAI Responses)

pub mod request;
pub mod response;
