//! Bottom bar: path on left, shortcuts on right.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::env;

use super::super::super::app::App;

/// Draw the bottom bar with current path and keyboard shortcuts.
pub(crate) fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Min(80)])
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

    let shortcuts = super::super::super::shortcuts::labels::bottom_bar(app.is_streaming);
    f.render_widget(
        Paragraph::new(shortcuts).alignment(ratatui::layout::Alignment::Right),
        shortcuts_area,
    );
}
