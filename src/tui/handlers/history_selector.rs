//! Handler for history selector popup (Alt+H).

use crossterm::event::{KeyCode, KeyModifiers};

use crate::core::history::filter_conversations;

use super::super::app::HistorySelectorState;

/// Action to apply after handling a history selector key.
pub(crate) enum HistorySelectorAction {
    Close,
    Load {
        id: String,
    },
    Delete {
        id: String,
    },
    Rename {
        id: String,
        new_title: String,
    },
    /// No action; keep the selector open.
    Keep,
}

/// Handle key when history selector is open.
pub(crate) fn handle_history_selector_key(
    key_code: KeyCode,
    key_modifiers: KeyModifiers,
    selector: &mut HistorySelectorState,
) -> HistorySelectorAction {
    // When renaming, keys go to rename input
    if selector.renaming.is_some() {
        match key_code {
            KeyCode::Esc => {
                selector.renaming = None;
                return HistorySelectorAction::Keep;
            }
            KeyCode::Enter => {
                if let Some((id, new_title)) = selector.renaming.take() {
                    return HistorySelectorAction::Rename { id, new_title };
                }
                return HistorySelectorAction::Keep;
            }
            KeyCode::Backspace => {
                if let Some((_, ref mut input)) = selector.renaming {
                    input.pop();
                }
                return HistorySelectorAction::Keep;
            }
            KeyCode::Char(c) if !key_modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some((_, ref mut input)) = selector.renaming {
                    input.push(c);
                }
                return HistorySelectorAction::Keep;
            }
            _ => return HistorySelectorAction::Keep,
        }
    }

    match key_code {
        KeyCode::Backspace => {
            selector.filter.pop();
        }
        KeyCode::Char(c) if !key_modifiers.contains(KeyModifiers::CONTROL) => {
            selector.filter.push(c);
        }
        _ => {}
    }

    let filtered = filter_conversations(&selector.conversations, &selector.filter);
    match key_code {
        KeyCode::Esc => HistorySelectorAction::Close,
        KeyCode::Char('r') if key_modifiers.contains(KeyModifiers::CONTROL) => {
            if selector.selected_index < filtered.len() {
                let meta = filtered[selector.selected_index];
                selector.renaming = Some((meta.id.clone(), meta.title.clone()));
                HistorySelectorAction::Keep
            } else {
                HistorySelectorAction::Keep
            }
        }
        KeyCode::Delete => {
            if selector.selected_index < filtered.len() {
                HistorySelectorAction::Delete {
                    id: filtered[selector.selected_index].id.clone(),
                }
            } else {
                HistorySelectorAction::Keep
            }
        }
        KeyCode::Char('d') if key_modifiers.contains(KeyModifiers::CONTROL) => {
            if selector.selected_index < filtered.len() {
                HistorySelectorAction::Delete {
                    id: filtered[selector.selected_index].id.clone(),
                }
            } else {
                HistorySelectorAction::Keep
            }
        }
        KeyCode::Up => {
            selector.selected_index = selector.selected_index.saturating_sub(1);
            HistorySelectorAction::Keep
        }
        KeyCode::Down => {
            if !filtered.is_empty() {
                selector.selected_index =
                    (selector.selected_index + 1).min(filtered.len().saturating_sub(1));
            }
            HistorySelectorAction::Keep
        }
        KeyCode::Enter => {
            if selector.selected_index < filtered.len() {
                let meta = filtered[selector.selected_index];
                HistorySelectorAction::Load {
                    id: meta.id.clone(),
                }
            } else {
                HistorySelectorAction::Keep
            }
        }
        KeyCode::Backspace | KeyCode::Char(_) => {
            selector.selected_index = selector
                .selected_index
                .min(filtered.len().saturating_sub(1));
            HistorySelectorAction::Keep
        }
        _ => HistorySelectorAction::Keep,
    }
}

/// Open the history selector. Caller must save current conversation first if dirty.
pub(crate) fn open_history_selector() -> HistorySelectorState {
    let conversations = crate::core::history::list_conversations().unwrap_or_else(|_| vec![]);
    HistorySelectorState {
        conversations,
        selected_index: 0,
        list_state: ratatui::widgets::ListState::default(),
        filter: String::new(),
        renaming: None,
    }
}
