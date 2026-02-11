//! TUI application state: messages, input, scroll, suggestions.

use crate::core::history::ConversationMeta;
use crate::core::llm::ConfirmState;
use crate::core::models::ModelInfo;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use serde_json::Value;
use std::time::Instant;

fn extract_content_value(msg: &Value) -> Option<String> {
    let content = msg.get("content")?;
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        for block in arr {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// Messages displayed in the history (user or assistant).
#[derive(Clone)]
pub enum ChatMessage {
    User(String),
    Assistant(String),
    Thinking,
    /// Tool call log line for verbose output.
    ToolLog(String),
}

/// Pending confirmation for a destructive command (popup displayed).
pub struct ConfirmPopup {
    pub command: String,
    pub state: ConfirmState,
}

/// State for the model selector popup.
pub struct ModelSelectorState {
    pub models: Vec<ModelInfo>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub fetch_error: Option<String>,
    /// Filter query (case-insensitive search on model id/name).
    pub filter: String,
}

/// State for the history selector popup (Alt+H).
pub struct HistorySelectorState {
    pub conversations: Vec<ConversationMeta>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub filter: String,
    /// When renaming: (conversation_id, new_title_input).
    pub renaming: Option<(String, String)>,
}

/// Scroll position: either a specific line index, or "at bottom" (follow new content).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollPosition {
    Line(usize),
    Bottom,
}

impl Default for ScrollPosition {
    fn default() -> Self {
        Self::Line(0)
    }
}

pub struct App {
    pub(crate) messages: Vec<ChatMessage>,
    /// User input in the text field.
    pub(crate) input: String,
    pub(crate) scroll: ScrollPosition,
    pub(crate) last_max_scroll: usize,
    /// Index of the selected suggestion (Tab to cycle).
    pub selected_suggestion: usize,
    /// When set, show confirmation popup and ignore normal input until y/n.
    pub confirm_popup: Option<ConfirmPopup>,
    /// Model ID displayed in the header and used for chat (e.g. "anthropic/claude-haiku-4.5").
    pub model_name: String,
    /// Same as model_name; used for API calls.
    pub current_model_id: String,
    /// When set, show model selector popup (Alt+M).
    pub model_selector: Option<ModelSelectorState>,
    /// When set, show history selector popup (Alt+H).
    pub history_selector: Option<HistorySelectorState>,
    /// Content width from last draw; used to compute scroll-to-start when adding new messages.
    pub(crate) last_content_width: Option<usize>,
    /// Credit balance: (total_credits, total_usage). Fetched on startup, refreshed every 30 min.
    pub(crate) credit_data: Option<(f64, f64)>,
    /// Rect of credits widget in header; used for click detection and hover.
    pub(crate) credits_header_rect: Option<Rect>,
    /// When credits were last successfully fetched; for 30-min refresh.
    pub(crate) credits_last_fetched_at: Option<Instant>,
    /// Mouse is over credits area; used for cursor style.
    pub(crate) hovering_credits: bool,
    /// Current conversation ID; None = new unsaved conversation.
    pub(crate) current_conversation_id: Option<String>,
    /// True if content has changed since last save.
    pub(crate) dirty: bool,
    /// Esc was pressed; next key = Option+key (Mac terminals with "Use option as meta").
    pub(crate) escape_pending: bool,
    /// True while a chat request is in flight (used by bottom bar to show cancel hint).
    pub(crate) is_streaming: bool,
}

impl App {
    pub fn new(model_id: String, model_name: String) -> Self {
        Self {
            messages: vec![],
            input: String::new(),
            scroll: ScrollPosition::default(),
            last_max_scroll: 0,
            selected_suggestion: 0,
            confirm_popup: None,
            model_name,
            current_model_id: model_id,
            model_selector: None,
            history_selector: None,
            last_content_width: None,
            credit_data: None,
            credits_header_rect: None,
            credits_last_fetched_at: None,
            hovering_credits: false,
            current_conversation_id: None,
            dirty: false,
            escape_pending: false,
            is_streaming: false,
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub(crate) fn set_conversation_id(&mut self, id: Option<String>) {
        self.current_conversation_id = id;
    }

    pub(crate) fn conversation_id(&self) -> Option<&str> {
        self.current_conversation_id.as_deref()
    }

    /// Reset to a new empty conversation.
    pub(crate) fn new_conversation(&mut self) {
        self.messages.clear();
        self.current_conversation_id = None;
        self.dirty = false;
        self.scroll = ScrollPosition::default();
        self.last_max_scroll = 0;
    }

    /// Populate messages from API format (user/assistant only).
    pub(crate) fn set_messages_from_api(&mut self, api_messages: &[Value]) {
        self.messages.clear();
        for msg in api_messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let content = extract_content_value(msg).unwrap_or_default();
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
    pub(super) fn append_assistant_chunk(&mut self, chunk: &str) {
        match self.messages.last_mut() {
            Some(ChatMessage::Assistant(s)) => s.push_str(chunk),
            _ => self.messages.push(ChatMessage::Assistant(chunk.to_string())),
        }
    }

    /// Remove the last message if it is an empty Assistant (e.g. before adding tool logs).
    pub(super) fn remove_last_if_empty_assistant(&mut self) {
        if self.messages.last().is_some_and(|m| matches!(m, ChatMessage::Assistant(s) if s.is_empty())) {
            self.messages.pop();
        }
    }

    /// Replace the last Assistant message with the given content, or push if none.
    pub(super) fn replace_or_push_assistant(&mut self, content: String) {
        if let Some(ChatMessage::Assistant(s)) = self.messages.last_mut() {
            *s = content;
        } else {
            self.messages.push(ChatMessage::Assistant(content));
        }
    }

    pub(super) fn push_tool_log(&mut self, line: String) {
        self.messages.push(ChatMessage::ToolLog(line));
    }

    pub(super) fn set_thinking(&mut self, thinking: bool) {
        if thinking {
            self.messages.push(ChatMessage::Thinking);
        } else {
            // Remove Thinking by value (may not be last if we streamed tool_log during thinking)
            self.messages.retain(|m| !matches!(m, ChatMessage::Thinking));
        }
    }

    /// Remove verbose progress (ToolLog) shown during thinking. Keeps history up to last User.
    pub(super) fn clear_progress_after_last_user(&mut self) {
        if let Some(last_user_idx) = self.messages.iter().rposition(|m| matches!(m, ChatMessage::User(_))) {
            self.messages.truncate(last_user_idx + 1);
        }
    }

    /// Append "[cancelled]" to the last assistant message (or create one).
    /// Keeps whatever partial content was already streamed.
    pub(super) fn append_cancelled_notice(&mut self) {
        // Remove trailing empty assistant and tool-log lines from streaming.
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

    /// Must be called before scroll_up/scroll_down when at bottom.
    pub(crate) fn materialize_scroll(&mut self) {
        if self.scroll == ScrollPosition::Bottom {
            self.scroll = ScrollPosition::Line(self.last_max_scroll);
        }
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.materialize_scroll();
        if let ScrollPosition::Line(pos) = self.scroll {
            self.scroll = ScrollPosition::Line((pos + n).min(self.last_max_scroll));
        }
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.materialize_scroll();
        if let ScrollPosition::Line(pos) = self.scroll {
            self.scroll = ScrollPosition::Line(pos.saturating_sub(n));
        }
    }

    /// Resolve scroll position to a concrete line index.
    pub(crate) fn scroll_line(&self) -> usize {
        match self.scroll {
            ScrollPosition::Line(n) => n.min(self.last_max_scroll),
            ScrollPosition::Bottom => self.last_max_scroll,
        }
    }
}
