//! Key handlers for popup dialogs (model selector, history selector).

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::mpsc;

use serde_json::Value;

use crate::core::history::{self};
use crate::core::llm;
use crate::core::models::ModelInfo;

use crate::tui::app::App;

use super::{HandleResult, history_selector, model_selector};

/// Handle key when model selector popup is open.
pub(super) fn handle_model_selector(
    key_code: KeyCode,
    modifiers: KeyModifiers,
    app: &mut App,
    pending_model_fetch: &mut Option<mpsc::Receiver<Result<Vec<ModelInfo>, String>>>,
) -> HandleResult {
    let Some(selector) = app.model_selector.as_mut() else {
        return HandleResult::Continue;
    };
    let action = model_selector::handle_model_selector_key(key_code, modifiers, selector);
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
    HandleResult::Continue
}

/// Handle key when history selector popup is open.
pub(super) fn handle_history_selector(
    key_code: KeyCode,
    modifiers: KeyModifiers,
    app: &mut App,
    api_messages: &mut Option<Vec<Value>>,
) -> HandleResult {
    let Some(selector) = app.history_selector.as_mut() else {
        return HandleResult::Continue;
    };
    let action = history_selector::handle_history_selector_key(key_code, modifiers, selector);
    match action {
        history_selector::HistorySelectorAction::Close => {
            app.history_selector = None;
        }
        history_selector::HistorySelectorAction::Load { id } => {
            if let Some(messages) = history::load_conversation(&id) {
                app.set_messages_from_api(&messages);
                app.set_conversation_id(Some(id.clone()));
                app.scroll = crate::tui::app::ScrollPosition::Bottom;
                app.token_usage = Some(llm::TokenUsage::estimated_from_messages(&messages));
                *api_messages = Some(messages);
            }
            app.history_selector = None;
        }
        history_selector::HistorySelectorAction::Delete { id } => {
            selector.error = None;
            match history::delete_conversation(&id) {
                Ok(()) => {
                    selector.conversations.retain(|c| c.id != id);
                    selector.content_cache.remove(&id);
                    let filtered = history::filter_conversations_with_content(
                        &selector.conversations,
                        &selector.filter,
                        &selector.content_cache,
                    );
                    selector.selected_index = selector
                        .selected_index
                        .min(filtered.len().saturating_sub(1));
                }
                Err(e) => selector.error = Some(format!("Delete failed: {}", e)),
            }
        }
        history_selector::HistorySelectorAction::Rename { id, new_title } => {
            selector.error = None;
            match history::rename_conversation(&id, &new_title) {
                Ok(()) => {
                    if let Some(meta) = selector.conversations.iter_mut().find(|c| c.id == id) {
                        meta.title = new_title.clone();
                    }
                }
                Err(e) => selector.error = Some(format!("Rename failed: {}", e)),
            }
        }
        history_selector::HistorySelectorAction::Keep => {}
    }
    HandleResult::Continue
}
