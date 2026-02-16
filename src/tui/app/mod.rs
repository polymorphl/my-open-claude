//! TUI application state: messages, input, scroll, suggestions.

mod messages;

use crate::core::history::ConversationMeta;
use crate::core::llm::{ConfirmState, TokenUsage};
use crate::core::models::ModelInfo;
use crate::core::workspace::Workspace;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::time::Instant;

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
    /// When the model fetch started; used for loading spinner animation.
    pub(crate) fetch_started_at: Option<Instant>,
}

/// State for the history selector popup (Alt+H).
pub struct HistorySelectorState {
    pub conversations: Vec<ConversationMeta>,
    pub selected_index: usize,
    pub list_state: ListState,
    pub filter: String,
    /// When renaming: (conversation_id, new_title_input).
    pub renaming: Option<(String, String)>,
    /// Error loading conversations or from delete/rename.
    pub error: Option<String>,
    /// Conversation ID -> concatenated message content for full-text search.
    pub content_cache: HashMap<String, String>,
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
    /// Cursor position in the input (byte index; used for Left/Right, insert, Backspace).
    pub(crate) input_cursor: usize,
    pub(crate) scroll: ScrollPosition,
    pub(crate) last_max_scroll: usize,
    /// Index of the selected suggestion (Tab to cycle).
    pub selected_suggestion: usize,
    /// Index of the selected slash command in the autocomplete list (when input starts with /).
    pub selected_command_index: usize,
    /// Mode to use when sending; set when user selects a slash command and inserts its template.
    pub(crate) pending_command_mode: Option<String>,
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
    /// Credits fetch error (e.g. network) to display on welcome screen.
    pub(crate) credits_fetch_error: Option<String>,
    /// Mouse is over credits area; used for cursor style.
    pub(crate) hovering_credits: bool,
    /// (msg_idx, start_line, end_line) for each User/Assistant; updated each draw.
    pub(crate) message_line_ranges: Vec<(usize, usize, usize)>,
    /// Unix timestamps (seconds) for each message; parallel to messages. None when loading from history.
    pub(crate) message_timestamps: Vec<Option<u64>>,
    /// Whether to show timestamps next to message labels (from MY_OPEN_CLAUDE_SHOW_TIMESTAMPS).
    pub(crate) show_timestamps: bool,
    /// Rect of history text area; for click hit testing.
    pub(crate) history_area_rect: Option<Rect>,
    /// Mouse is over a message block; used for cursor style.
    pub(crate) hovering_message_block: bool,
    /// When set, show "Copied!" toast until this instant.
    pub(crate) copy_toast_until: Option<Instant>,
    /// When set, show "Save failed" toast until this instant.
    pub(crate) save_error_toast_until: Option<Instant>,
    /// Current conversation ID; None = new unsaved conversation.
    pub(crate) current_conversation_id: Option<String>,
    /// True if content has changed since last save.
    pub(crate) dirty: bool,
    /// Esc was pressed; next key = Option+key (Mac terminals with "Use option as meta").
    pub(crate) escape_pending: bool,
    /// True while a chat request is in flight (used by bottom bar to show cancel hint).
    pub(crate) is_streaming: bool,
    /// Last known token usage from the API (updated after each chat completion).
    pub(crate) token_usage: Option<TokenUsage>,
    /// Context window size (in tokens) for the current model.
    pub(crate) context_length: u64,
    /// Workspace (root, project type, AGENT.md) detected at startup.
    pub workspace: Workspace,
}

impl App {
    pub fn new(
        model_id: String,
        model_name: String,
        workspace: Workspace,
        show_timestamps: bool,
    ) -> Self {
        let context_length = crate::core::models::resolve_context_length(&model_id);
        Self {
            messages: vec![],
            input: String::new(),
            input_cursor: 0,
            scroll: ScrollPosition::default(),
            last_max_scroll: 0,
            selected_suggestion: 0,
            selected_command_index: 0,
            pending_command_mode: None,
            confirm_popup: None,
            model_name,
            current_model_id: model_id,
            model_selector: None,
            history_selector: None,
            last_content_width: None,
            credit_data: None,
            credits_header_rect: None,
            credits_last_fetched_at: None,
            credits_fetch_error: None,
            hovering_credits: false,
            message_line_ranges: vec![],
            message_timestamps: vec![],
            show_timestamps,
            history_area_rect: None,
            hovering_message_block: false,
            copy_toast_until: None,
            save_error_toast_until: None,
            current_conversation_id: None,
            dirty: false,
            escape_pending: false,
            is_streaming: false,
            token_usage: None,
            context_length,
            workspace,
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

    pub(crate) fn set_save_error_toast(&mut self, until: Instant) {
        self.save_error_toast_until = Some(until);
    }

    /// Reset to a new empty conversation.
    pub(crate) fn new_conversation(&mut self) {
        self.messages.clear();
        self.message_timestamps.clear();
        self.current_conversation_id = None;
        self.dirty = false;
        self.scroll = ScrollPosition::default();
        self.last_max_scroll = 0;
        self.token_usage = None;
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
