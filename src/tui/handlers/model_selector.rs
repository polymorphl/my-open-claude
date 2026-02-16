//! Handler for model selector popup.

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Instant;

use ratatui::widgets::ListState;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::models::{self, ModelInfo, filter_models};

use super::super::app::{App, ModelSelectorState};

/// Action to apply after handling a model selector key.
pub(crate) enum ModelSelectorAction {
    Close,
    Select(ModelInfo),
    /// No action; keep the selector open.
    Keep,
}

/// Handle key when model selector is open. Returns action to apply; caller applies to app.
pub(crate) fn handle_model_selector_key(
    key_code: KeyCode,
    key_modifiers: KeyModifiers,
    selector: &mut ModelSelectorState,
) -> ModelSelectorAction {
    // Filter input
    match key_code {
        KeyCode::Backspace => {
            selector.filter.pop();
        }
        KeyCode::Char(c) if !key_modifiers.contains(KeyModifiers::CONTROL) => {
            selector.filter.push(c);
        }
        _ => {}
    }

    let filtered = filter_models(&selector.models, &selector.filter);
    match key_code {
        KeyCode::Esc => ModelSelectorAction::Close,
        KeyCode::Up => {
            selector.selected_index = selector.selected_index.saturating_sub(1);
            ModelSelectorAction::Keep
        }
        KeyCode::Down => {
            if !filtered.is_empty() {
                selector.selected_index =
                    (selector.selected_index + 1).min(filtered.len().saturating_sub(1));
            }
            ModelSelectorAction::Keep
        }
        KeyCode::Enter => {
            if selector.fetch_error.is_none() && selector.selected_index < filtered.len() {
                ModelSelectorAction::Select(filtered[selector.selected_index].clone())
            } else {
                ModelSelectorAction::Keep
            }
        }
        KeyCode::Backspace | KeyCode::Char(_) => {
            selector.selected_index = selector
                .selected_index
                .min(filtered.len().saturating_sub(1));
            ModelSelectorAction::Keep
        }
        _ => ModelSelectorAction::Keep,
    }
}

/// Open the model selector.
pub(crate) fn open_model_selector(
    app: &mut App,
    config: &Arc<Config>,
    pending_model_fetch: &mut Option<mpsc::Receiver<Result<Vec<ModelInfo>, String>>>,
    rt: &Arc<Runtime>,
) {
    let config = Arc::clone(config);
    let rt_clone = Arc::clone(rt);
    let (tx, rx) = mpsc::channel();
    app.model_selector = Some(ModelSelectorState {
        models: vec![],
        selected_index: 0,
        list_state: ListState::default(),
        fetch_error: None,
        filter: String::new(),
        fetch_started_at: Some(Instant::now()),
    });
    *pending_model_fetch = Some(rx);
    std::thread::spawn(move || {
        let result = rt_clone
            .block_on(models::fetch_models_with_tools(config.as_ref()))
            .map_err(|e| e.to_string());
        let _ = tx.send(result);
    });
}
