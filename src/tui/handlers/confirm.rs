//! Handler for confirm popup (y/n for destructive command).

use crossterm::event::KeyCode;
use std::sync::mpsc;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::llm;
use crate::core::models;

use super::super::app::{App, ConfirmPopup, ScrollPosition};
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
            let (progress_tx, progress_rx) = mpsc::channel();
            let (stream_tx, stream_rx) = mpsc::channel();
            let (result_tx, result_rx) = mpsc::channel();
            let cancel_token = CancellationToken::new();
            let cancel_token_clone = cancel_token.clone();
            let config = Arc::clone(config);
            let model_id = app.current_model_id.clone();
            let context_length = models::resolve_context_length(&model_id);
            let rt_clone = Arc::clone(rt);
            std::thread::spawn(move || {
                let on_progress: llm::OnProgress = Box::new(move |s| {
                    let _ = progress_tx.send(s.to_string());
                });
                let on_content_chunk: llm::OnContentChunk = Box::new(move |s| {
                    let _ = stream_tx.send(s.to_string());
                });
                let result = rt_clone.block_on(llm::chat_resume(
                    config.as_ref(),
                    &model_id,
                    context_length,
                    popup.state,
                    confirmed,
                    Some(on_progress),
                    Some(on_content_chunk),
                    Some(cancel_token_clone),
                ));
                let _ = result_tx.send(result.map_err(|e| e.to_string()));
            });
            ConfirmPopupResult::Spawned(PendingChat {
                progress_rx,
                stream_rx,
                result_rx,
                cancel_token,
            })
        } else {
            // Can't process yet; put popup back
            ConfirmPopupResult::PutBack(popup)
        }
    } else {
        ConfirmPopupResult::PutBack(popup)
    }
}
