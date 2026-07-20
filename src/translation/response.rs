//! Response translation: Backend (Chat Completions) → DBeaver (OpenAI Responses)
//!
//! Converts a standard Chat Completions response from the backend back into
//! DBeaver's OpenAI Responses API format. Critically, it ensures that the
//! `usage` object always includes `input_tokens_details` and
//! `output_tokens_details` — DBeaver crashes with NullPointerException
//! if these fields are absent.

use crate::models::{
    CachedTokens, ChatCompletionsResponse, DBeaverResponse, ReasoningTokens, ResponseContentPart,
    ResponseOutputMessage, UsageInfo,
};

use std::time::{SystemTime, UNIX_EPOCH};

/// Translate a Chat Completions response into a DBeaver OpenAI Responses response.
///
/// # Translation rules
///
/// 1. **Text extraction**: Takes `choices[0].message.content` as the response text.
///    Falls back to an empty string if content is None.
/// 2. **ID generation**: Response ID is prefixed with `resp_`, message ID with `msg_`,
///    both using the current unix timestamp.
/// 3. **Usage mapping**: Maps `prompt_tokens` → `input_tokens`, `completion_tokens` →
///    `output_tokens`. If the backend doesn't return usage, both default to 0.
/// 4. **NPE prevention**: `input_tokens_details.cached_tokens` and
///    `output_tokens_details.reasoning_tokens` are ALWAYS present (default to 0).
pub fn translate_response(chat_resp: ChatCompletionsResponse, model: &str) -> DBeaverResponse {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Extract text from the first choice
    let text = chat_resp
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("")
        .to_string();

    // Map usage, defaulting to 0 if backend omitted it
    let usage = chat_resp.usage.as_ref();
    let input_tokens = usage.map(|u| u.prompt_tokens).unwrap_or(0);
    let output_tokens = usage.map(|u| u.completion_tokens).unwrap_or(0);

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
                text,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ChatChoice, ChatCompletionMessage, ChatUsage};

    fn make_chat_response(
        content: Option<&str>,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
    ) -> ChatCompletionsResponse {
        ChatCompletionsResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1712345678,
            model: "gpt-4o".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatCompletionMessage {
                    role: "assistant".to_string(),
                    content: content.map(|s| s.to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: prompt_tokens.map(|p| ChatUsage {
                prompt_tokens: p,
                completion_tokens: completion_tokens.unwrap_or(0),
                total_tokens: p + completion_tokens.unwrap_or(0),
            }),
        }
    }

    #[test]
    fn test_basic_response_translation() {
        let chat_resp = make_chat_response(Some("Hello!"), Some(10), Some(5));
        let result = translate_response(chat_resp, "gpt-4o");

        assert_eq!(result.object, "response");
        assert!(result.id.starts_with("resp_"));
        assert_eq!(result.model, "gpt-4o");
        assert_eq!(result.output.len(), 1);
        assert_eq!(result.output[0].content[0].text, "Hello!");
        assert_eq!(result.output[0].role, "assistant");
        assert_eq!(result.usage.input_tokens, 10);
        assert_eq!(result.usage.output_tokens, 5);
    }

    #[test]
    fn test_usage_details_always_present() {
        let chat_resp = make_chat_response(Some("Hi"), Some(5), Some(3));
        let result = translate_response(chat_resp, "gpt-4o");

        // These fields MUST always be present — DBeaver NPEs without them
        assert_eq!(result.usage.input_tokens_details.cached_tokens, 0);
        assert_eq!(result.usage.output_tokens_details.reasoning_tokens, 0);
    }

    #[test]
    fn test_usage_defaults_to_zero_when_missing() {
        // Response with no usage field
        let chat_resp = make_chat_response(Some("OK"), None, None);
        let result = translate_response(chat_resp, "gpt-4o");

        assert_eq!(result.usage.input_tokens, 0);
        assert_eq!(result.usage.output_tokens, 0);
        assert_eq!(result.usage.input_tokens_details.cached_tokens, 0);
        assert_eq!(result.usage.output_tokens_details.reasoning_tokens, 0);
    }

    #[test]
    fn test_null_content_becomes_empty_string() {
        // Backend can return content: null (e.g., for tool calls)
        let chat_resp = make_chat_response(None, Some(10), Some(2));
        let result = translate_response(chat_resp, "gpt-4o");

        assert_eq!(result.output[0].content[0].text, "");
    }

    #[test]
    fn test_empty_choices_list() {
        let chat_resp = ChatCompletionsResponse {
            id: "chatcmpl-empty".to_string(),
            object: "chat.completion".to_string(),
            created: 1712345678,
            model: "gpt-4o".to_string(),
            choices: vec![],
            usage: None,
        };

        let result = translate_response(chat_resp, "gpt-4o");
        assert_eq!(result.output[0].content[0].text, "");
        assert_eq!(result.usage.input_tokens, 0);
    }

    #[test]
    fn test_model_propagation() {
        let chat_resp = make_chat_response(Some("test"), Some(1), Some(1));
        let result = translate_response(chat_resp, "mistral-large-latest");

        assert_eq!(result.model, "mistral-large-latest");
    }

    #[test]
    fn test_output_message_structure() {
        let chat_resp = make_chat_response(Some("Response text"), Some(10), Some(5));
        let result = translate_response(chat_resp, "gpt-4o");

        let msg = &result.output[0];
        assert_eq!(msg.r#type, "message");
        assert_eq!(msg.status, "completed");
        assert!(msg.id.starts_with("msg_"));

        let content = &msg.content[0];
        assert_eq!(content.r#type, "output_text");
        assert!(content.annotations.is_none());
    }
}
