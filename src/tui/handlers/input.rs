//! Handler for main input (chat input, suggestions, scroll, slash commands).

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::commands::{self, SlashCommand};
use crate::core::config::Config;

use super::super::app::{App, ScrollPosition};
use super::super::constants::{self, SUGGESTIONS};
use super::PendingChat;
use super::chat_spawn;

/// Filter query from input: everything after the leading "/".
fn slash_filter(app: &App) -> &str {
    if app.input.starts_with('/') {
        app.input.get(1..).unwrap_or("")
    } else {
        ""
    }
}

/// Get filtered commands for current input. Returns empty when not in slash mode.
fn filtered_commands(app: &App) -> Vec<&'static SlashCommand> {
    if app.input.starts_with('/') {
        commands::filter_commands(slash_filter(app))
    } else {
        vec![]
    }
}

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
    let in_slash_mode = app.input.starts_with('/');
    let commands = filtered_commands(app);

    match (key_code, key_modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => super::HandleResult::Break,

        // Slash autocomplete: Up/Down/Tab navigate commands
        (KeyCode::Tab, KeyModifiers::SHIFT) if in_slash_mode && !commands.is_empty() => {
            app.selected_command_index = app
                .selected_command_index
                .saturating_sub(1)
                .min(commands.len().saturating_sub(1));
            super::HandleResult::Continue
        }
        (KeyCode::Tab, _) if in_slash_mode && !commands.is_empty() => {
            app.selected_command_index = (app.selected_command_index + 1) % commands.len();
            super::HandleResult::Continue
        }
        (KeyCode::Up, _) if in_slash_mode && !commands.is_empty() => {
            app.selected_command_index = app
                .selected_command_index
                .saturating_sub(1)
                .min(commands.len().saturating_sub(1));
            super::HandleResult::Continue
        }
        (KeyCode::Down, _) if in_slash_mode && !commands.is_empty() => {
            app.selected_command_index = (app.selected_command_index + 1) % commands.len();
            super::HandleResult::Continue
        }

        // Slash autocomplete: Enter selects command and inserts template
        (KeyCode::Enter, _) if in_slash_mode && !commands.is_empty() && pending_chat.is_none() => {
            let cmd = commands[app.selected_command_index];
            let rest = app
                .input
                .get(cmd.full_name().len()..)
                .unwrap_or("")
                .trim()
                .to_string();
            app.input = if rest.is_empty() {
                format!("{} ", cmd.prompt_prefix)
            } else {
                format!("{} {}", cmd.prompt_prefix, rest)
            };
            app.pending_command_mode = Some(cmd.mode.to_string());
            app.selected_command_index = 0;
            super::HandleResult::Continue
        }

        // Esc: close slash autocomplete (clear input)
        (KeyCode::Esc, _) if in_slash_mode => {
            app.input.clear();
            app.selected_command_index = 0;
            super::HandleResult::Continue
        }

        // Normal Tab: cycle Ask/Build suggestions
        (KeyCode::Tab, KeyModifiers::SHIFT) => {
            app.selected_suggestion = app.selected_suggestion.saturating_sub(1);
            super::HandleResult::Continue
        }
        (KeyCode::Tab, _) => {
            app.selected_suggestion = (app.selected_suggestion + 1) % SUGGESTIONS.len();
            super::HandleResult::Continue
        }

        // Shift+Enter or Alt+Enter: insert newline in textarea
        // Note: On macOS, Shift+Enter often reports modifiers::NONE (crossterm #669);
        // Alt+Enter (Option+Enter) usually works as a fallback.
        (KeyCode::Enter, mods)
            if mods.contains(KeyModifiers::SHIFT) || mods.contains(KeyModifiers::ALT) =>
        {
            app.input.push('\n');
            super::HandleResult::Continue
        }

        // Enter: send message
        (KeyCode::Enter, _) => {
            let input = app.input.trim().to_string();
            if !input.is_empty() && pending_chat.is_none() {
                let mode = app
                    .pending_command_mode
                    .take()
                    .unwrap_or_else(|| SUGGESTIONS[app.selected_suggestion].to_string());

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
                    mode,
                    prev_messages,
                );
                app.is_streaming = true;
                *pending_chat = Some(pc);
            }
            super::HandleResult::Continue
        }

        // Ctrl+U: clear input (e.g. recover from pasted error)
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
            app.input.clear();
            app.selected_command_index = 0;
            app.pending_command_mode = None;
            super::HandleResult::Continue
        }

        (KeyCode::Backspace, _) => {
            app.input.pop();
            if !app.input.starts_with('/') {
                app.selected_command_index = 0;
            }
            if app.input.is_empty() {
                app.pending_command_mode = None;
            }
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
        (KeyCode::Home, _) => {
            app.materialize_scroll();
            app.scroll = ScrollPosition::Line(0);
            super::HandleResult::Continue
        }
        (KeyCode::End, _) => {
            app.scroll = ScrollPosition::Bottom;
            super::HandleResult::Continue
        }
        (KeyCode::Char(c), mods) => {
            if mods.contains(KeyModifiers::ALT) {
                return super::HandleResult::Continue;
            }
            app.input.push(c);
            // Clamp selected_command_index when filter shrinks (user typed more chars)
            if app.input.starts_with('/') {
                let new_commands = commands::filter_commands(slash_filter(app));
                if !new_commands.is_empty() && app.selected_command_index >= new_commands.len() {
                    app.selected_command_index = new_commands.len().saturating_sub(1);
                }
            }
            super::HandleResult::Continue
        }
        _ => super::HandleResult::Continue,
    }
}
