//! TUI rendering: layout and widgets for the chat interface.

mod header;
mod history;
mod history_selector_popup;
mod input;
mod popups;
mod welcome_raccoon;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};

use super::app::App;
use super::constants::ACCENT;

pub(super) fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let is_welcome = app.messages.is_empty();

    if is_welcome {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(35),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .flex(Flex::Center)
            .split(area);
        header::draw_header(f, app, chunks[0], ACCENT);
        input::draw_welcome_center(f, app, chunks[2]);
        input::draw_bottom_bar(f, app, chunks[4]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(3),
                Constraint::Length(6),
            ])
            .split(area);
        header::draw_header(f, app, chunks[0], ACCENT);
        history::draw_history(f, app, chunks[1]);
        input::draw_input_section(f, app, chunks[2]);
    }

    if let Some(ref popup) = app.confirm_popup {
        popups::draw_confirm_popup(f, area, &popup.command);
    }
    if let Some(ref mut selector) = app.model_selector {
        popups::draw_model_selector_popup(f, area, selector);
    }
    if let Some(ref mut selector) = app.history_selector {
        history_selector_popup::draw_history_selector_popup(f, area, selector);
    }
}
