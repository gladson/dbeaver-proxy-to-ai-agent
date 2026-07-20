//! SSE (Server-Sent Events) streaming for DBeaver responses.
//!
//! DBeaver expects two SSE event types when `stream: true`:
//! 1. `response.output_text.delta` — contains the assistant's text
//! 2. `response.completed` — contains the full response object
//!
//! Note: The proxy does NOT stream token-by-token from the backend.
//! It collects the full response, then emits the complete text as a single
//! delta event followed by the completion event. This matches the behavior
//! of the original Python proxy.

use axum::body::Body;
use axum::http::header;
use axum::response::Response;

use crate::models::DBeaverResponse;

/// Build an SSE streaming response for DBeaver.
///
/// Emits two events:
/// 1. `response.output_text.delta` with the full assistant text
/// 2. `response.completed` with the complete response object
pub fn stream_response(response: DBeaverResponse) -> Response<Body> {
    let text = response
        .output
        .first()
        .and_then(|m| m.content.first())
        .map(|c| c.text.as_str())
        .unwrap_or("");

    let delta_event = format!(
        "event: response.output_text.delta\ndata: {}\n\n",
        serde_json::json!({
            "type": "response.output_text.delta",
            "delta": text,
        })
    );

    let response_json = serde_json::to_value(&response).unwrap_or(serde_json::Value::Null);

    let complete_event = format!(
        "event: response.completed\ndata: {}\n\n",
        serde_json::json!({
            "type": "response.completed",
            "sequence_number": 1,
            "response": response_json,
        })
    );

    let body = format!("{}{}", delta_event, complete_event);

    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from(body))
        .unwrap()
}

/// Build a non-streaming JSON response for DBeaver.
pub fn json_response(response: DBeaverResponse) -> Response<Body> {
    let body = serde_json::to_vec(&response).unwrap_or_default();

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;

    async fn collect_body(response: Response<Body>) -> Bytes {
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_stream_content_type() {
        let resp = DBeaverResponse::new("Hello!", "gpt-4o", 10, 5);
        let response = stream_response(resp);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
    }

    #[tokio::test]
    async fn test_json_content_type() {
        let resp = DBeaverResponse::new("Hello!", "gpt-4o", 10, 5);
        let response = json_response(resp);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[tokio::test]
    async fn test_stream_events() {
        let resp = DBeaverResponse::new("Hello!", "gpt-4o", 10, 5);
        let response = stream_response(resp);
        let bytes = collect_body(response).await;
        let body = String::from_utf8_lossy(&bytes);

        assert!(body.contains("event: response.output_text.delta"));
        assert!(body.contains(r#""delta":"Hello!""#));
        assert!(body.contains("event: response.completed"));
        assert!(body.contains(r#""sequence_number":1"#));
        assert!(body.contains(r#""type":"response.completed""#));
    }

    #[tokio::test]
    async fn test_stream_full_response_in_completed() {
        let resp = DBeaverResponse::new("Hello!", "gpt-4o", 10, 5);
        let response = stream_response(resp);
        let bytes = collect_body(response).await;
        let body = String::from_utf8_lossy(&bytes);

        assert!(body.contains(r#""object":"response""#));
        assert!(body.contains(r#""model":"gpt-4o""#));
        assert!(body.contains(r#""input_tokens":10"#));
        assert!(body.contains(r#""output_tokens":5"#));
        assert!(body.contains(r#""cached_tokens":0"#));
        assert!(body.contains(r#""reasoning_tokens":0"#));
    }

    #[tokio::test]
    async fn test_json_response_body() {
        let resp = DBeaverResponse::new("JSON body", "gpt-4o", 10, 5);
        let response = json_response(resp);
        let body = collect_body(response).await;

        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["object"], "response");
        assert_eq!(value["output"][0]["content"][0]["text"], "JSON body");
    }
}
