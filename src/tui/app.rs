//! TUI application state: messages, input, scroll, suggestions.

use crate::core::llm::ConfirmState;

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

pub struct App {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) input: String,
    pub(super) scroll: usize,
    pub(super) last_max_scroll: usize,
    /// Index of the selected suggestion (Tab to cycle).
    pub(super) selected_suggestion: usize,
    /// When set, show confirmation popup and ignore normal input until y/n.
    pub(super) confirm_popup: Option<ConfirmPopup>,
    /// Model ID displayed in the header (e.g. "anthropic/claude-haiku-4.5").
    pub(super) model_name: String,
    /// Content width from last draw; used to compute scroll-to-start when adding new messages.
    pub(super) last_content_width: Option<usize>,
}

impl App {
    pub fn new(model_name: String) -> Self {
        Self {
            messages: vec![],
            input: String::new(),
            scroll: 0,
            last_max_scroll: 0,
            selected_suggestion: 0,
            confirm_popup: None,
            model_name,
            last_content_width: None,
        }
    }

    pub(super) fn push_user(&mut self, text: &str) {
        self.messages.push(ChatMessage::User(text.to_string()));
    }

    pub(super) fn push_assistant(&mut self, text: String) {
        self.messages.push(ChatMessage::Assistant(text));
    }

    pub(super) fn push_tool_log(&mut self, line: String) {
        self.messages.push(ChatMessage::ToolLog(line));
    }

    pub(super) fn set_thinking(&mut self, thinking: bool) {
        if thinking {
            self.messages.push(ChatMessage::Thinking);
        } else {
            self.messages.pop(); // remove "Thinking"
        }
    }

    /// Must be called before scroll_up/scroll_down when at bottom (scroll == usize::MAX).
    pub(super) fn materialize_scroll(&mut self) {
        if self.scroll == usize::MAX {
            self.scroll = self.last_max_scroll;
        }
    }

    pub(super) fn scroll_down(&mut self, n: usize) {
        self.materialize_scroll();
        self.scroll = (self.scroll + n).min(self.last_max_scroll);
    }

    pub(super) fn scroll_up(&mut self, n: usize) {
        self.materialize_scroll();
        self.scroll = self.scroll.saturating_sub(n);
    }
}
