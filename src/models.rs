//! Shared data types for the DBeaver proxy.
//!
//! This module defines the serialization contracts for:
//! - Incoming DBeaver requests (OpenAI Responses API format)
//! - Outgoing backend requests (Chat Completions format)
//! - Backend responses
//! - DBeaver-bound responses
//! - Model listing and error responses

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// DBeaver → Proxy (OpenAI Responses API format)
// ──────────────────────────────────────────────

/// Request from DBeaver in OpenAI Responses API format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBeaverRequest {
    /// Array of messages in Responses API format
    pub input: Vec<DBeaverMessage>,

    /// Model identifier (optional, proxy uses default if absent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    /// Tool choice mode ("auto", "none", "required", or specific tool)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Maximum number of output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

/// A single message in the DBeaver request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBeaverMessage {
    /// Message role: "system", "user", or "assistant"
    pub role: String,

    /// Content parts (DBeaver sends an array of content blocks)
    pub content: Vec<ContentPart>,
}

/// A single content part within a DBeaver message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    /// Content type, e.g. "input_text"
    #[serde(rename = "type")]
    pub r#type: String,

    /// The text content
    pub text: String,
}

// ──────────────────────────────────────
// Proxy → Backend (Chat Completions API)
// ──────────────────────────────────────

/// Request sent to the backend in Chat Completions format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsRequest {
    /// Model to use
    pub model: String,

    /// Array of messages in Chat Completions format
    pub messages: Vec<ChatMessage>,

    /// Whether to stream (always false — proxy collects full response)
    pub stream: bool,

    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    /// Tool choice mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Maximum tokens (mapped from DBeaver's max_output_tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// A single message in Chat Completions format (flat content string).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role: "system", "user", or "assistant"
    pub role: String,

    /// Plain text content
    pub content: String,
}

/// Response from the backend in Chat Completions format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionsResponse {
    /// Response ID
    pub id: String,

    /// Object type, e.g. "chat.completion"
    pub object: String,

    /// Unix timestamp of creation
    pub created: u64,

    /// Model used
    pub model: String,

    /// Array of completion choices
    pub choices: Vec<ChatChoice>,

    /// Token usage information (may be absent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ChatUsage>,
}

/// A single completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// Index of this choice
    pub index: u32,

    /// The completion message
    pub message: ChatCompletionMessage,

    /// Reason for finishing: "stop", "length", "tool_calls", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// A message within a Chat Completions response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    /// Message role
    pub role: String,

    /// Content text (may be null for tool calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Tool calls (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUsage {
    /// Tokens used in the prompt
    pub prompt_tokens: u32,

    /// Tokens used in the completion
    pub completion_tokens: u32,

    /// Total tokens
    pub total_tokens: u32,
}

// ──────────────────────────────────────
// Proxy → DBeaver (OpenAI Responses API)
// ──────────────────────────────────────

/// Response sent back to DBeaver in OpenAI Responses API format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DBeaverResponse {
    /// Response ID, prefixed with "resp_"
    pub id: String,

    /// Always "response"
    pub object: String,

    /// Unix timestamp
    pub created: u64,

    /// Model that generated the response
    pub model: String,

    /// Array of output messages
    pub output: Vec<ResponseOutputMessage>,

    /// Token usage information
    pub usage: UsageInfo,
}

/// A single output message in the DBeaver response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOutputMessage {
    /// Message ID, prefixed with "msg_"
    pub id: String,

    /// Always "message"
    #[serde(rename = "type")]
    pub r#type: String,

    /// Status, e.g. "completed"
    pub status: String,

    /// Role, e.g. "assistant"
    pub role: String,

    /// Array of content parts
    pub content: Vec<ResponseContentPart>,
}

/// A single content part in the DBeaver response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContentPart {
    /// Always "output_text"
    #[serde(rename = "type")]
    pub r#type: String,

    /// The response text
    pub text: String,

    /// Annotations (may be null)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<String>>,
}

/// Token usage information for DBeaver.
///
/// DBeaver expects `input_tokens_details` and `output_tokens_details`
/// to always be present — omitting them causes a NullPointerException.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Tokens used in the input
    pub input_tokens: u32,

    /// Details about input tokens (DBeaver requires this field)
    pub input_tokens_details: CachedTokens,

    /// Tokens used in the output
    pub output_tokens: u32,

    /// Details about output tokens (DBeaver requires this field)
    pub output_tokens_details: ReasoningTokens,
}

/// Cached token info (always present to satisfy DBeaver expectations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTokens {
    /// Number of cached tokens (defaults to 0)
    pub cached_tokens: u32,
}

/// Reasoning token info (always present to satisfy DBeaver expectations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTokens {
    /// Number of reasoning tokens (defaults to 0)
    pub reasoning_tokens: u32,
}

// ────────────────────────────
// Models listing
// ────────────────────────────

/// Response from GET /v1/models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    /// Always "list"
    pub object: String,

    /// Array of available models
    pub data: Vec<ModelInfo>,
}

/// Information about a single model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID
    pub id: String,

    /// Always "model"
    pub object: String,

    /// Unix timestamp
    pub created: u64,

    /// Owner of the model
    pub owned_by: String,
}

// ────────────────────────────
// Error response
// ────────────────────────────

/// Standard error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ErrorResponse {
    /// Error details
    pub error: ErrorDetail,
}

/// Error detail fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ErrorDetail {
    /// Human-readable error message
    pub message: String,
}

// ────────────────────────────
// Tool definitions
// ────────────────────────────

/// Definition of a tool available to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool type, e.g. "function"
    #[serde(rename = "type")]
    pub r#type: String,

    /// Function definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionDefinition>,
}

/// Definition of a callable function tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,

    /// Description of what the function does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for function parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Tool choice configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// String mode: "auto", "none", "required"
    Mode(String),

    /// Specific tool: {"type": "function", "function": {"name": "..."}}
    Specific {
        /// Tool type
        #[serde(rename = "type")]
        r#type: String,
        /// Function to call
        function: SpecificFunction,
    },
}

/// Specific function reference in tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificFunction {
    /// Function name
    pub name: String,
}

/// A tool call made by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID
    pub id: String,

    /// Tool type
    #[serde(rename = "type")]
    pub r#type: String,

    /// Function details
    pub function: FunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,

    /// Arguments as a JSON string
    pub arguments: String,
}

// ────────────────────────────
// Helpers
// ────────────────────────────

impl DBeaverResponse {
    /// Create a new response placeholder with the given text and model.
    #[allow(dead_code)]
    pub fn new(text: &str, model: &str, input_tokens: u32, output_tokens: u32) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        DBeaverResponse {
            id: format!("resp_{}", now),
            object: "response".to_string(),
            created: now,
            model: model.to_string(),
            output: vec![ResponseOutputMessage {
                id: format!("msg_{}", now),
                r#type: "message".to_string(),
                status: "completed".to_string(),
                role: "assistant".to_string(),
                content: vec![ResponseContentPart {
                    r#type: "output_text".to_string(),
                    text: text.to_string(),
                    annotations: None,
                }],
            }],
            usage: UsageInfo {
                input_tokens,
                input_tokens_details: CachedTokens { cached_tokens: 0 },
                output_tokens,
                output_tokens_details: ReasoningTokens {
                    reasoning_tokens: 0,
                },
            },
        }
    }
}

impl ModelListResponse {
    /// Create a model list response from a list of model IDs.
    pub fn new(models: &[String]) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        ModelListResponse {
            object: "list".to_string(),
            data: models
                .iter()
                .map(|id| ModelInfo {
                    id: id.clone(),
                    object: "model".to_string(),
                    created: now,
                    owned_by: "dbeaver-proxy".to_string(),
                })
                .collect(),
        }
    }
}

// ────────────────────────────
// Tests
// ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_dbeaver_request() {
        let json = serde_json::json!({
            "input": [
                {
                    "type": "message",
                    "role": "system",
                    "content": [{"type": "input_text", "text": "You are a SQL assistant"}]
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "SELECT 1"}]
                }
            ],
            "model": "gpt-4o",
            "stream": true,
            "temperature": 0.3
        });

        let req: DBeaverRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.input.len(), 2);
        assert_eq!(req.input[0].role, "system");
        assert_eq!(req.input[0].content[0].text, "You are a SQL assistant");
        assert_eq!(req.input[1].content[0].text, "SELECT 1");
        assert_eq!(req.model.unwrap(), "gpt-4o");
        assert_eq!(req.stream, Some(true));
    }

    #[test]
    fn test_deserialize_dbeaver_request_with_tools() {
        let json = serde_json::json!({
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "What's the weather?"}]
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get the weather",
                        "parameters": {"type": "object", "properties": {}}
                    }
                }
            ],
            "tool_choice": "auto"
        });

        let req: DBeaverRequest = serde_json::from_value(json).unwrap();
        assert!(req.tools.is_some());
        assert_eq!(req.tools.unwrap().len(), 1);
        assert!(req.tool_choice.is_some());
    }

    #[test]
    fn test_serialize_chat_completions_request() {
        let req = ChatCompletionsRequest {
            model: "gpt-4o".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are helpful.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                },
            ],
            stream: false,
            temperature: Some(0.5),
            tools: None,
            tool_choice: None,
            max_tokens: Some(100),
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["stream"], false);
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
        assert_eq!(json["max_tokens"], 100);
    }

    #[test]
    fn test_deserialize_chat_completions_response() {
        let json = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1712345678,
            "model": "gpt-4o",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help?"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let resp: ChatCompletionsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.id, "chatcmpl-123");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(
            resp.choices[0].message.content.as_deref(),
            Some("Hello! How can I help?")
        );
        assert_eq!(resp.usage.as_ref().unwrap().prompt_tokens, 10);
    }

    #[test]
    fn test_deserialize_chat_completions_response_no_usage() {
        let json = serde_json::json!({
            "id": "chatcmpl-456",
            "object": "chat.completion",
            "created": 1712345679,
            "model": "gpt-4o",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Sure!"
                    },
                    "finish_reason": "stop"
                }
            ]
        });

        let resp: ChatCompletionsResponse = serde_json::from_value(json).unwrap();
        assert!(resp.usage.is_none());
    }

    #[test]
    fn test_serialize_dbeaver_response() {
        let resp = DBeaverResponse::new("Hello!", "gpt-4o", 10, 5);

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["object"], "response");
        assert_eq!(json["output"][0]["content"][0]["text"], "Hello!");
        assert!(json["usage"]["input_tokens_details"].is_object());
        assert!(json["usage"]["output_tokens_details"].is_object());
        assert_eq!(json["usage"]["input_tokens_details"]["cached_tokens"], 0);
        assert_eq!(
            json["usage"]["output_tokens_details"]["reasoning_tokens"],
            0
        );
        assert_eq!(json["usage"]["input_tokens"], 10);
        assert_eq!(json["usage"]["output_tokens"], 5);
    }

    #[test]
    fn test_serialize_model_list() {
        let models = vec!["gpt-4o".to_string(), "gpt-3.5-turbo".to_string()];
        let resp = ModelListResponse::new(&models);

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["object"], "list");
        assert_eq!(json["data"].as_array().unwrap().len(), 2);
        assert_eq!(json["data"][0]["id"], "gpt-4o");
        assert_eq!(json["data"][1]["id"], "gpt-3.5-turbo");
    }

    #[test]
    fn test_tool_choice_deserialize() {
        // String variant
        let json = serde_json::json!("auto");
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        match tc {
            ToolChoice::Mode(m) => assert_eq!(m, "auto"),
            _ => panic!("Expected Mode variant"),
        }

        // Object variant
        let json = serde_json::json!({
            "type": "function",
            "function": {"name": "get_weather"}
        });
        let tc: ToolChoice = serde_json::from_value(json).unwrap();
        match tc {
            ToolChoice::Specific { r#type, function } => {
                assert_eq!(r#type, "function");
                assert_eq!(function.name, "get_weather");
            }
            _ => panic!("Expected Specific variant"),
        }
    }
}
