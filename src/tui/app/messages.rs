//! Message handling for the chat history.

use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::message;

use super::{App, ChatMessage};

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl App {
    /// Populate messages from persisted format (user, assistant, tool_log).
    /// Malformed messages (e.g. unsupported content types) are surfaced as
    /// "[Unsupported message format]" with a log warning instead of silently omitted.
    /// If `fallback_timestamp` is set, it is used for messages that lack a timestamp (old format).
    pub(crate) fn set_messages_from_api(
        &mut self,
        api_messages: &[Value],
        fallback_timestamp: Option<u64>,
    ) {
        self.messages.clear();
        self.message_timestamps.clear();
        for msg in api_messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            match role {
                "user" | "assistant" => {
                    let content = match message::extract_content(msg) {
                        Some(c) => c,
                        None => {
                            let content_type = msg
                                .get("content")
                                .map(|c| {
                                    if c.is_string() {
                                        "string"
                                    } else if c.is_array() {
                                        "array"
                                    } else if c.is_object() {
                                        "object"
                                    } else {
                                        "other"
                                    }
                                })
                                .unwrap_or("missing");
                            log::warn!(
                                "Could not extract content from message: role={}, content_type={}",
                                role,
                                content_type
                            );
                            "[Unsupported message format]".to_string()
                        }
                    };
                    let timestamp = msg
                        .get("timestamp")
                        .and_then(|t| t.as_u64())
                        .or(fallback_timestamp);
                    if role == "user" {
                        self.messages.push(ChatMessage::User(content));
                        self.message_timestamps.push(timestamp);
                    } else {
                        self.messages.push(ChatMessage::Assistant(content));
                        self.message_timestamps.push(timestamp);
                    }
                }
                "tool_log" => {
                    let content = msg
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.messages.push(ChatMessage::ToolLog(content));
                    self.message_timestamps.push(None);
                }
                _ => {}
            }
        }
    }

    /// Serialize app messages to persistence format (user, assistant, tool_log).
    /// Used when saving; preserves ToolLog and timestamps for display when re-opening.
    pub(crate) fn messages_to_persist_format(
        msgs: &[ChatMessage],
        timestamps: &[Option<u64>],
    ) -> Vec<Value> {
        msgs.iter()
            .enumerate()
            .filter_map(|(i, m)| {
                let ts = timestamps.get(i).and_then(|t| *t);
                match m {
                    ChatMessage::User(s) => {
                        let mut v = serde_json::json!({"role": "user", "content": s});
                        if let Some(t) = ts {
                            v["timestamp"] = serde_json::json!(t);
                        }
                        Some(v)
                    }
                    ChatMessage::Assistant(s) => {
                        let mut v = serde_json::json!({"role": "assistant", "content": s});
                        if let Some(t) = ts {
                            v["timestamp"] = serde_json::json!(t);
                        }
                        Some(v)
                    }
                    ChatMessage::ToolLog(s) => {
                        Some(serde_json::json!({"role": "tool_log", "content": s}))
                    }
                    ChatMessage::Thinking => None,
                }
            })
            .collect()
    }

    pub(crate) fn push_user(&mut self, text: &str) {
        self.messages.push(ChatMessage::User(text.to_string()));
        self.message_timestamps.push(Some(unix_timestamp_secs()));
    }

    pub(crate) fn push_assistant(&mut self, text: String) {
        self.messages.push(ChatMessage::Assistant(text));
        self.message_timestamps.push(Some(unix_timestamp_secs()));
    }

    /// Append a streamed content chunk to the last Assistant message, or create one if none.
    pub(crate) fn append_assistant_chunk(&mut self, chunk: &str) {
        match self.messages.last_mut() {
            Some(ChatMessage::Assistant(s)) => s.push_str(chunk),
            _ => {
                self.messages
                    .push(ChatMessage::Assistant(chunk.to_string()));
                self.message_timestamps.push(Some(unix_timestamp_secs()));
            }
        }
    }

    /// Remove the last message if it is an empty Assistant (e.g. before adding tool logs).
    pub(crate) fn remove_last_if_empty_assistant(&mut self) {
        if self
            .messages
            .last()
            .is_some_and(|m| matches!(m, ChatMessage::Assistant(s) if s.is_empty()))
        {
            self.messages.pop();
            self.message_timestamps.pop();
        }
    }

    /// Replace the last Assistant message with the given content, or push if none.
    pub(crate) fn replace_or_push_assistant(&mut self, content: String) {
        if let Some(ChatMessage::Assistant(s)) = self.messages.last_mut() {
            *s = content;
        } else {
            self.messages.push(ChatMessage::Assistant(content));
            self.message_timestamps.push(Some(unix_timestamp_secs()));
        }
    }

    pub(crate) fn push_tool_log(&mut self, line: String) {
        self.messages.push(ChatMessage::ToolLog(line));
        self.message_timestamps.push(None);
    }

    pub(crate) fn set_thinking(&mut self, thinking: bool) {
        if thinking {
            self.messages.push(ChatMessage::Thinking);
            self.message_timestamps.push(None);
        } else {
            // Remove Thinking by value (may not be last if we streamed ToolLog during thinking)
            let (messages, timestamps): (Vec<_>, Vec<_>) = self
                .messages
                .drain(..)
                .zip(self.message_timestamps.drain(..))
                .filter(|(m, _)| !matches!(m, ChatMessage::Thinking))
                .unzip();
            self.messages = messages;
            self.message_timestamps = timestamps;
        }
    }

    /// Append "[cancelled]" to the last assistant message (or create one).
    /// Keeps whatever partial content was already streamed.
    pub(crate) fn append_cancelled_notice(&mut self) {
        self.remove_last_if_empty_assistant();
        match self.messages.last_mut() {
            Some(ChatMessage::Assistant(s)) if !s.is_empty() => {
                s.push_str("\n\n*[Request cancelled]*");
            }
            _ => {
                self.messages
                    .push(ChatMessage::Assistant("*[Request cancelled]*".to_string()));
                self.message_timestamps.push(Some(unix_timestamp_secs()));
            }
        }
    }
}
