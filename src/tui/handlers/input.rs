//! Handler for main input (chat input, suggestions, scroll).

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::mpsc;
use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::llm;

use super::super::app::{App, ScrollPosition};
use super::super::constants::SUGGESTIONS;
use super::PendingChat;

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
                app.input.clear();
                app.push_user(&input);
                app.push_assistant(String::new());
                app.scroll = ScrollPosition::Line(0);

                let (progress_tx, progress_rx) = mpsc::channel();
                let (stream_tx, stream_rx) = mpsc::channel();
                let (result_tx, result_rx) = mpsc::channel();
                let config = Arc::clone(config);
                let rt_clone = Arc::clone(rt);
                let mode = SUGGESTIONS[app.selected_suggestion].to_string();
                let prev_messages = api_messages.clone();
                let model_id = app.current_model_id.clone();
                std::thread::spawn(move || {
                    let on_progress: llm::OnProgress = Box::new(move |s| {
                        let _ = progress_tx.send(s.to_string());
                    });
                    let on_content_chunk: llm::OnContentChunk = Box::new(move |s| {
                        let _ = stream_tx.send(s.to_string());
                    });
                    let result = rt_clone
                        .block_on(llm::chat(
                            config.as_ref(),
                            &model_id,
                            &input,
                            &mode,
                            None,
                            prev_messages,
                            Some(on_progress),
                            Some(on_content_chunk),
                        ))
                        .map_err(|e| e.to_string());
                    let _ = result_tx.send(result);
                });
                *pending_chat = Some(PendingChat {
                    progress_rx,
                    stream_rx,
                    result_rx,
                });
            }
            super::HandleResult::Continue
        }
        (KeyCode::Backspace, _) => {
            app.input.pop();
            super::HandleResult::Continue
        }
        (KeyCode::Up, _) => {
            app.scroll_up(3);
            super::HandleResult::Continue
        }
        (KeyCode::Down, _) => {
            app.scroll_down(3);
            super::HandleResult::Continue
        }
        (KeyCode::PageUp, _) => {
            app.scroll_up(10);
            super::HandleResult::Continue
        }
        (KeyCode::PageDown, _) => {
            app.scroll_down(10);
            super::HandleResult::Continue
        }
        (KeyCode::Char(c), _) => {
            app.input.push(c);
            super::HandleResult::Continue
        }
        _ => super::HandleResult::Continue,
    }
}
