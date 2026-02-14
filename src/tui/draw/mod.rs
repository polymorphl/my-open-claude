//! TUI rendering: layout and widgets for the chat interface.

mod header;
mod history;
mod history_selector_popup;
mod input;
mod popups;
mod welcome_mascot;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::time::Instant;

use crate::core::commands;

use super::app::App;
use super::constants::ACCENT;

pub(super) fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let is_welcome = app.messages.is_empty();
    if is_welcome {
        app.history_area_rect = None;
        app.message_line_ranges.clear();
    }

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
        let input_section_height = if app.input.starts_with('/')
            && !commands::filter_commands(app.input.get(1..).unwrap_or("")).is_empty()
        {
            input::AUTOCOMPLETE_VISIBLE_LINES + super::constants::INPUT_LINES + 3
        } else {
            super::constants::INPUT_LINES + 3
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(3),
                Constraint::Length(input_section_height),
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

    // Toast: top right, below header (y=2). Opaque background so it's visible over history.
    if let Some(deadline) = app.copy_toast_until {
        if deadline > Instant::now() {
            const HEADER_HEIGHT: u16 = 2;
            let toast_text = " Copied ";
            let toast_width = toast_text.len() as u16 + 2;
            let toast_height = 3u16; // borders + content
            let toast_area = Rect {
                x: area.x + area.width.saturating_sub(toast_width).saturating_sub(1),
                y: area.y + HEADER_HEIGHT,
                width: toast_width,
                height: toast_height,
            };
            f.render_widget(Clear, toast_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(Color::Black));
            let para = Paragraph::new(Line::from(toast_text))
                .block(block)
                .style(Style::default().fg(ACCENT).bg(Color::Black));
            f.render_widget(para, toast_area);
        } else {
            app.copy_toast_until = None;
        }
    }
}
