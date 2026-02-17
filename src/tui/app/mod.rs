//! TUI application state: messages, input, scroll, suggestions.

mod messages;

use crate::core::commands::ResolvedCommand;
use crate::core::history::ConversationMeta;
use crate::core::llm::{ConfirmState, TokenUsage};
use crate::core::models::ModelInfo;
use crate::core::templates::CustomTemplate;
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

/// Which field is focused in the command form.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandFormField {
    Name,
    Description,
    Prompt,
    Mode,
}

/// Phase of the command form popup.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandFormPhase {
    SelectCommand,
    EditForm,
}

/// State for create/update command popup.
pub struct CommandFormState {
    pub form_mode: CommandFormMode,
    pub name: String,
    pub description: String,
    pub prompt_prefix: String,
    pub llm_mode: String,
    pub focused_field: CommandFormField,
    pub error: Option<String>,
    pub phase: CommandFormPhase,
    pub selected_index: usize,
}

#[derive(Clone)]
pub enum CommandFormMode {
    Create,
    Update {
        /// Original name (for validation: exclude from conflict check).
        original_name: Option<String>,
    },
}

/// State for delete command popup.
pub struct DeleteCommandState {
    pub selected_index: usize,
    pub selected: Vec<bool>,
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
    /// Merged built-in + custom commands for slash autocomplete.
    pub resolved_commands: Vec<ResolvedCommand>,
    /// Custom templates (mutable for create/update/delete).
    pub custom_templates: Vec<CustomTemplate>,
    /// Error loading templates.json (shown as toast/welcome message).
    pub templates_load_error: Option<String>,
    /// Create/update command form popup.
    pub command_form_popup: Option<CommandFormState>,
    /// Delete command popup.
    pub delete_command_popup: Option<DeleteCommandState>,
}

impl App {
    pub fn new(
        model_id: String,
        model_name: String,
        workspace: Workspace,
        show_timestamps: bool,
    ) -> Self {
        let context_length = crate::core::models::resolve_context_length(&model_id);

        let (resolved_commands, custom_templates, templates_load_error) =
            match crate::core::templates::load_templates(crate::core::commands::BUILTIN_NAMES) {
                Ok(custom) => {
                    let custom_clone = custom.clone();
                    match crate::core::commands::resolve_commands(custom) {
                        Ok(resolved) => (resolved, custom_clone, None),
                        Err(e) => (vec![], vec![], Some(e.to_string())),
                    }
                }
                Err(e) => (
                    crate::core::commands::resolve_commands(vec![]).unwrap_or_default(),
                    vec![],
                    Some(e.to_string()),
                ),
            };

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
            resolved_commands,
            custom_templates,
            templates_load_error,
            command_form_popup: None,
            delete_command_popup: None,
        }
    }

    pub(crate) fn open_create_command_popup(&mut self) {
        self.command_form_popup = Some(CommandFormState {
            form_mode: CommandFormMode::Create,
            name: String::new(),
            description: String::new(),
            prompt_prefix: String::new(),
            llm_mode: "Build".to_string(),
            focused_field: CommandFormField::Name,
            error: None,
            phase: CommandFormPhase::EditForm,
            selected_index: 0,
        });
    }

    pub(crate) fn open_update_command_popup(&mut self) {
        if self.custom_templates.is_empty() {
            return;
        }
        self.command_form_popup = Some(CommandFormState {
            form_mode: CommandFormMode::Update {
                original_name: None,
            },
            name: String::new(),
            description: String::new(),
            prompt_prefix: String::new(),
            llm_mode: "Build".to_string(),
            focused_field: CommandFormField::Name,
            error: None,
            phase: CommandFormPhase::SelectCommand,
            selected_index: 0,
        });
    }

    pub(crate) fn open_delete_command_popup(&mut self) {
        if self.custom_templates.is_empty() {
            return;
        }
        self.delete_command_popup = Some(DeleteCommandState {
            selected_index: 0,
            selected: vec![false; self.custom_templates.len()],
        });
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

    /// Recompute resolved_commands from custom_templates (after create/update/delete).
    pub(crate) fn reload_resolved_commands(&mut self) {
        if let Ok(resolved) = crate::core::commands::resolve_commands(self.custom_templates.clone())
        {
            self.resolved_commands = resolved;
        }
    }
}
