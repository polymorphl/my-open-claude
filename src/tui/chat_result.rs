//! Handling of chat completion results and conversation save.

use std::time::{Duration, Instant};

use serde_json::Value;

use crate::core::config::Config;
use crate::core::history::{self, first_message_preview};
use crate::core::llm;

use super::app;
use super::constants;

const SAVE_ERROR_TOAST_DURATION: Duration = Duration::from_secs(4);

/// Save the current conversation if it has unsaved changes.
/// Logs and surfaces save errors via a toast.
pub(super) fn save_conversation_if_dirty(
    app: &mut app::App,
    api_messages: &Option<Vec<Value>>,
    config: &Config,
) {
    if !app.is_dirty() {
        return;
    }
    let Some(msgs) = api_messages else { return };
    if msgs.is_empty() {
        return;
    }
    let title = first_message_preview(msgs, constants::TITLE_PREVIEW_MAX_LEN);
    match history::save_conversation(app.conversation_id(), &title, msgs, config) {
        Ok(id) => {
            app.set_conversation_id(Some(id));
            app.clear_dirty();
        }
        Err(e) => {
            log::warn!("Failed to save conversation: {}", e);
            app.set_save_error_toast(Instant::now() + SAVE_ERROR_TOAST_DURATION);
        }
    }
}

/// Process a chat result: update app state, show confirmation popup, or display error.
pub(super) fn handle_chat_result(
    app: &mut app::App,
    api_messages: &mut Option<Vec<Value>>,
    result: Result<llm::ChatResult, llm::ChatError>,
    tool_log_already_streamed: bool,
    config: &Config,
) {
    match result {
        Ok(llm::ChatResult::Complete {
            content,
            tool_log,
            messages,
            usage,
        }) => {
            app.token_usage = Some(usage);
            if tool_log_already_streamed {
                app.clear_progress_after_last_user();
            } else {
                for line in tool_log {
                    app.push_tool_log(line);
                }
            }
            app.replace_or_push_assistant(content);
            app.scroll = app::ScrollPosition::Bottom;
            let title = first_message_preview(&messages, constants::TITLE_PREVIEW_MAX_LEN);
            match history::save_conversation(app.conversation_id(), &title, &messages, config) {
                Ok(id) => {
                    app.set_conversation_id(Some(id));
                    app.clear_dirty();
                }
                Err(e) => {
                    log::warn!("Failed to save conversation: {}", e);
                    app.set_save_error_toast(std::time::Instant::now() + SAVE_ERROR_TOAST_DURATION);
                }
            }
            *api_messages = Some(messages);
        }
        Ok(llm::ChatResult::NeedsConfirmation { command, state }) => {
            app.confirm_popup = Some(app::ConfirmPopup { command, state });
        }
        Err(llm::ChatError::Cancelled) => {
            app.append_cancelled_notice();
        }
        Err(ref e) => {
            app.replace_or_push_assistant(format!("Error: {}", e));
            app.scroll = app::ScrollPosition::Bottom;
        }
    }
}
