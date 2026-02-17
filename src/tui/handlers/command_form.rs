//! Handler for create/update command form popup.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::core::commands::BUILTIN_NAMES;
use crate::core::templates::{self, CustomTemplate};

use super::HandleResult;
use crate::tui::app::{CommandFormField, CommandFormMode, CommandFormPhase, CommandFormState};

fn validate_name(name: &str, exclude: Option<&str>, custom_names: &[String]) -> Option<String> {
    if name.trim().is_empty() {
        return Some("Name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(
            "Name must contain only letters, numbers, hyphens, and underscores".to_string(),
        );
    }
    let name_lower = name.to_lowercase();
    if BUILTIN_NAMES.contains(&name_lower.as_str()) {
        return Some("Name conflicts with built-in command".to_string());
    }
    for other in custom_names {
        let other_lower = other.to_lowercase();
        if other_lower == name_lower {
            if exclude.is_some_and(|ex| ex.to_lowercase() == other_lower) {
                continue;
            }
            return Some("Name already used by another custom command".to_string());
        }
    }
    None
}

fn validate_form(state: &CommandFormState, custom_templates: &[CustomTemplate]) -> Option<String> {
    let custom_names: Vec<String> = custom_templates.iter().map(|t| t.name.clone()).collect();
    let exclude = match &state.form_mode {
        CommandFormMode::Update {
            original_name: Some(n),
        } => Some(n.as_str()),
        _ => None,
    };
    if let Some(e) = validate_name(&state.name, exclude, &custom_names) {
        return Some(e);
    }
    if state.description.trim().is_empty() {
        return Some("Description cannot be empty".to_string());
    }
    if state.prompt_prefix.trim().is_empty() {
        return Some("Prompt cannot be empty".to_string());
    }
    if state.llm_mode != "Ask" && state.llm_mode != "Build" {
        return Some("Mode must be Ask or Build".to_string());
    }
    None
}

fn save_command(app: &mut crate::tui::app::App, state: CommandFormState) {
    let template = templates::CustomTemplate {
        name: state.name.trim().to_string(),
        description: state.description.trim().to_string(),
        prompt_prefix: state.prompt_prefix.trim().to_string(),
        mode: state.llm_mode.clone(),
    };

    match &state.form_mode {
        CommandFormMode::Create => {
            app.custom_templates.push(template);
        }
        CommandFormMode::Update {
            original_name: Some(orig),
        } => {
            if let Some(idx) = app
                .custom_templates
                .iter()
                .position(|t| t.name.to_lowercase() == orig.to_lowercase())
            {
                app.custom_templates[idx] = template;
            }
        }
        _ => return,
    }

    match templates::save_templates(&app.custom_templates) {
        Ok(()) => {
            app.reload_resolved_commands();
            app.templates_load_error = None;
        }
        Err(e) => {
            app.templates_load_error = Some(e.to_string());
        }
    }
}

pub(super) fn handle_command_form_popup(
    key_code: KeyCode,
    key_modifiers: KeyModifiers,
    app: &mut crate::tui::app::App,
) -> HandleResult {
    let Some(state) = app.command_form_popup.as_mut() else {
        return HandleResult::Continue;
    };

    match state.phase {
        CommandFormPhase::SelectCommand => {
            let len = app.custom_templates.len();
            if len == 0 {
                app.command_form_popup = None;
                return HandleResult::Continue;
            }
            match key_code {
                KeyCode::Esc => {
                    app.command_form_popup = None;
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
                KeyCode::Enter => {
                    let template = app.custom_templates[state.selected_index].clone();
                    let original = template.name.clone();
                    state.phase = CommandFormPhase::EditForm;
                    state.name = template.name;
                    state.description = template.description;
                    state.prompt_prefix = template.prompt_prefix;
                    state.llm_mode = template.mode;
                    state.focused_field = CommandFormField::Name;
                    state.error = None;
                    if let CommandFormMode::Update { original_name } = &mut state.form_mode {
                        *original_name = Some(original);
                    }
                }
                _ => {}
            }
        }
        CommandFormPhase::EditForm => match key_code {
            KeyCode::Esc => {
                if matches!(state.form_mode, CommandFormMode::Update { .. }) {
                    state.phase = CommandFormPhase::SelectCommand;
                    state.name.clear();
                    state.description.clear();
                    state.prompt_prefix.clear();
                } else {
                    app.command_form_popup = None;
                }
            }
            KeyCode::Tab => {
                state.focused_field = match state.focused_field {
                    CommandFormField::Name => CommandFormField::Description,
                    CommandFormField::Description => CommandFormField::Prompt,
                    CommandFormField::Prompt => CommandFormField::Mode,
                    CommandFormField::Mode => CommandFormField::Name,
                };
                state.error = None;
            }
            KeyCode::BackTab => {
                state.focused_field = match state.focused_field {
                    CommandFormField::Name => CommandFormField::Mode,
                    CommandFormField::Description => CommandFormField::Name,
                    CommandFormField::Prompt => CommandFormField::Description,
                    CommandFormField::Mode => CommandFormField::Prompt,
                };
                state.error = None;
            }
            KeyCode::Enter
                if state.focused_field == CommandFormField::Prompt
                    && (key_modifiers.contains(KeyModifiers::SHIFT)
                        || key_modifiers.contains(KeyModifiers::ALT)) =>
            {
                state.prompt_prefix.push('\n');
            }
            KeyCode::Enter => {
                if state.focused_field == CommandFormField::Mode {
                    state.llm_mode = if state.llm_mode == "Ask" {
                        "Build".to_string()
                    } else {
                        "Ask".to_string()
                    };
                    return HandleResult::Continue;
                }
                let custom = app.custom_templates.clone();
                if let Some(e) = validate_form(state, &custom) {
                    state.error = Some(e);
                    return HandleResult::Continue;
                }
                let Some(state) = app.command_form_popup.take() else {
                    return HandleResult::Continue;
                };
                save_command(app, state);
            }
            KeyCode::Backspace => {
                state.error = None;
                match state.focused_field {
                    CommandFormField::Name => {
                        if !state.name.is_empty() {
                            let idx = state
                                .name
                                .char_indices()
                                .last()
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            state.name.truncate(idx);
                        }
                    }
                    CommandFormField::Description => {
                        if !state.description.is_empty() {
                            let idx = state
                                .description
                                .char_indices()
                                .last()
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            state.description.truncate(idx);
                        }
                    }
                    CommandFormField::Prompt => {
                        if !state.prompt_prefix.is_empty() {
                            let idx = state
                                .prompt_prefix
                                .char_indices()
                                .last()
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            state.prompt_prefix.truncate(idx);
                        }
                    }
                    CommandFormField::Mode => {}
                }
            }
            KeyCode::Up | KeyCode::Down => {
                if state.focused_field == CommandFormField::Mode {
                    state.llm_mode = if state.llm_mode == "Ask" {
                        "Build".to_string()
                    } else {
                        "Ask".to_string()
                    };
                }
            }
            KeyCode::Char(c) => {
                if key_modifiers.contains(KeyModifiers::ALT) {
                    return HandleResult::Continue;
                }
                state.error = None;
                match state.focused_field {
                    CommandFormField::Name => state.name.push(c),
                    CommandFormField::Description => state.description.push(c),
                    CommandFormField::Prompt => state.prompt_prefix.push(c),
                    CommandFormField::Mode => {
                        if c == ' ' || c == '\t' {
                            state.llm_mode = if state.llm_mode == "Ask" {
                                "Build".to_string()
                            } else {
                                "Ask".to_string()
                            };
                        }
                    }
                }
            }
            _ => {}
        },
    }

    HandleResult::Continue
}
