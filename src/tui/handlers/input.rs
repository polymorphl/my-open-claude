//! Handler for main input (chat input, suggestions, scroll).

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;

use super::super::app::{App, ScrollPosition};
use super::super::constants::{self, SUGGESTIONS};
use super::PendingChat;
use super::chat_spawn;

/// Handle main input keys (when no popup is open).
pub(crate) fn handle_main_input(
    key_code: KeyCode,
    key_modifiers: KeyModifiers,
    app: &mut App,
    config: &Arc<Config>,
    pending_chat: &mut Option<PendingChat>,
    api_messages: &mut Option<Vec<Value>>,
    rt: &Arc<Runtime>,
) -> super::HandleResult {
    match (key_code, key_modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => super::HandleResult::Break,
        (KeyCode::Tab, KeyModifiers::SHIFT) => {
            app.selected_suggestion = app.selected_suggestion.saturating_sub(1);
            super::HandleResult::Continue
        }
        (KeyCode::Tab, _) => {
            app.selected_suggestion = (app.selected_suggestion + 1) % SUGGESTIONS.len();
            super::HandleResult::Continue
        }
        (KeyCode::Enter, _) => {
            let input = app.input.trim().to_string();
            if !input.is_empty() && pending_chat.is_none() {
                app.mark_dirty();
                app.input.clear();
                app.push_user(&input);
                app.push_assistant(String::new());
                app.scroll = ScrollPosition::Bottom;

                let model_id = app.current_model_id.clone();
                let prev_messages = api_messages.clone();
                let pc = chat_spawn::spawn_chat(
                    rt,
                    Arc::clone(config),
                    model_id,
                    input,
                    SUGGESTIONS[app.selected_suggestion].to_string(),
                    prev_messages,
                );
                app.is_streaming = true;
                *pending_chat = Some(pc);
            }
            super::HandleResult::Continue
        }
        (KeyCode::Backspace, _) => {
            app.input.pop();
            super::HandleResult::Continue
        }
        (KeyCode::Up, _) => {
            app.scroll_up(constants::SCROLL_LINES_SMALL);
            super::HandleResult::Continue
        }
        (KeyCode::Down, _) => {
            app.scroll_down(constants::SCROLL_LINES_SMALL);
            super::HandleResult::Continue
        }
        (KeyCode::PageUp, _) => {
            app.scroll_up(constants::SCROLL_LINES_PAGE);
            super::HandleResult::Continue
        }
        (KeyCode::PageDown, _) => {
            app.scroll_down(constants::SCROLL_LINES_PAGE);
            super::HandleResult::Continue
        }
        (KeyCode::Char(c), mods) => {
            // Ignore Alt+key: user likely intended a shortcut (e.g. Alt+H)
            if mods.contains(KeyModifiers::ALT) {
                return super::HandleResult::Continue;
            }
            app.input.push(c);
            super::HandleResult::Continue
        }
        _ => super::HandleResult::Continue,
    }
}
