//! TUI (Text User Interface) to interact with the Claude assistant in chat mode.

mod app;
mod chat_result;
mod constants;
mod draw;
mod handlers;
mod shortcuts;
mod text;

#[allow(unused_imports)]
pub use app::{App, ChatMessage, ConfirmPopup, HistorySelectorState, ModelSelectorState};

use crossterm::event::{self, Event};
use crossterm::execute;
use std::io::{self};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::runtime::Runtime;

use crate::core::config::Config;
use crate::core::credits;
use crate::core::models::{self};
use crate::core::workspace::Workspace;

use handlers::{HandleResult, PendingChat, set_cursor_shape};

const CREDITS_REFRESH_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 minutes

use draw::draw;

/// Spawn credits fetch in background. Returns receiver for (total_credits, total_usage) or error string.
fn spawn_credits_fetch(
    config: Arc<Config>,
    rt: &Arc<Runtime>,
) -> mpsc::Receiver<Result<(f64, f64), String>> {
    let (tx, rx) = mpsc::channel();
    let rt_clone = Arc::clone(rt);
    thread::spawn(move || {
        let result = rt_clone
            .block_on(credits::fetch_credits(config.as_ref()))
            .map(|d| (d.total_credits, d.total_usage))
            .map_err(|e| e.to_string());
        let _ = tx.send(result);
    });
    rx
}

/// Guard that restores terminal state on drop (including on panic).
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Self {
        Self
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
        let _ = execute!(
            std::io::stdout(),
            crossterm::event::PopKeyboardEnhancementFlags
        );
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        set_cursor_shape(false); // restore default cursor
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

/// Run the TUI loop. Uses a dedicated Tokio runtime for async chat calls.
pub fn run(config: Arc<Config>, workspace: Workspace) -> io::Result<()> {
    use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, enable_raw_mode};
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    let _guard = TerminalGuard::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, Clear(ClearType::All))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let rt = Arc::new(
        Runtime::new().map_err(|e| io::Error::other(format!("Failed to create runtime: {}", e)))?,
    );

    let model_name = models::resolve_model_display_name(&config.model_id);
    let mut app = App::new(
        config.model_id.clone(),
        model_name,
        workspace,
        config.show_timestamps,
    );
    let mut api_messages: Option<Vec<Value>> = None;
    let mut pending_chat: Option<PendingChat> = None;
    let mut pending_model_fetch: Option<mpsc::Receiver<Result<Vec<models::ModelInfo>, String>>> =
        None;

    // Enable mouse events for credits click
    execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;

    // Kitty keyboard protocol: Alt+key as single event with modifier (Ghostty, WezTerm, kitty, etc.)
    let _ = execute!(
        io::stdout(),
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | crossterm::event::KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        )
    );

    // Start credits fetch in background
    let mut pending_credits_fetch = Some(spawn_credits_fetch(Arc::clone(&config), &rt));

    loop {
        if let Some(ref credits_rx) = pending_credits_fetch
            && let Ok(result) = credits_rx.try_recv()
        {
            match result {
                Ok((total, used)) => {
                    app.credit_data = Some((total, used));
                    app.credits_last_fetched_at = Some(Instant::now());
                    app.credits_fetch_error = None;
                }
                Err(e) => {
                    app.credits_fetch_error = Some(e);
                }
            }
            pending_credits_fetch = None;
        }

        // Re-fetch credits every 30 minutes (only after first successful fetch)
        if pending_credits_fetch.is_none()
            && app
                .credits_last_fetched_at
                .is_some_and(|t| t.elapsed() >= CREDITS_REFRESH_INTERVAL)
        {
            pending_credits_fetch = Some(spawn_credits_fetch(Arc::clone(&config), &rt));
        }

        if let Some(ref fetch_rx) = pending_model_fetch
            && let Ok(result) = fetch_rx.try_recv()
        {
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
                app.is_streaming = false;
                chat_result::handle_chat_result(
                    &mut app,
                    &mut api_messages,
                    result,
                    true,
                    config.as_ref(),
                );
                pending_chat = None;
            }
        }

        terminal.draw(|f| draw(f, &mut app, f.area()))?;

        if event::poll(std::time::Duration::from_millis(
            constants::EVENT_POLL_TIMEOUT_MS,
        ))? {
            match event::read()? {
                Event::Mouse(mouse) => {
                    let _ = handlers::handle_mouse(mouse, &mut app);
                }
                Event::Key(key) => {
                    // When Esc would start Option+key (meta), drain: terminals (Ghostty, etc.) send
                    // Esc+key; the second byte may arrive with delay—loop with short polls.
                    let key_to_handle =
                        if handlers::would_esc_start_meta_sequence(&key, &app, &pending_chat) {
                            let step_ms = 25u64;
                            let mut elapsed = 0u64;
                            let mut next_key = None;
                            while elapsed < constants::ESC_SEQUENCE_DRAIN_MS {
                                if event::poll(std::time::Duration::from_millis(step_ms))? {
                                    match event::read()? {
                                        Event::Key(next) => {
                                            next_key = Some(next);
                                            break;
                                        }
                                        Event::Mouse(m) => {
                                            let _ = handlers::handle_mouse(m, &mut app);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                                elapsed += step_ms;
                            }
                            match next_key {
                                Some(next) => {
                                    app.escape_pending = true;
                                    next
                                }
                                None => key,
                            }
                        } else {
                            key
                        };
                    let result = handlers::handle_key(
                        key_to_handle,
                        handlers::HandleKeyContext {
                            app: &mut app,
                            config: &config,
                            api_messages: &mut api_messages,
                            pending_chat: &mut pending_chat,
                            pending_model_fetch: &mut pending_model_fetch,
                            rt: &rt,
                        },
                    );
                    if result == HandleResult::Break {
                        chat_result::save_conversation_if_dirty(
                            &mut app,
                            &api_messages,
                            config.as_ref(),
                        );
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    terminal.show_cursor()?;
    Ok(())
}
