//! Message handling for the chat history.

use serde_json::Value;

use crate::core::message;

use super::{App, ChatMessage};

impl App {
    /// Populate messages from API format (user/assistant only).
    /// Malformed messages (e.g. unsupported content types) are surfaced as
    /// "[Unsupported message format]" with a log warning instead of silently omitted.
    pub(crate) fn set_messages_from_api(&mut self, api_messages: &[Value]) {
        self.messages.clear();
        for msg in api_messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
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
            match role {
                "user" => self.messages.push(ChatMessage::User(content)),
                "assistant" => self.messages.push(ChatMessage::Assistant(content)),
                _ => {}
            }
        }
    }

    pub(crate) fn push_user(&mut self, text: &str) {
        self.messages.push(ChatMessage::User(text.to_string()));
    }

    pub(crate) fn push_assistant(&mut self, text: String) {
        self.messages.push(ChatMessage::Assistant(text));
    }

    /// Append a streamed content chunk to the last Assistant message, or create one if none.
    pub(crate) fn append_assistant_chunk(&mut self, chunk: &str) {
        match self.messages.last_mut() {
            Some(ChatMessage::Assistant(s)) => s.push_str(chunk),
            _ => self
                .messages
                .push(ChatMessage::Assistant(chunk.to_string())),
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
        }
    }

    /// Replace the last Assistant message with the given content, or push if none.
    pub(crate) fn replace_or_push_assistant(&mut self, content: String) {
        if let Some(ChatMessage::Assistant(s)) = self.messages.last_mut() {
            *s = content;
        } else {
            self.messages.push(ChatMessage::Assistant(content));
        }
    }

    pub(crate) fn push_tool_log(&mut self, line: String) {
        self.messages.push(ChatMessage::ToolLog(line));
    }

    pub(crate) fn set_thinking(&mut self, thinking: bool) {
        if thinking {
            self.messages.push(ChatMessage::Thinking);
        } else {
            // Remove Thinking by value (may not be last if we streamed ToolLog during thinking)
            self.messages
                .retain(|m| !matches!(m, ChatMessage::Thinking));
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
            }
        }
    }
}
