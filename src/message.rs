//! Typed model of the messages emitted by the `claude` CLI on its
//! `--output-format stream-json` stdout stream.
//!
//! Each line of output is a single JSON object with a top-level `type`
//! discriminant. Field names mirror the CLI's JSON exactly via serde renames.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single message emitted by the `claude` CLI.
///
/// The enum is tagged on the top-level `type` field. Any message whose `type`
/// is not modelled here (for example `stream_event`, or a future type) decodes
/// into [`Message::Unknown`] instead of failing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    /// A system message. The `subtype` field distinguishes variants (e.g.
    /// `"init"`); the init fields are inlined here.
    #[serde(rename = "system")]
    System {
        /// The system message subtype, e.g. `"init"`.
        subtype: String,
        /// Where the API key was sourced from, when known.
        #[serde(rename = "apiKeySource", skip_serializing_if = "Option::is_none")]
        api_key_source: Option<String>,
        /// The working directory the CLI is operating in.
        #[serde(default)]
        cwd: String,
        /// The session identifier.
        #[serde(default)]
        session_id: String,
        /// The tools available in this session.
        #[serde(default)]
        tools: Vec<String>,
        /// MCP server configuration; shape varies, so kept as raw JSON.
        #[serde(default)]
        mcp_servers: Value,
        /// The active model.
        #[serde(default)]
        model: String,
        /// The active permission mode.
        #[serde(rename = "permissionMode", default)]
        permission_mode: String,
        /// Available slash commands, when reported.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slash_commands: Option<Vec<String>>,
        /// The active output style, when reported.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_style: Option<String>,
        /// A unique identifier for this message, when reported.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },

    /// An assistant message wrapping a nested Anthropic message.
    #[serde(rename = "assistant")]
    Assistant {
        /// The nested Anthropic message.
        message: AnthropicMessage,
        /// The session identifier.
        #[serde(default)]
        session_id: String,
        /// The parent tool-use id, when this message is a sub-agent turn.
        #[serde(default)]
        parent_tool_use_id: Option<String>,
        /// A unique identifier for this message, when reported.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },

    /// A user message wrapping a nested message param.
    #[serde(rename = "user")]
    User {
        /// The nested user message.
        message: MessageParam,
        /// The session identifier.
        #[serde(default)]
        session_id: String,
        /// The parent tool-use id, when this message is a sub-agent turn.
        #[serde(default)]
        parent_tool_use_id: Option<String>,
        /// A unique identifier for this message, when reported.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },

    /// A terminal result message that ends the stream.
    #[serde(rename = "result")]
    Result {
        /// The result subtype: `"success"`, `"error_max_turns"`, or
        /// `"error_during_execution"`.
        subtype: String,
        /// Total wall-clock duration in milliseconds.
        #[serde(default)]
        duration_ms: u64,
        /// Duration spent in API calls in milliseconds.
        #[serde(default)]
        duration_api_ms: u64,
        /// Whether the run ended in an error.
        #[serde(default)]
        is_error: bool,
        /// Number of turns taken.
        #[serde(default)]
        num_turns: u64,
        /// The session identifier.
        #[serde(default)]
        session_id: String,
        /// Total cost of the run in USD.
        #[serde(default)]
        total_cost_usd: f64,
        /// Aggregate usage information; shape varies, so kept as raw JSON.
        #[serde(default)]
        usage: Value,
        /// The final result text; only present on success.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        /// A unique identifier for this message, when reported.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },

    /// Any message type not modelled above (e.g. `stream_event` or a future
    /// type). Ensures deserialization of the stream never fails on an unknown
    /// discriminant.
    #[serde(other)]
    Unknown,
}

/// The nested Anthropic message carried by an [`Message::Assistant`] variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    /// The message identifier, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The model that produced the message, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The role of the message, when present (typically `"assistant"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The reason generation stopped, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Usage information; shape varies, so kept as raw JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Value>,
    /// The content blocks that make up the message.
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// The nested user message carried by an [`Message::User`] variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParam {
    /// The role of the message, when present (typically `"user"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The content blocks that make up the message.
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// A single content block within a message.
///
/// Tagged on the block's own `type` field, snake-cased to match the CLI's
/// literals (`text`, `tool_use`, `tool_result`). Unknown block types decode
/// into [`ContentBlock::Unknown`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// A plain-text block.
    Text {
        /// The text content.
        text: String,
    },
    /// An extended-thinking block.
    Thinking {
        /// The thinking content.
        thinking: String,
        /// The cryptographic signature for the thinking block, when present.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// A tool-use request block.
    ToolUse {
        /// The tool-use identifier.
        id: String,
        /// The name of the tool being invoked.
        name: String,
        /// The tool input; shape varies per tool, so kept as raw JSON.
        input: Value,
    },
    /// A tool-result block (typically carried within a user message).
    ToolResult {
        /// The id of the tool-use this result corresponds to.
        tool_use_id: String,
        /// The result content; shape varies, so kept as raw JSON.
        #[serde(default)]
        content: Value,
        /// Whether the tool call resulted in an error.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Any content block type not modelled above.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_system_init_line() {
        let line = r#"{"type":"system","subtype":"init","apiKeySource":"none","cwd":"/tmp/example","session_id":"sess_test","tools":["Read","Bash"],"mcp_servers":[],"model":"claude-sonnet","permissionMode":"default","slash_commands":["clear"],"uuid":"u-1"}"#;
        let msg: Message = serde_json::from_str(line).expect("system line should decode");
        match msg {
            Message::System {
                subtype,
                session_id,
                model,
                tools,
                ..
            } => {
                assert_eq!(subtype, "init");
                assert_eq!(session_id, "sess_test");
                assert_eq!(model, "claude-sonnet");
                assert_eq!(tools, vec!["Read".to_string(), "Bash".to_string()]);
            }
            other => panic!("expected System, got {other:?}"),
        }
    }

    #[test]
    fn deserializes_assistant_with_text_and_tool_use() {
        let line = r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet","role":"assistant","stop_reason":"tool_use","content":[{"type":"text","text":"hello there"},{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"ls"}}]},"session_id":"sess_test","parent_tool_use_id":null}"#;
        let msg: Message = serde_json::from_str(line).expect("assistant line should decode");
        let content = match msg {
            Message::Assistant { message, .. } => message.content,
            other => panic!("expected Assistant, got {other:?}"),
        };
        assert_eq!(content.len(), 2);

        let text = content
            .iter()
            .find_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .expect("should contain a text block");
        assert_eq!(text, "hello there");

        let (name, input) = content
            .iter()
            .find_map(|b| match b {
                ContentBlock::ToolUse { name, input, .. } => Some((name.clone(), input.clone())),
                _ => None,
            })
            .expect("should contain a tool_use block");
        assert_eq!(name, "Bash");
        assert_eq!(input["command"], "ls");
    }

    #[test]
    fn deserializes_thinking_block() {
        let line = r#"{"type":"thinking","thinking":"let me reason","signature":"sig123"}"#;
        let block: ContentBlock =
            serde_json::from_str(line).expect("thinking block should decode");
        match block {
            ContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "let me reason");
                assert_eq!(signature, Some("sig123".to_string()));
            }
            other => panic!("expected Thinking, got {other:?}"),
        }
    }

    #[test]
    fn deserializes_user_with_tool_result() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu_1","content":"file listing","is_error":false}]},"session_id":"sess_test","parent_tool_use_id":"tu_1"}"#;
        let msg: Message = serde_json::from_str(line).expect("user line should decode");
        let content = match msg {
            Message::User { message, .. } => message.content,
            other => panic!("expected User, got {other:?}"),
        };
        let (tool_use_id, is_error) = content
            .iter()
            .find_map(|b| match b {
                ContentBlock::ToolResult {
                    tool_use_id,
                    is_error,
                    ..
                } => Some((tool_use_id.clone(), *is_error)),
                _ => None,
            })
            .expect("should contain a tool_result block");
        assert_eq!(tool_use_id, "tu_1");
        assert_eq!(is_error, Some(false));
    }

    #[test]
    fn deserializes_result_success_line() {
        let line = r#"{"type":"result","subtype":"success","duration_ms":1200,"duration_api_ms":900,"is_error":false,"num_turns":3,"session_id":"sess_test","total_cost_usd":0.0123,"usage":{"input_tokens":10},"result":"all done","uuid":"u-2"}"#;
        let msg: Message = serde_json::from_str(line).expect("result line should decode");
        match msg {
            Message::Result {
                subtype,
                is_error,
                num_turns,
                total_cost_usd,
                result,
                ..
            } => {
                assert_eq!(subtype, "success");
                assert!(!is_error);
                assert_eq!(num_turns, 3);
                assert!((total_cost_usd - 0.0123).abs() < f64::EPSILON);
                assert_eq!(result.as_deref(), Some("all done"));
            }
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[test]
    fn unknown_type_decodes_to_unknown_without_error() {
        let line = r#"{"type":"stream_event","event":{"delta":{"text":"partial"}},"session_id":"sess_test"}"#;
        let msg: Message = serde_json::from_str(line).expect("unknown line must not fail");
        assert!(matches!(msg, Message::Unknown));
    }
}
