//! Popups: confirm destructive command, model selector.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::core::models::filter_models;

use super::super::app::ModelSelectorState;
use super::super::constants::ACCENT;

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

pub(crate) fn draw_confirm_popup(f: &mut Frame, area: Rect, command: &str) {
    let popup_rect = popup_area(area, 70, 25);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" ⚠ Destructive command ");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Command: "),
            Span::styled(
                command,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("y ", Style::default().fg(ACCENT)),
            Span::raw("confirm  "),
            Span::styled("n ", Style::default().fg(Color::DarkGray)),
            Span::raw("cancel"),
        ]),
    ];
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(Clear, popup_rect);
    f.render_widget(paragraph, popup_rect);
}

pub(crate) fn draw_model_selector_popup(f: &mut Frame, area: Rect, selector: &mut ModelSelectorState) {
    let popup_rect = popup_area(area, 60, 50);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" Select model (Alt+M) ");

    let inner = block.inner(popup_rect);
    f.render_widget(Clear, popup_rect);
    f.render_widget(block, popup_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);
    let filter_area = chunks[0];
    let list_area = chunks[1];
    let hint_area = chunks[2];

    let filter_content = if selector.filter.is_empty() {
        Span::styled("Filter... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(selector.filter.as_str())
    };
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let filter_inner = filter_block.inner(filter_area);
    let filter_para = Paragraph::new(Line::from(filter_content))
        .block(filter_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(filter_para, filter_area);
    let cx = filter_inner.x
        + selector
            .filter
            .chars()
            .count()
            .min(filter_inner.width as usize) as u16;
    let cy = filter_area.y + 1;
    f.set_cursor_position(ratatui::layout::Position::new(cx, cy));

    if let Some(ref err) = selector.fetch_error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(para, list_area);
    } else if selector.models.is_empty() {
        let para = Paragraph::new(Line::from(Span::styled(
            "Loading...",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
        f.render_widget(para, list_area);
    } else {
        let filtered = filter_models(&selector.models, &selector.filter);
        let clamped_index = selector
            .selected_index
            .min(filtered.len().saturating_sub(1));
        selector.selected_index = clamped_index;

        if filtered.is_empty() {
            let msg = if selector.filter.is_empty() {
                "No models"
            } else {
                "No models match filter"
            };
            let para = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )));
            f.render_widget(para, list_area);
        } else {
            let items: Vec<ListItem> = filtered
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let style = if i == selector.selected_index {
                        Style::default().fg(Color::Black).bg(ACCENT)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!(" {} ", m.name)).style(style)
                })
                .collect();

            selector.list_state.select(Some(selector.selected_index));

            let list = List::new(items)
                .highlight_style(Style::default().fg(Color::Black).bg(ACCENT));
            f.render_stateful_widget(list, list_area, &mut selector.list_state);
        }
    }

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("select  "),
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("confirm  "),
        Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("cancel  "),
        Span::styled("type ", Style::default().fg(Color::DarkGray)),
        Span::raw("filter  "),
        Span::styled("Alt+M ", Style::default().fg(Color::DarkGray)),
        Span::raw("reopen"),
    ]));
    f.render_widget(hint, hint_area);
}
