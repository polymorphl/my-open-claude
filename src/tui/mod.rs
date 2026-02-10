//! TUI (Text User Interface) to interact with the Claude assistant in chat mode.

mod app;
mod constants;
mod draw;
mod text;

#[allow(unused_imports)]
pub use app::{App, ChatMessage, ConfirmPopup, ModelSelectorState};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::execute;
use ratatui::layout::Position;
use std::io::{self, Write};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::credits;
use crate::core::llm;
use crate::core::models::{self, filter_models};
use crate::core::persistence;

use constants::SUGGESTIONS;

const CREDITS_URL: &str = "https://openrouter.ai/settings/credits";
const CREDITS_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 minutes

/// Set cursor to pointer (hand) or default. Uses OSC 22 (Kitty, iTerm2, Ghostty, Foot).
fn set_cursor_shape(pointer: bool) {
    let seq = if pointer {
        b"\x1b]22;pointer\x07"
    } else {
        b"\x1b]22;default\x07"
    };
    let _ = io::stdout().write_all(seq);
    let _ = io::stdout().flush();
}

enum ModelSelectorAction {
    Close,
    Select(models::ModelInfo),
}

/// Holds receivers for a chat request in progress (progress logs, streamed content, final result).
struct PendingChat {
    progress_rx: mpsc::Receiver<String>,
    stream_rx: mpsc::Receiver<String>,
    result_rx: mpsc::Receiver<Result<llm::ChatResult, String>>,
}

use draw::draw;

/// Guard that restores terminal state on drop (including on panic).
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Self {
        Self
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        set_cursor_shape(false); // restore default cursor
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

/// Run the TUI loop. Uses a dedicated Tokio runtime for async chat calls.
pub fn run(config: Arc<Config>) -> io::Result<()> {
    use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, enable_raw_mode};
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;

    let _guard = TerminalGuard::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let rt = Arc::new(
        Runtime::new().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Failed to create runtime: {}", e))
        })?,
    );

    let model_name = models::resolve_model_display_name(&config.model_id);
    let mut app = App::new(config.model_id.clone(), model_name);
    let mut api_messages: Option<Vec<Value>> = None;
    let mut pending_chat: Option<PendingChat> = None;
    let mut pending_model_fetch: Option<mpsc::Receiver<Result<Vec<models::ModelInfo>, String>>> = None;

    // Enable mouse events for credits click
    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;

    // Start credits fetch in background
    let mut pending_credits_fetch = {
        let (credits_tx, credits_rx) = mpsc::channel();
        let config = Arc::clone(&config);
        let rt_clone = Arc::clone(&rt);
        thread::spawn(move || {
            let result = rt_clone
                .block_on(credits::fetch_credits(config.as_ref()))
                .map(|d| (d.total_credits, d.total_usage))
                .map_err(|e| e.to_string());
            let _ = credits_tx.send(result);
        });
        Some(credits_rx)
    };

    loop {
        if let Some(ref credits_rx) = pending_credits_fetch {
            if let Ok(result) = credits_rx.try_recv() {
                if let Ok((total, used)) = result {
                    app.credit_data = Some((total, used));
                    app.credits_last_fetched_at = Some(Instant::now());
                }
                pending_credits_fetch = None;
            }
        }

        // Re-fetch credits every 30 minutes (only after first successful fetch)
        if pending_credits_fetch.is_none()
            && app
                .credits_last_fetched_at
                .is_some_and(|t| t.elapsed() >= CREDITS_REFRESH_INTERVAL)
        {
            let config = Arc::clone(&config);
            let rt_clone = Arc::clone(&rt);
            let (tx, rx) = mpsc::channel();
            pending_credits_fetch = Some(rx);
            thread::spawn(move || {
                let result = rt_clone
                    .block_on(credits::fetch_credits(config.as_ref()))
                    .map(|d| (d.total_credits, d.total_usage))
                    .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });
        }

        if let Some(ref fetch_rx) = pending_model_fetch {
            if let Ok(result) = fetch_rx.try_recv() {
                if let Some(ref mut selector) = app.model_selector {
                    match result {
                        Ok(models) => {
                            selector.models = models;
                            selector.selected_index = 0;
                            selector.fetch_error = None;
                        }
                        Err(e) => {
                            selector.fetch_error = Some(e);
                        }
                    }
                }
                pending_model_fetch = None;
            }
        }

        if let Some(ref mut chat) = pending_chat {
            while let Ok(msg) = chat.progress_rx.try_recv() {
                app.remove_last_if_empty_assistant();
                app.push_tool_log(msg);
            }
            while let Ok(chunk) = chat.stream_rx.try_recv() {
                app.append_assistant_chunk(&chunk);
            }
            if let Ok(result) = chat.result_rx.try_recv() {
                app.set_thinking(false);
                handle_chat_result(&mut app, &mut api_messages, result, true);
                pending_chat = None;
            }
        }

        terminal.draw(|f| draw(f, &mut app, f.area()))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Mouse(mouse) => {
                    // Crossterm mouse coords can be 1-based (xterm SGR); convert for Rect::contains
                    let pos = Position::new(
                        mouse.column.saturating_sub(1),
                        mouse.row.saturating_sub(1),
                    );
                    let over_credits = app
                        .credits_header_rect
                        .is_some_and(|rect| rect.contains(pos));
                    if app.confirm_popup.is_none() && app.model_selector.is_none() {
                        match mouse.kind {
                            MouseEventKind::Moved => {
                                if app.hovering_credits != over_credits {
                                    app.hovering_credits = over_credits;
                                    set_cursor_shape(over_credits);
                                }
                            }
                            MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                                if over_credits {
                                    let _ = opener::open(CREDITS_URL);
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                app.scroll_up(3);
                            }
                            MouseEventKind::ScrollDown => {
                                app.scroll_down(3);
                            }
                            _ => {}
                        }
                    }
                }
                Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                if let Some(popup) = app.confirm_popup.take() {
                    let confirmed = matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y'));
                    let cancelled =
                        matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter);
                    if confirmed || cancelled {
                        if pending_chat.is_none() {
                            app.push_assistant(String::new());
                            app.scroll = app::ScrollPosition::Line(0);
                            let (progress_tx, progress_rx) = mpsc::channel();
                            let (stream_tx, stream_rx) = mpsc::channel();
                            let (result_tx, result_rx) = mpsc::channel();
                            let config = Arc::clone(&config);
                            let model_id = app.current_model_id.clone();
                            let rt_clone = Arc::clone(&rt);
                            thread::spawn(move || {
                                let on_progress: llm::OnProgress = Box::new(move |s| {
                                    let _ = progress_tx.send(s.to_string());
                                });
                                let on_content_chunk: llm::OnContentChunk = Box::new(move |s| {
                                    let _ = stream_tx.send(s.to_string());
                                });
                                let result = rt_clone.block_on(llm::chat_resume(
                                    config.as_ref(),
                                    &model_id,
                                    popup.state,
                                    confirmed,
                                    Some(on_progress),
                                    Some(on_content_chunk),
                                ));
                                let _ = result_tx.send(result.map_err(|e| e.to_string()));
                            });
                            pending_chat = Some(PendingChat {
                                progress_rx,
                                stream_rx,
                                result_rx,
                            });
                        }
                    } else {
                        app.confirm_popup = Some(popup);
                    }
                    continue;
                }

                if app.model_selector.is_some() {
                    let action = if let Some(ref mut selector) = app.model_selector {
                        match key.code {
                            KeyCode::Backspace => {
                                selector.filter.pop();
                            }
                            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                                selector.filter.push(c);
                            }
                            _ => {}
                        }
                        let filtered = filter_models(&selector.models, &selector.filter);
                        match key.code {
                            KeyCode::Esc => Some(ModelSelectorAction::Close),
                            KeyCode::Up => {
                                selector.selected_index = selector.selected_index.saturating_sub(1);
                                None
                            }
                            KeyCode::Down => {
                                if !filtered.is_empty() {
                                    selector.selected_index = (selector.selected_index + 1)
                                        .min(filtered.len().saturating_sub(1));
                                }
                                None
                            }
                            KeyCode::Enter => {
                                if selector.fetch_error.is_none()
                                    && selector.selected_index < filtered.len()
                                {
                                    Some(ModelSelectorAction::Select(
                                        filtered[selector.selected_index].clone(),
                                    ))
                                } else {
                                    None
                                }
                            }
                            KeyCode::Backspace | KeyCode::Char(_) => {
                                selector.selected_index = selector
                                    .selected_index
                                    .min(filtered.len().saturating_sub(1));
                                None
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    if let Some(action) = action {
                        match action {
                            ModelSelectorAction::Close => {
                                app.model_selector = None;
                                pending_model_fetch = None;
                            }
                            ModelSelectorAction::Select(model) => {
                                app.current_model_id = model.id.clone();
                                app.model_name = model.name.clone();
                                let _ = persistence::save_last_model(&model.id);
                                app.model_selector = None;
                                pending_model_fetch = None;
                            }
                        }
                    }
                    continue;
                }

                // Alt+M: Option+M on macOS often sends µ (U+00B5) instead of Char+m with ALT modifier
                let open_model_selector = (key.code, key.modifiers) == (KeyCode::Char('m'), KeyModifiers::ALT)
                    || key.code == KeyCode::Char('\u{00B5}') // µ = Option+M on Mac US keyboard
                    || key.code == KeyCode::F(2); // F2 as fallback (works on all platforms)
                if open_model_selector {
                    let config = Arc::clone(&config);
                    let rt_clone = Arc::clone(&rt);
                    let (tx, rx) = mpsc::channel();
                    app.model_selector = Some(app::ModelSelectorState {
                        models: vec![],
                        selected_index: 0,
                        list_state: ratatui::widgets::ListState::default(),
                        fetch_error: None,
                        filter: String::new(),
                    });
                    pending_model_fetch = Some(rx);
                    thread::spawn(move || {
                        let result = rt_clone
                            .block_on(models::fetch_models_with_tools(config.as_ref()))
                            .map_err(|e| e.to_string());
                        let _ = tx.send(result);
                    });
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
                        if !input.is_empty() && pending_chat.is_none() {
                            app.input.clear();
                            app.push_user(&input);
                            app.push_assistant(String::new());
                            app.scroll = app::ScrollPosition::Line(0);

                            let (progress_tx, progress_rx) = mpsc::channel();
                            let (stream_tx, stream_rx) = mpsc::channel();
                            let (result_tx, result_rx) = mpsc::channel();
                            let config = config.clone();
                            let rt = Arc::clone(&rt);
                            let mode = SUGGESTIONS[app.selected_suggestion].to_string();
                            let prev_messages = api_messages.clone();

                            let model_id = app.current_model_id.clone();
                            thread::spawn(move || {
                                let on_progress: llm::OnProgress = Box::new(move |s| {
                                    let _ = progress_tx.send(s.to_string());
                                });
                                let on_content_chunk: llm::OnContentChunk = Box::new(move |s| {
                                    let _ = stream_tx.send(s.to_string());
                                });
                                let result = rt
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

                            pending_chat = Some(PendingChat {
                                progress_rx,
                                stream_rx,
                                result_rx,
                            });
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
                _ => {}
            }
        }
    }

    terminal.show_cursor()?;
    Ok(())
}

fn handle_chat_result(
    app: &mut App,
    api_messages: &mut Option<Vec<Value>>,
    result: Result<llm::ChatResult, impl std::fmt::Display>,
    tool_log_already_streamed: bool,
) {
    match result {
        Ok(llm::ChatResult::Complete {
            content,
            tool_log,
            messages,
        }) => {
            *api_messages = Some(messages);
            if tool_log_already_streamed {
                app.clear_progress_after_last_user();
            } else {
                for line in tool_log {
                    app.push_tool_log(line);
                }
            }
            app.replace_or_push_assistant(content);
            app.scroll = app::ScrollPosition::Line(0);
        }
        Ok(llm::ChatResult::NeedsConfirmation { command, state }) => {
            app.confirm_popup = Some(app::ConfirmPopup { command, state });
        }
        Err(e) => {
            app.replace_or_push_assistant(format!("Error: {}", e));
            app.scroll = app::ScrollPosition::Line(0);
        }
    }
}
