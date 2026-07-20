//! Request translation: DBeaver (OpenAI Responses) → Backend (Chat Completions)
//!
//! Converts DBeaver's `POST /v1/responses` input format into the standard
//! Chat Completions format that most AI backends (OpenAI, Mistral, etc.) expect.

use crate::config::Config;
use crate::models::{
    ChatCompletionsRequest, ChatMessage, ContentPart, DBeaverMessage, DBeaverRequest,
};

/// Translate a DBeaver OpenAI Responses API request into a Chat Completions request.
///
/// # Translation rules
///
/// 1. **Content flattening**: Each message's `content: [{type, text}, ...]` array is
///    flattened into a single string by concatenating all text fields with newlines.
/// 2. **Role mapping**: Known roles (`system`, `user`, `assistant`) are preserved.
///    Any unrecognized role defaults to `user`.
/// 3. **Stream**: Always forced to `false`. The proxy collects the full backend
///    response and handles streaming to DBeaver separately.
/// 4. **Model fallback**: Uses the request's `model` field if present, otherwise
///    falls back to `config.model`.
/// 5. **Passthrough fields**: `temperature`, `tools`, `tool_choice` are passed
///    through unchanged. `max_output_tokens` is mapped to `max_tokens`.
pub fn translate_request(dbeaver_req: DBeaverRequest, config: &Config) -> ChatCompletionsRequest {
    let model = dbeaver_req.model.unwrap_or_else(|| config.model.clone());

    let messages: Vec<ChatMessage> = dbeaver_req
        .input
        .into_iter()
        .map(translate_message)
        .collect();

    ChatCompletionsRequest {
        model,
        messages,
        stream: false,
        temperature: dbeaver_req.temperature,
        tools: dbeaver_req.tools,
        tool_choice: dbeaver_req.tool_choice,
        max_tokens: dbeaver_req.max_output_tokens,
    }
}

/// Translate a single DBeaver message into a Chat Completions message.
fn translate_message(msg: DBeaverMessage) -> ChatMessage {
    let role = normalize_role(msg.role);
    let content = flatten_content(msg.content);

    ChatMessage { role, content }
}

/// Flatten a vector of content parts into a single string.
///
/// Each part's `text` is joined with a newline separator.
/// If the content array is empty, returns an empty string.
fn flatten_content(content: Vec<ContentPart>) -> String {
    content
        .into_iter()
        .map(|part| part.text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalize a role string to one of the recognized roles.
///
/// DBeaver sends roles like `system`, `user`, `assistant`.
/// Any unknown role is mapped to `user` to avoid backend rejection.
fn normalize_role(role: String) -> String {
    match role.as_str() {
        "system" | "user" | "assistant" => role,
        _ => "user".to_string(),
    }
}

/// Create a Chat Completions request directly from flat text (utility for testing).
#[allow(dead_code)]
pub fn simple_chat_request(
    system_prompt: &str,
    user_message: &str,
    config: &Config,
) -> ChatCompletionsRequest {
    ChatCompletionsRequest {
        model: config.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ],
        stream: false,
        temperature: None,
        tools: None,
        tool_choice: None,
        max_tokens: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ContentPart, DBeaverMessage};

    fn test_config() -> Config {
        Config {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
        }
    }

    #[test]
    fn test_basic_request_translation() {
        let config = test_config();

        let dbeaver_req = DBeaverRequest {
            input: vec![
                DBeaverMessage {
                    role: "system".to_string(),
                    content: vec![ContentPart {
                        r#type: "input_text".to_string(),
                        text: "You are a SQL assistant".to_string(),
                    }],
                },
                DBeaverMessage {
                    role: "user".to_string(),
                    content: vec![ContentPart {
                        r#type: "input_text".to_string(),
                        text: "SELECT 1".to_string(),
                    }],
                },
            ],
            model: Some("mistral-large-latest".to_string()),
            stream: Some(true),
            temperature: Some(0.7),
            tools: None,
            tool_choice: None,
            max_output_tokens: Some(500),
        };

        let result = translate_request(dbeaver_req, &config);

        assert_eq!(result.model, "mistral-large-latest");
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0].role, "system");
        assert_eq!(result.messages[0].content, "You are a SQL assistant");
        assert_eq!(result.messages[1].role, "user");
        assert_eq!(result.messages[1].content, "SELECT 1");
        assert!(!result.stream, "stream must be forced to false");
        assert_eq!(result.temperature, Some(0.7));
        assert_eq!(result.max_tokens, Some(500));
    }

    #[test]
    fn test_model_fallback_to_config() {
        let config = test_config();

        let dbeaver_req = DBeaverRequest {
            input: vec![DBeaverMessage {
                role: "user".to_string(),
                content: vec![ContentPart {
                    r#type: "input_text".to_string(),
                    text: "Hi".to_string(),
                }],
            }],
            model: None,
            stream: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            max_output_tokens: None,
        };

        let result = translate_request(dbeaver_req, &config);
        assert_eq!(result.model, config.model);
    }

    #[test]
    fn test_unrecognized_role_defaults_to_user() {
        let config = test_config();

        let dbeaver_req = DBeaverRequest {
            input: vec![DBeaverMessage {
                role: "unknown-role".to_string(),
                content: vec![ContentPart {
                    r#type: "input_text".to_string(),
                    text: "test".to_string(),
                }],
            }],
            model: None,
            stream: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            max_output_tokens: None,
        };

        let result = translate_request(dbeaver_req, &config);
        assert_eq!(result.messages[0].role, "user");
    }

    #[test]
    fn test_content_flattening_multiple_parts() {
        let config = test_config();

        let dbeaver_req = DBeaverRequest {
            input: vec![DBeaverMessage {
                role: "user".to_string(),
                content: vec![
                    ContentPart {
                        r#type: "input_text".to_string(),
                        text: "Part one".to_string(),
                    },
                    ContentPart {
                        r#type: "input_text".to_string(),
                        text: "Part two".to_string(),
                    },
                ],
            }],
            model: None,
            stream: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            max_output_tokens: None,
        };

        let result = translate_request(dbeaver_req, &config);
        assert_eq!(result.messages[0].content, "Part one\nPart two");
    }

    #[test]
    fn test_empty_content_array() {
        let config = test_config();

        let dbeaver_req = DBeaverRequest {
            input: vec![DBeaverMessage {
                role: "user".to_string(),
                content: vec![],
            }],
            model: None,
            stream: None,
            temperature: None,
            tools: None,
            tool_choice: None,
            max_output_tokens: None,
        };

        let result = translate_request(dbeaver_req, &config);
        assert_eq!(result.messages[0].content, "");
    }

    #[test]
    fn test_stream_always_false() {
        let config = test_config();

        // Even when DBeaver sends stream: true
        let dbeaver_req = DBeaverRequest {
            input: vec![DBeaverMessage {
                role: "user".to_string(),
                content: vec![ContentPart {
                    r#type: "input_text".to_string(),
                    text: "hello".to_string(),
                }],
            }],
            model: None,
            stream: Some(true),
            temperature: None,
            tools: None,
            tool_choice: None,
            max_output_tokens: None,
        };

        let result = translate_request(dbeaver_req, &config);
        assert!(!result.stream, "stream must always be forced to false");
    }

    #[test]
    fn test_simple_chat_request_utility() {
        let config = test_config();
        let result = simple_chat_request("System prompt", "User message", &config);

        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0].role, "system");
        assert_eq!(result.messages[0].content, "System prompt");
        assert_eq!(result.messages[1].role, "user");
        assert_eq!(result.messages[1].content, "User message");
        assert!(!result.stream);
    }
}
