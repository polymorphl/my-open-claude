//! Draw delete command popup.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::core::templates::CustomTemplate;

use super::super::app::DeleteCommandState;
use super::super::constants::ACCENT;

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

pub(crate) fn draw_delete_command_popup(
    f: &mut Frame,
    area: Rect,
    state: &DeleteCommandState,
    custom_templates: &[CustomTemplate],
) {
    let popup_rect = popup_area(area, 50, 45);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" Delete custom commands ");
    let inner = block.inner(popup_rect);
    f.render_widget(Clear, popup_rect);
    f.render_widget(block, popup_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let lines: Vec<Line> = custom_templates
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let selected = state.selected.get(i).copied().unwrap_or(false);
            let cursor = i == state.selected_index;
            let checkbox = if selected { "[x]" } else { "[ ]" };
            let name = format!("/{}", t.name);
            let style = if cursor {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else if selected {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(
                format!("{} {} - {}", checkbox, name, t.description),
                style,
            ))
        })
        .collect();

    f.render_widget(Paragraph::new(lines), chunks[0]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Space ", Style::default().fg(Color::DarkGray)),
        Span::raw("toggle  "),
        Span::styled("↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("navigate  "),
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("delete  "),
        Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("cancel"),
    ]));
    f.render_widget(hint, chunks[1]);
}
