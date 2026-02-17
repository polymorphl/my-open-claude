//! Handler for delete command popup.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::core::templates;

use super::HandleResult;
use crate::tui::app::App;

pub(super) fn handle_delete_command_popup(
    key_code: KeyCode,
    _key_modifiers: KeyModifiers,
    app: &mut App,
) -> HandleResult {
    let Some(state) = app.delete_command_popup.as_mut() else {
        return HandleResult::Continue;
    };

    let len = app.custom_templates.len();
    if len == 0 {
        app.delete_command_popup = None;
        return HandleResult::Continue;
    }

    match key_code {
        KeyCode::Esc => {
            app.delete_command_popup = None;
        }
        KeyCode::Up => {
            state.selected_index = state.selected_index.saturating_sub(1);
            if state.selected_index >= len {
                state.selected_index = len.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            state.selected_index = (state.selected_index + 1).min(len.saturating_sub(1));
        }
        KeyCode::Char(' ') => {
            if state.selected_index < state.selected.len() {
                state.selected[state.selected_index] = !state.selected[state.selected_index];
            }
        }
        KeyCode::Enter => {
            let any_selected = state.selected.iter().any(|&b| b);
            if !any_selected {
                app.delete_command_popup = None;
                return HandleResult::Continue;
            }
            let mut remaining: Vec<_> = app
                .custom_templates
                .iter()
                .enumerate()
                .filter_map(|(i, t)| {
                    if state.selected[i] {
                        None
                    } else {
                        Some(t.clone())
                    }
                })
                .collect();
            std::mem::swap(&mut app.custom_templates, &mut remaining);
            if templates::save_templates(&app.custom_templates).is_ok() {
                app.reload_resolved_commands();
                app.templates_load_error = None;
            }
            app.delete_command_popup = None;
        }
        _ => {}
    }

    HandleResult::Continue
}
