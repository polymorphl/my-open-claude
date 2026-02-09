//! TUI (Text User Interface) to interact with the Claude assistant in chat mode.

mod app;
mod constants;
mod draw;
mod text;

#[allow(unused_imports)]
pub use app::{App, ChatMessage, ConfirmPopup};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use std::io;
use tokio::runtime::Handle;

use serde_json::Value;

use crate::core::llm;

use constants::SUGGESTIONS;
use draw::draw;

/// Run the TUI loop. Uses `handle` to run async chat from a blocking thread.
pub fn run(handle: Handle) -> io::Result<()> {
    use crossterm::terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    // Full API conversation history so the model keeps context across turns.
    let mut api_messages: Option<Vec<Value>> = None;
    loop {
        terminal.draw(|f| draw(f, &mut app, f.area()))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // When popup is shown, only handle y/n/Enter
                if let Some(popup) = app.confirm_popup.take() {
                    let confirmed = matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y'));
                    let cancelled = matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter);
                    if confirmed || cancelled {
                        let result = handle.block_on(llm::chat_resume(popup.state, confirmed));
                        app.set_thinking(false);
                        match result {
                            Ok(llm::ChatResult::Complete { content, tool_log, messages }) => {
                                api_messages = Some(messages);
                                for line in tool_log {
                                    app.push_tool_log(line);
                                }
                                app.push_assistant(content);
                                app.scroll = usize::MAX;
                            }
                            Ok(llm::ChatResult::NeedsConfirmation { command, state }) => {
                                app.confirm_popup = Some(app::ConfirmPopup { command, state });
                            }
                            Err(e) => {
                                app.push_assistant(format!("Error: {}", e));
                            }
                        }
                    } else {
                        app.confirm_popup = Some(popup);
                    }
                    continue;
                }

                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Tab, KeyModifiers::SHIFT) => {
                        app.selected_suggestion = app.selected_suggestion.saturating_sub(1);
                    }
                    (KeyCode::Tab, _) => {
                        app.selected_suggestion = (app.selected_suggestion + 1) % SUGGESTIONS.len();
                    }
                    (KeyCode::Enter, _) => {
                        let input = app.input.trim().to_string();
                        if input.is_empty() {
                            // Do not send when input is empty
                        } else {
                            app.input.clear();
                            app.push_user(&input);
                            app.scroll = usize::MAX;
                            app.set_thinking(true);
                            terminal.draw(|f| draw(f, &mut app, f.area()))?;
                            let mode = SUGGESTIONS[app.selected_suggestion];
                            let result = handle.block_on(llm::chat(
                                &input,
                                mode,
                                None, // TUI mode: get NeedsConfirmation and show popup
                                api_messages.clone(),
                            ));
                            app.set_thinking(false);
                            match result {
                                Ok(llm::ChatResult::Complete { content, tool_log, messages }) => {
                                    api_messages = Some(messages);
                                    for line in tool_log {
                                        app.push_tool_log(line);
                                    }
                                    app.push_assistant(content);
                                }
                                Ok(llm::ChatResult::NeedsConfirmation { command, state }) => {
                                    app.confirm_popup = Some(app::ConfirmPopup { command, state });
                                }
                                Err(e) => app.push_assistant(format!("Error: {}", e)),
                            }
                            app.scroll = usize::MAX;
                        }
                    }
                    (KeyCode::Backspace, _) => {
                        app.input.pop();
                    }
                    (KeyCode::Up, _) => app.scroll_up(3),
                    (KeyCode::Down, _) => app.scroll_down(3),
                    (KeyCode::PageUp, _) => app.scroll_up(10),
                    (KeyCode::PageDown, _) => app.scroll_down(10),
                    (KeyCode::Char(c), _) => {
                        app.input.push(c);
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
