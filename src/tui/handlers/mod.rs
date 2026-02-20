//! Event handlers for the TUI: keyboard and mouse.

mod chat_spawn;
mod command_form;
mod confirm;
mod delete_command;
mod history_selector;
mod input;
mod model_selector;
mod popups;
mod selection;
mod shortcuts;

use crossterm::event::{KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Position;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::llm;
use crate::core::models::ModelInfo;

use super::app::{App, CopyTarget};
use super::constants;
use super::shortcuts::Shortcut;

use self::shortcuts::{ShortcutContext, handle_shortcut};

const CREDITS_URL: &str = "https://openrouter.ai/settings/credits";

/// Holds receivers for a chat request in progress (progress logs, streamed content, final result).
pub struct PendingChat {
    pub progress_rx: mpsc::Receiver<String>,
    pub stream_rx: mpsc::Receiver<String>,
    pub result_rx: mpsc::Receiver<Result<llm::ChatResult, llm::ChatError>>,
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

/// True when Esc would start Option+key (escape_pending) rather than cancel/slash-clear.
pub(crate) fn would_esc_start_meta_sequence(
    key: &crossterm::event::KeyEvent,
    app: &App,
    pending_chat: &Option<PendingChat>,
) -> bool {
    Shortcut::is_escape(key)
        && app.confirm_popup.is_none()
        && app.model_selector.is_none()
        && app.history_selector.is_none()
        && app.command_form_popup.is_none()
        && app.delete_command_popup.is_none()
        && !app.input.starts_with('/')
        && pending_chat.is_none()
}

/// Handle a mouse event.
pub fn handle_mouse(mouse: crossterm::event::MouseEvent, app: &mut App) -> HandleResult {
    let pos = Position::new(mouse.column.saturating_sub(1), mouse.row.saturating_sub(1));
    let over_credits = app
        .credits_header_rect
        .is_some_and(|rect| rect.contains(pos));
    let over_message = selection::hit_test_message(app, pos);
    let buffer_coords = selection::pos_to_buffer_coords(app, pos);

    if app.confirm_popup.is_none()
        && app.model_selector.is_none()
        && app.history_selector.is_none()
        && app.command_form_popup.is_none()
        && app.delete_command_popup.is_none()
    {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if over_credits {
                    // Click on credits handled on Up
                } else if let Some((line, col)) = buffer_coords {
                    app.selection_drag_start = Some((line, col));
                    app.selection = Some((line, col, line, col));
                } else {
                    app.selection_drag_start = None;
                    app.selection = None;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let (Some((start_line, start_col)), Some((line, col))) =
                    (app.selection_drag_start, buffer_coords)
                {
                    let (sl, sc, el, ec) = if (line, col) < (start_line, start_col) {
                        (line, col, start_line, start_col)
                    } else {
                        (start_line, start_col, line, col)
                    };
                    app.selection = Some((sl, sc, el, ec));
                }
                let pointer =
                    over_credits || over_message.is_some() || app.selection_drag_start.is_some();
                let prev_pointer = app.hovering_credits || app.hovering_message_block;
                app.hovered_message_idx = over_message;
                if prev_pointer != pointer {
                    app.hovering_credits = over_credits;
                    app.hovering_message_block = over_message.is_some();
                    set_cursor_shape(pointer);
                }
            }
            MouseEventKind::Moved => {
                let pointer =
                    over_credits || over_message.is_some() || app.selection_drag_start.is_some();
                let prev_pointer = app.hovering_credits || app.hovering_message_block;
                app.hovered_message_idx = over_message;
                if prev_pointer != pointer {
                    app.hovering_credits = over_credits;
                    app.hovering_message_block = over_message.is_some();
                    set_cursor_shape(pointer);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if over_credits {
                    let _ = opener::open(CREDITS_URL);
                } else if let Some(drag_start) = app.selection_drag_start.take() {
                    let is_click = app
                        .selection
                        .map(|(sl, sc, el, ec)| (sl, sc) == (el, ec) || drag_start == (sl, sc))
                        .unwrap_or(true);
                    if is_click {
                        if let Some(target) = selection::hit_test_copy_region(app, pos) {
                            let content = match &target {
                                CopyTarget::Message(idx) => {
                                    app.messages.get(*idx).and_then(|m| match m {
                                        super::app::ChatMessage::User(s)
                                        | super::app::ChatMessage::Assistant(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                }
                                CopyTarget::Code(code) => Some(code.clone()),
                            };
                            if let Some(content) = content
                                && arboard::Clipboard::new()
                                    .and_then(|mut c| c.set_text(content))
                                    .is_ok()
                            {
                                app.copy_toast_until =
                                    Some(Instant::now() + Duration::from_secs(2));
                            }
                        }
                        app.selection = None;
                    }
                } else if !over_credits && buffer_coords.is_none() {
                    app.selection = None;
                }
            }
            MouseEventKind::ScrollUp => {
                app.selection = None;
                app.selection_drag_start = None;
                app.scroll_up(constants::SCROLL_LINES_SMALL);
            }
            MouseEventKind::ScrollDown => {
                app.selection = None;
                app.selection_drag_start = None;
                app.scroll_down(constants::SCROLL_LINES_SMALL);
            }
            _ => {}
        }
    }
    HandleResult::Continue
}

/// Context for key event handling. Bundles mutable state to reduce parameter count.
pub struct HandleKeyContext<'a> {
    pub app: &'a mut App,
    pub config: &'a Arc<Config>,
    pub api_messages: &'a mut Option<Vec<Value>>,
    pub pending_chat: &'a mut Option<PendingChat>,
    pub pending_model_fetch: &'a mut Option<mpsc::Receiver<Result<Vec<ModelInfo>, String>>>,
    pub rt: &'a Arc<Runtime>,
}

/// Handle a key event. Returns HandleResult::Break to exit the main loop.
pub fn handle_key(key: crossterm::event::KeyEvent, ctx: HandleKeyContext<'_>) -> HandleResult {
    let HandleKeyContext {
        app,
        config,
        api_messages,
        pending_chat,
        pending_model_fetch,
        rt,
    } = ctx;

    if key.kind != KeyEventKind::Press {
        return HandleResult::Continue;
    }

    // Esc+key sequence (Option as Meta on Mac terminals)
    if app.escape_pending {
        if let Some(shortcut) = Shortcut::match_key(&key, true) {
            app.escape_pending = false;
            return handle_shortcut(
                shortcut,
                ShortcutContext {
                    app,
                    config,
                    api_messages,
                    pending_chat,
                    pending_model_fetch,
                    rt,
                },
            );
        }
        app.escape_pending = false;
    }

    if let Some(shortcut) = Shortcut::match_key(&key, false) {
        if shortcut == Shortcut::Quit {
            return HandleResult::Break;
        }
        if shortcut != Shortcut::None {
            return handle_shortcut(
                shortcut,
                ShortcutContext {
                    app,
                    config,
                    api_messages,
                    pending_chat,
                    pending_model_fetch,
                    rt,
                },
            );
        }
    }

    // Cmd+C (SUPER on macOS): copy selection, else message under cursor or at scroll position.
    if key.code == crossterm::event::KeyCode::Char('c')
        && key.modifiers.contains(KeyModifiers::SUPER)
        && app.confirm_popup.is_none()
        && app.model_selector.is_none()
        && app.history_selector.is_none()
        && app.command_form_popup.is_none()
        && app.delete_command_popup.is_none()
    {
        if selection::try_copy_selection(app) {
            // Selection copied
        } else if let Some(msg_idx) = app
            .hovered_message_idx
            .or_else(|| selection::message_idx_at_scroll_line(app))
        {
            selection::try_copy_message(app, msg_idx);
        }
        return HandleResult::Continue;
    }

    // Esc: in slash mode, clear input; else cancel in-flight or start Option+key sequence.
    if Shortcut::is_escape(&key)
        && app.confirm_popup.is_none()
        && app.model_selector.is_none()
        && app.history_selector.is_none()
        && app.command_form_popup.is_none()
        && app.delete_command_popup.is_none()
    {
        if app.input.starts_with('/') {
            app.input.clear();
            app.input_cursor = 0;
            app.selected_command_index = 0;
            return HandleResult::Continue;
        }
        if let Some(pc) = pending_chat.as_ref() {
            pc.cancel_token.cancel();
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
    if app.history_selector.is_some() {
        return popups::handle_history_selector(key.code, key.modifiers, app, api_messages);
    }

    // Model selector popup
    if app.model_selector.is_some() {
        return popups::handle_model_selector(key.code, key.modifiers, app, pending_model_fetch);
    }

    // Command form popup (create/update)
    if app.command_form_popup.is_some() {
        return command_form::handle_command_form_popup(key.code, key.modifiers, app);
    }

    // Delete command popup
    if app.delete_command_popup.is_some() {
        return delete_command::handle_delete_command_popup(key.code, key.modifiers, app);
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
