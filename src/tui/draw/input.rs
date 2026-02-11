//! Input section, welcome center, bottom bar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::env;

use super::super::app::App;
use super::super::constants::{ACCENT, SUGGESTIONS};
use super::welcome_raccoon;

/// Width of the centered input when in welcome (no conversation) mode.
const WELCOME_INPUT_WIDTH: u16 = 64;

pub(crate) fn draw_welcome_center(f: &mut Frame, app: &mut App, area: Rect) {
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(29),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let raccoon_area = inner_chunks[0];
    let input_area_outer = inner_chunks[2];
    let suggestions_area = inner_chunks[3];

    welcome_raccoon::draw_raccoon(f, raccoon_area);

    let input_width = WELCOME_INPUT_WIDTH.min(area.width);
    let input_area = Rect {
        x: area.x + area.width.saturating_sub(input_width) / 2,
        y: input_area_outer.y,
        width: input_width,
        height: input_area_outer.height,
    };
    draw_input_block(f, app, input_area);

    let suggestion_spans: Vec<Span> = SUGGESTIONS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == app.selected_suggestion;
            Span::styled(
                format!(" {} ", s),
                if selected {
                    Style::default().fg(Color::Black).bg(ACCENT)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            )
        })
        .collect();
    f.render_widget(
        Paragraph::new(Line::from(suggestion_spans))
            .alignment(ratatui::layout::Alignment::Center),
        suggestions_area,
    );
}

/// Draw the input block and set cursor position.
fn draw_input_block(f: &mut Frame, app: &mut App, input_area: Rect) {
    let input_content = if app.input.is_empty() {
        Span::styled("Ask anything... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(app.input.as_str())
    };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = input_block.inner(input_area);
    let input_paragraph = Paragraph::new(Line::from(input_content))
        .block(input_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(input_paragraph, input_area);
    let cx = inner.x + app.input.len().min(inner.width as usize) as u16;
    let cy = input_area.y + 1;
    f.set_cursor_position(Position::new(cx, cy));
}

/// Draw suggestions row.
fn draw_suggestions(f: &mut Frame, app: &mut App, area: Rect) {
    let suggestion_spans: Vec<Span> = SUGGESTIONS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == app.selected_suggestion;
            Span::styled(
                format!(" {} ", s),
                if selected {
                    Style::default().fg(Color::Black).bg(ACCENT)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            )
        })
        .collect();
    f.render_widget(
        Paragraph::new(Line::from(suggestion_spans))
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

/// Bottom bar: path on left, shortcuts on right.
pub(crate) fn draw_bottom_bar(f: &mut Frame, _app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(100)])
        .split(area);
    let path_area = chunks[0];
    let shortcuts_area = chunks[1];

    let path_display = env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "?".to_string());
    let max_path_len = path_area.width as usize;
    let path_display = if path_display.chars().count() > max_path_len && max_path_len > 2 {
        let tail: String = path_display.chars().rev().take(max_path_len - 1).collect();
        format!("…{}", tail.chars().rev().collect::<String>())
    } else {
        path_display
    };
    let path_line = Line::from(Span::styled(
        path_display,
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(
        Paragraph::new(path_line).alignment(ratatui::layout::Alignment::Left),
        path_area,
    );

    let shortcuts = super::super::shortcuts::labels::bottom_bar();
    f.render_widget(
        Paragraph::new(shortcuts).alignment(ratatui::layout::Alignment::Right),
        shortcuts_area,
    );
}

pub(crate) fn draw_input_section(f: &mut Frame, app: &mut App, input_section: Rect) {
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(input_section);

    let input_area = input_chunks[0];
    let suggestions_area = input_chunks[1];
    let shortcuts_area = input_chunks[2];

    draw_input_block(f, app, input_area);
    draw_suggestions(f, app, suggestions_area);
    draw_bottom_bar(f, app, shortcuts_area);
}
