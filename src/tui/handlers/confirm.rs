//! Handler for confirm popup (y/n for destructive command).

use crossterm::event::KeyCode;
use std::sync::Arc;

use tokio::runtime::Runtime;

use crate::core::config::Config;

use super::super::app::{App, ConfirmPopup, ScrollPosition};
use super::chat_spawn;
use super::PendingChat;

/// Result of handling a key in the confirm popup.
pub(crate) enum ConfirmPopupResult {
    /// Put the popup back (user pressed something other than y/n/enter, or pending_chat already set).
    PutBack(ConfirmPopup),
    /// Spawned chat resume; caller should set pending_chat.
    Spawned(PendingChat),
}

/// Handle key when confirm popup is showing.
pub(crate) fn handle_confirm_popup(
    key_code: KeyCode,
    popup: ConfirmPopup,
    app: &mut App,
    config: &Arc<Config>,
    pending_chat_is_none: bool,
    rt: &Arc<Runtime>,
) -> ConfirmPopupResult {
    let confirmed = matches!(key_code, KeyCode::Char('y') | KeyCode::Char('Y'));
    let cancelled = matches!(
        key_code,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter
    );

    if confirmed || cancelled {
        if pending_chat_is_none {
            app.push_assistant(String::new());
            app.scroll = ScrollPosition::Bottom;
            let model_id = app.current_model_id.clone();
            let pc = chat_spawn::spawn_chat_resume(
                rt,
                Arc::clone(config),
                model_id,
                popup.state,
                confirmed,
            );
            ConfirmPopupResult::Spawned(pc)
        } else {
            // Can't process yet; put popup back
            ConfirmPopupResult::PutBack(popup)
        }
    } else {
        ConfirmPopupResult::PutBack(popup)
    }
}
