//! History selector popup (Alt+H).

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::core::history::{ConversationMeta, filter_conversations};

use super::super::app::HistorySelectorState;
use super::super::constants::ACCENT;

fn format_conversation(meta: &ConversationMeta) -> String {
    use chrono::TimeZone;
    let dt = chrono::Utc.timestamp_opt(meta.updated_at as i64, 0);
    let date_str = dt
        .single()
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| meta.updated_at.to_string());
    format!("{} — {}", meta.title, date_str)
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

pub(crate) fn draw_history_selector_popup(
    f: &mut Frame,
    area: Rect,
    selector: &mut HistorySelectorState,
) {
    let popup_rect = popup_area(area, 60, 50);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" Load conversation (Alt+H) ");

    let inner = block.inner(popup_rect);
    f.render_widget(Clear, popup_rect);
    f.render_widget(block, popup_rect);

    let is_renaming = selector.renaming.is_some();
    let constraints: &[Constraint] = if is_renaming {
        &[
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ]
    } else {
        &[
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);
    let filter_area = chunks[0];
    let (list_area, hint_area) = if is_renaming {
        let rename_area = chunks[1];
        let rename_content = selector
            .renaming
            .as_ref()
            .map(|(_, input)| {
                Line::from(vec![
                    Span::styled("Rename to: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(input.as_str()),
                    Span::styled("_", Style::default().fg(Color::DarkGray)),
                ])
            })
            .unwrap_or_else(|| Line::from(""));
        let rename_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT));
        let rename_para = Paragraph::new(rename_content)
            .block(rename_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(rename_para, rename_area);
        (chunks[2], chunks[3])
    } else {
        (chunks[1], chunks[2])
    };

    let filter_content = if selector.filter.is_empty() {
        Span::styled("Filter... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(selector.filter.as_str())
    };
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let filter_para = Paragraph::new(Line::from(filter_content))
        .block(filter_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(filter_para, filter_area);

    let filtered = filter_conversations(&selector.conversations, &selector.filter);
    let clamped_index = selector
        .selected_index
        .min(filtered.len().saturating_sub(1));
    selector.selected_index = clamped_index;

    if let Some(ref err) = selector.error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!("Error loading history: {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(para, list_area);
    } else if filtered.is_empty() {
        let msg = if selector.filter.is_empty() {
            "No conversations yet"
        } else {
            "No conversations match filter"
        };
        let para = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(para, list_area);
    } else {
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, meta)| {
                let style = if i == selector.selected_index {
                    Style::default().fg(Color::Black).bg(ACCENT)
                } else {
                    Style::default()
                };
                ListItem::new(format!(" {} ", format_conversation(meta))).style(style)
            })
            .collect();

        selector.list_state.select(Some(selector.selected_index));

        let list = List::new(items).highlight_style(Style::default().fg(Color::Black).bg(ACCENT));
        f.render_stateful_widget(list, list_area, &mut selector.list_state);
    }

    let hint = if is_renaming {
        Paragraph::new(Line::from(vec![
            Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
            Span::raw("confirm  "),
            Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
            Span::raw("cancel "),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("↑↓ ", Style::default().fg(Color::DarkGray)),
            Span::raw("select  "),
            Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
            Span::raw("load  "),
            Span::styled("Ctrl+R ", Style::default().fg(Color::DarkGray)),
            Span::raw("rename  "),
            Span::styled("Delete/Ctrl+D ", Style::default().fg(Color::DarkGray)),
            Span::raw("delete  "),
            Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
            Span::raw("cancel  "),
            Span::styled("Alt+N ", Style::default().fg(Color::DarkGray)),
            Span::raw("new "),
        ]))
    };
    f.render_widget(hint, hint_area);
}
