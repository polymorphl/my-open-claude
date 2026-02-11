//! Event handlers for the TUI: keyboard and mouse.

mod confirm;
mod history_selector;
mod input;
mod model_selector;

use crossterm::event::{KeyCode, KeyEventKind, MouseEventKind};
use ratatui::layout::Position;
use std::sync::mpsc;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::history::{self, first_message_preview};
use crate::core::llm;
use crate::core::models::ModelInfo;

use super::app::App;
use super::shortcuts::Shortcut;

const CREDITS_URL: &str = "https://openrouter.ai/settings/credits";

fn handle_shortcut(
    shortcut: Shortcut,
    _key: crossterm::event::KeyEvent,
    app: &mut App,
    config: &Arc<Config>,
    api_messages: &mut Option<Vec<Value>>,
    _pending_chat: &mut Option<PendingChat>,
    pending_model_fetch: &mut Option<mpsc::Receiver<Result<Vec<ModelInfo>, String>>>,
    rt: &Arc<Runtime>,
) -> HandleResult {
    match shortcut {
        Shortcut::History => {
            if app.is_dirty() {
                if let Some(msgs) = api_messages.as_ref() {
                    if !msgs.is_empty() {
                        let title = first_message_preview(msgs, 60);
                        if let Ok(id) =
                            history::save_conversation(app.conversation_id(), &title, msgs, config.as_ref())
                        {
                            app.set_conversation_id(Some(id));
                            app.clear_dirty();
                        }
                    }
                }
            }
            app.history_selector = Some(history_selector::open_history_selector());
        }
        Shortcut::NewConversation => {
            if app.is_dirty() {
                if let Some(msgs) = api_messages.as_ref() {
                    if !msgs.is_empty() {
                        let title = first_message_preview(msgs, 60);
                        let _ = history::save_conversation(
                            app.conversation_id(),
                            &title,
                            msgs,
                            config.as_ref(),
                        );
                    }
                }
            }
            app.new_conversation();
            *api_messages = None;
        }
        Shortcut::ModelSelector => {
            model_selector::open_model_selector(app, config, pending_model_fetch, rt);
        }
        Shortcut::Quit => {
            return HandleResult::Break;
        }
        Shortcut::None => {}
    }
    HandleResult::Continue
}

/// Holds receivers for a chat request in progress (progress logs, streamed content, final result).
pub struct PendingChat {
    pub progress_rx: mpsc::Receiver<String>,
    pub stream_rx: mpsc::Receiver<String>,
    pub result_rx: mpsc::Receiver<Result<llm::ChatResult, String>>,
    /// Token to cancel the in-flight request.
    pub cancel_token: CancellationToken,
}

/// Result of handling an event: continue the loop or exit.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HandleResult {
    Continue,
    Break,
}

/// Set cursor to pointer (hand) or default. Uses OSC 22 (Kitty, iTerm2, Ghostty, Foot).
pub fn set_cursor_shape(pointer: bool) {
    use std::io::Write;
    let seq = if pointer {
        b"\x1b]22;pointer\x07"
    } else {
        b"\x1b]22;default\x07"
    };
    let _ = std::io::stdout().write_all(seq);
    let _ = std::io::stdout().flush();
}

/// Handle a mouse event.
pub fn handle_mouse(mouse: crossterm::event::MouseEvent, app: &mut App) -> HandleResult {
    let pos = Position::new(mouse.column.saturating_sub(1), mouse.row.saturating_sub(1));
    let over_credits = app
        .credits_header_rect
        .is_some_and(|rect| rect.contains(pos));
    if app.confirm_popup.is_none() && app.model_selector.is_none() && app.history_selector.is_none() {
        match mouse.kind {
            MouseEventKind::Moved => {
                if app.hovering_credits != over_credits {
                    app.hovering_credits = over_credits;
                    set_cursor_shape(over_credits);
                }
            }
            MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                if over_credits {
                    let _ = opener::open(CREDITS_URL);
                }
            }
            MouseEventKind::ScrollUp => {
                app.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                app.scroll_down(3);
            }
            _ => {}
        }
    }
    HandleResult::Continue
}

/// Handle a key event. Returns HandleResult::Break to exit the main loop.
pub fn handle_key(
    key: crossterm::event::KeyEvent,
    app: &mut App,
    config: &Arc<Config>,
    api_messages: &mut Option<Vec<Value>>,
    pending_chat: &mut Option<PendingChat>,
    pending_model_fetch: &mut Option<mpsc::Receiver<Result<Vec<ModelInfo>, String>>>,
    rt: &Arc<Runtime>,
) -> HandleResult {
    if key.kind != KeyEventKind::Press {
        return HandleResult::Continue;
    }

    // Esc+key sequence (Option as Meta on Mac terminals)
    if app.escape_pending {
        if let Some(shortcut) = Shortcut::match_key(&key, true) {
            app.escape_pending = false;
            return handle_shortcut(shortcut, key, app, config, api_messages, pending_chat, pending_model_fetch, rt);
        }
        app.escape_pending = false;
    }

    if let Some(shortcut) = Shortcut::match_key(&key, false) {
        if shortcut == Shortcut::Quit {
            return HandleResult::Break;
        }
        // Don't trigger NewConversation on `~` when user is typing (e.g. ~/path)
        if shortcut == Shortcut::NewConversation
            && key.code == KeyCode::Char('~')
            && !app.input.is_empty()
        {
            // Fall through to input handler
        } else if shortcut != Shortcut::None {
            return handle_shortcut(shortcut, key, app, config, api_messages, pending_chat, pending_model_fetch, rt);
        }
    }

    // Esc: cancel in-flight request if one is pending, otherwise start Option+key sequence.
    if Shortcut::is_escape(&key)
        && app.confirm_popup.is_none()
        && app.model_selector.is_none()
        && app.history_selector.is_none()
    {
        if pending_chat.is_some() {
            pending_chat.as_ref().unwrap().cancel_token.cancel();
            return HandleResult::Continue;
        }
        app.escape_pending = true;
        return HandleResult::Continue;
    }

    // Confirm popup (y/n for destructive command)
    if let Some(popup) = app.confirm_popup.take() {
        match confirm::handle_confirm_popup(
            key.code,
            popup,
            app,
            config,
            pending_chat.is_none(),
            rt,
        ) {
            confirm::ConfirmPopupResult::PutBack(p) => app.confirm_popup = Some(p),
            confirm::ConfirmPopupResult::Spawned(pc) => *pending_chat = Some(pc),
        }
        return HandleResult::Continue;
    }

    // Popups (confirm, history, model) - handled before general shortcuts
    // History selector popup
    if let Some(ref mut selector) = app.history_selector {
        let action = history_selector::handle_history_selector_key(
            key.code,
            key.modifiers,
            selector,
        );
        match action {
            history_selector::HistorySelectorAction::Close => {
                app.history_selector = None;
            }
            history_selector::HistorySelectorAction::Load { id } => {
                if let Some(messages) = history::load_conversation(&id) {
                    app.set_messages_from_api(&messages);
                    app.set_conversation_id(Some(id.clone()));
                    app.scroll = super::app::ScrollPosition::Bottom;
                    // Estimate token usage from loaded messages so the header shows it immediately.
                    app.token_usage = Some(llm::TokenUsage::estimated_from_messages(&messages));
                    *api_messages = Some(messages);
                }
                app.history_selector = None;
            }
            history_selector::HistorySelectorAction::Delete { id } => {
                let _ = history::delete_conversation(&id);
                selector.conversations.retain(|c| c.id != id);
                let filtered =
                    history::filter_conversations(&selector.conversations, &selector.filter);
                selector.selected_index = selector
                    .selected_index
                    .min(filtered.len().saturating_sub(1));
            }
            history_selector::HistorySelectorAction::Rename { id, new_title } => {
                if let Ok(()) = history::rename_conversation(&id, &new_title) {
                    if let Some(meta) = selector.conversations.iter_mut().find(|c| c.id == id) {
                        meta.title = new_title;
                    }
                }
            }
            history_selector::HistorySelectorAction::Keep => {}
        }
        return HandleResult::Continue;
    }

    // Model selector popup
    if let Some(ref mut selector) = app.model_selector {
        let action = model_selector::handle_model_selector_key(
            key.code,
            key.modifiers,
            selector,
        );
        match action {
            model_selector::ModelSelectorAction::Close => {
                app.model_selector = None;
                *pending_model_fetch = None;
            }
            model_selector::ModelSelectorAction::Select(model) => {
                app.current_model_id = model.id.clone();
                app.model_name = model.name.clone();
                app.context_length = model.context_length;
                app.token_usage = None;
                let _ = crate::core::persistence::save_last_model(&model.id);
                app.model_selector = None;
                *pending_model_fetch = None;
            }
            model_selector::ModelSelectorAction::Keep => {}
        }
        return HandleResult::Continue;
    }

    // Main input handling
    input::handle_main_input(
        key.code,
        key.modifiers,
        app,
        config,
        pending_chat,
        api_messages,
        rt,
    )
}
