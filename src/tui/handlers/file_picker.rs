//! Keyboard handler for the file picker popup.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::tui::app::{App, FilePickerState};

/// Result of a key event inside the file picker.
pub enum FilePickerAction {
    /// Insert the selected file path at the `@` token position.
    Insert { rel_path: std::path::PathBuf },
    /// Close the picker and remove the `@` + filter text that was typed.
    Cancel,
    /// Keep the picker open (navigation / filter update).
    Keep,
}

/// Handle a single key event for the file picker popup, returning the action to take.
pub fn handle_file_picker_key(
    code: KeyCode,
    mods: KeyModifiers,
    picker: &mut FilePickerState,
) -> FilePickerAction {
    match code {
        KeyCode::Esc => FilePickerAction::Cancel,

        KeyCode::Up => {
            let filtered_len = picker.filtered_entries().len();
            if filtered_len > 0 {
                picker.selected_index = if picker.selected_index == 0 {
                    filtered_len.saturating_sub(1)
                } else {
                    picker.selected_index - 1
                };
                picker.list_state.select(Some(picker.selected_index));
            }
            FilePickerAction::Keep
        }

        KeyCode::Down => {
            let filtered_len = picker.filtered_entries().len();
            if filtered_len > 0 {
                picker.selected_index = (picker.selected_index + 1) % filtered_len;
                picker.list_state.select(Some(picker.selected_index));
            }
            FilePickerAction::Keep
        }

        KeyCode::Enter => {
            let filtered = picker.filtered_entries();
            if filtered.is_empty() {
                return FilePickerAction::Keep;
            }
            let idx = picker.selected_index.min(filtered.len().saturating_sub(1));
            let entry = &filtered[idx];
            if entry.is_dir {
                let name = entry.name.clone();
                picker.descend(&name);
                FilePickerAction::Keep
            } else {
                let rel_path = entry.rel_path.clone();
                FilePickerAction::Insert { rel_path }
            }
        }

        KeyCode::Backspace => {
            if !picker.filter.is_empty() {
                // Pop last UTF-8 char from filter
                let mut chars = picker.filter.chars();
                chars.next_back();
                picker.filter = chars.as_str().to_string();
                // Reset selection when filter changes
                picker.selected_index = 0;
                picker.list_state.select(Some(0));
                FilePickerAction::Keep
            } else {
                picker.ascend();
                FilePickerAction::Keep
            }
        }

        KeyCode::Char(c)
            if !mods.contains(KeyModifiers::ALT) && !mods.contains(KeyModifiers::CONTROL) =>
        {
            picker.filter.push(c);
            // Clamp selected_index to new filtered length
            let filtered_len = picker.filtered_entries().len();
            if filtered_len > 0 && picker.selected_index >= filtered_len {
                picker.selected_index = filtered_len.saturating_sub(1);
            } else if filtered_len == 0 {
                picker.selected_index = 0;
            }
            picker.list_state.select(Some(picker.selected_index));
            FilePickerAction::Keep
        }

        _ => FilePickerAction::Keep,
    }
}

/// Wrapper called from handlers/mod.rs — dispatches file picker keys and applies side effects to app.
pub fn handle_file_picker(code: KeyCode, mods: KeyModifiers, app: &mut App) {
    let picker = match app.file_picker.as_mut() {
        Some(p) => p,
        None => return,
    };

    let action = handle_file_picker_key(code, mods, picker);

    match action {
        FilePickerAction::Insert { rel_path } => {
            let at_start = app
                .file_picker
                .as_ref()
                .map(|p| p.at_token_start)
                .unwrap_or(0);
            // Replace app.input[at_token_start..input_cursor] with "@rel_path "
            let insertion = format!("@{} ", rel_path.display());
            let end = app.input_cursor.min(app.input.len());
            let start = at_start.min(end);
            app.input.replace_range(start..end, &insertion);
            app.input_cursor = start + insertion.len();
            app.file_picker = None;
        }
        FilePickerAction::Cancel => {
            // Remove the `@` and any filter chars that were typed
            let at_start = app
                .file_picker
                .as_ref()
                .map(|p| p.at_token_start)
                .unwrap_or(0);
            let end = app.input_cursor.min(app.input.len());
            let start = at_start.min(end);
            app.input.replace_range(start..end, "");
            app.input_cursor = start;
            app.file_picker = None;
        }
        FilePickerAction::Keep => {}
    }
}
