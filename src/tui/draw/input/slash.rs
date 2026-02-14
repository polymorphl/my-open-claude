//! Slash command autocomplete list.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::core::commands;

use super::super::super::app::App;
use super::super::super::constants::{ACCENT, ACCENT_SECONDARY};

/// Draw the slash command autocomplete list above the given area.
/// List is scrollable when there are more commands than the visible viewport.
pub(super) fn draw(f: &mut Frame, app: &App, area: Rect) {
    if !app.input.starts_with('/') {
        return;
    }
    let filter = app.input.get(1..).unwrap_or("");
    let filtered = commands::filter_commands(filter);
    if filtered.is_empty() {
        return;
    }
    let total = filtered.len();
    let visible = super::AUTOCOMPLETE_VISIBLE_LINES as usize;
    let scroll_start = app
        .selected_command_index
        .saturating_sub(visible.saturating_sub(1))
        .min(total.saturating_sub(visible).max(0));
    let scroll_end = (scroll_start + visible).min(total);

    let lines: Vec<Line> = filtered[scroll_start..scroll_end]
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let idx = scroll_start + i;
            let selected = idx == app.selected_command_index;
            let name = cmd.full_name();
            let desc = format!("  {}", cmd.description);
            if selected {
                Line::from(vec![
                    Span::styled(name, Style::default().fg(Color::Black).bg(ACCENT)),
                    Span::styled(desc, Style::default().fg(Color::Black).bg(ACCENT)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(name, Style::default().fg(ACCENT_SECONDARY)),
                    Span::styled(desc, Style::default().fg(Color::DarkGray)),
                ])
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}
