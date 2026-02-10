//! TUI application state: messages, input, scroll, suggestions.

use crate::core::llm::ConfirmState;
use crate::core::models::ModelInfo;
use ratatui::widgets::ListState;

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
    pub(super) messages: Vec<ChatMessage>,
    pub(super) input: String,
    pub(super) scroll: ScrollPosition,
    pub(super) last_max_scroll: usize,
    /// Index of the selected suggestion (Tab to cycle).
    pub(super) selected_suggestion: usize,
    /// When set, show confirmation popup and ignore normal input until y/n.
    pub(super) confirm_popup: Option<ConfirmPopup>,
    /// Model ID displayed in the header and used for chat (e.g. "anthropic/claude-haiku-4.5").
    pub(super) model_name: String,
    /// Same as model_name; used for API calls.
    pub(super) current_model_id: String,
    /// When set, show model selector popup (Alt+M).
    pub(super) model_selector: Option<ModelSelectorState>,
    /// Content width from last draw; used to compute scroll-to-start when adding new messages.
    pub(super) last_content_width: Option<usize>,
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

    /// Must be called before scroll_up/scroll_down when at bottom.
    pub(super) fn materialize_scroll(&mut self) {
        if self.scroll == ScrollPosition::Bottom {
            self.scroll = ScrollPosition::Line(self.last_max_scroll);
        }
    }

    pub(super) fn scroll_to_bottom(&mut self) {
        self.scroll = ScrollPosition::Bottom;
    }

    pub(super) fn scroll_down(&mut self, n: usize) {
        self.materialize_scroll();
        if let ScrollPosition::Line(pos) = self.scroll {
            self.scroll = ScrollPosition::Line((pos + n).min(self.last_max_scroll));
        }
    }

    pub(super) fn scroll_up(&mut self, n: usize) {
        self.materialize_scroll();
        if let ScrollPosition::Line(pos) = self.scroll {
            self.scroll = ScrollPosition::Line(pos.saturating_sub(n));
        }
    }

    /// Resolve scroll position to a concrete line index.
    pub(super) fn scroll_line(&self) -> usize {
        match self.scroll {
            ScrollPosition::Line(n) => n.min(self.last_max_scroll),
            ScrollPosition::Bottom => self.last_max_scroll,
        }
    }
}
