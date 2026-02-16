//! Shortcut key handling (History, NewConversation, ModelSelector, Quit).

use std::sync::Arc;
use std::sync::mpsc;

use tokio::runtime::Runtime;

use crate::core::history::{self, first_message_preview};
use crate::core::models::ModelInfo;
use crate::tui::shortcuts::Shortcut;

use super::super::app::App;
use super::super::constants;
use super::HandleResult;
use super::history_selector;
use super::model_selector;

/// Context passed to shortcut handlers (reduces parameter count).
pub(super) struct ShortcutContext<'a> {
    pub app: &'a mut App,
    pub config: &'a Arc<crate::core::config::Config>,
    pub api_messages: &'a mut Option<Vec<serde_json::Value>>,
    pub pending_chat: &'a mut Option<super::PendingChat>,
    pub pending_model_fetch: &'a mut Option<mpsc::Receiver<Result<Vec<ModelInfo>, String>>>,
    pub rt: &'a Arc<Runtime>,
}

pub(super) fn handle_shortcut(shortcut: Shortcut, ctx: ShortcutContext<'_>) -> HandleResult {
    match shortcut {
        Shortcut::History => {
            if ctx.app.is_dirty() {
                let to_save =
                    App::messages_to_persist_format(&ctx.app.messages, &ctx.app.message_timestamps);
                if !to_save.is_empty() {
                    let title = first_message_preview(&to_save, constants::TITLE_PREVIEW_MAX_LEN);
                    if let Ok(id) = history::save_conversation(
                        ctx.app.conversation_id(),
                        &title,
                        &to_save,
                        ctx.config.as_ref(),
                    ) {
                        ctx.app.set_conversation_id(Some(id));
                        ctx.app.clear_dirty();
                    }
                }
            }
            ctx.app.history_selector = Some(history_selector::open_history_selector());
        }
        Shortcut::NewConversation => {
            if let Some(pc) = ctx.pending_chat.as_ref() {
                pc.cancel_token.cancel();
            }
            *ctx.pending_chat = None;
            ctx.app.is_streaming = false;
            if ctx.app.is_dirty() {
                let to_save =
                    App::messages_to_persist_format(&ctx.app.messages, &ctx.app.message_timestamps);
                if !to_save.is_empty() {
                    let title = first_message_preview(&to_save, constants::TITLE_PREVIEW_MAX_LEN);
                    let _ = history::save_conversation(
                        ctx.app.conversation_id(),
                        &title,
                        &to_save,
                        ctx.config.as_ref(),
                    );
                }
            }
            ctx.app.new_conversation();
            *ctx.api_messages = None;
        }
        Shortcut::ModelSelector => {
            model_selector::open_model_selector(
                ctx.app,
                ctx.config,
                ctx.pending_model_fetch,
                ctx.rt,
            );
        }
        Shortcut::Quit => {
            return HandleResult::Break;
        }
        Shortcut::None => {}
    }
    HandleResult::Continue
}
