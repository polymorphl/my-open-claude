//! Event handlers for the TUI: keyboard and mouse.

mod confirm;
mod input;
mod model_selector;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::layout::Position;
use std::sync::mpsc;
use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::llm;
use crate::core::models::ModelInfo;

use super::app::App;

const CREDITS_URL: &str = "https://openrouter.ai/settings/credits";

/// Holds receivers for a chat request in progress (progress logs, streamed content, final result).
pub struct PendingChat {
    pub progress_rx: mpsc::Receiver<String>,
    pub stream_rx: mpsc::Receiver<String>,
    pub result_rx: mpsc::Receiver<Result<llm::ChatResult, String>>,
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
    if app.confirm_popup.is_none() && app.model_selector.is_none() {
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
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return HandleResult::Break;
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
                let _ = crate::core::persistence::save_last_model(&model.id);
                app.model_selector = None;
                *pending_model_fetch = None;
            }
            model_selector::ModelSelectorAction::Keep => {}
        }
        return HandleResult::Continue;
    }

    // Alt+M: open model selector
    let open_model_selector = (key.code, key.modifiers) == (KeyCode::Char('m'), KeyModifiers::ALT)
        || key.code == KeyCode::Char('\u{00B5}') // µ = Option+M on Mac US keyboard
        || key.code == KeyCode::F(2); // F2 as fallback
    if open_model_selector {
        model_selector::open_model_selector(app, config, pending_model_fetch, rt);
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
