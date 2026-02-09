//! TUI rendering: layout and widgets for the chat interface.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use std::env;

use super::app::{App, ChatMessage};
use super::constants::{ACCENT, SUGGESTIONS};
use super::text::{parse_markdown_inline, wrap_message};

pub(super) fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(6),
        ])
        .split(area);

    let header_area = chunks[0];
    let history_area = chunks[1];
    let input_section = chunks[2];

    draw_header(f, header_area);
    draw_history(f, app, history_area);
    draw_input_section(f, app, input_section);

    if let Some(ref popup) = app.confirm_popup {
        draw_confirm_popup(f, area, &popup.command);
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let logo = Line::from(vec![
        Span::styled(
            "my-open-claude ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled("·", Style::default().fg(Color::DarkGray)),
        Span::raw(" assistant"),
    ]);
    let logo_block = Paragraph::new(logo).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(logo_block, area);
}

fn draw_history(f: &mut Frame, app: &mut App, history_area: Rect) {
    let history_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(history_area);
    let text_area = history_chunks[0];
    let scrollbar_area = history_chunks[1];
    let wrap_width = text_area.width as usize;
    let content_width = wrap_width.saturating_sub(2); // indentation "  "

    let mut lines: Vec<Line<'static>> = Vec::new();
    for msg in &app.messages {
        match msg {
            ChatMessage::User(s) => {
                lines.push(Line::from(vec![
                    Span::styled("Vous ", Style::default().fg(Color::DarkGray)),
                    Span::styled("→ ", Style::default().fg(ACCENT)),
                ]));
                for chunk in wrap_message(s, content_width) {
                    if chunk.is_empty() {
                        lines.push(Line::from(Span::raw("")));
                    } else {
                        let mut spans = vec![Span::raw("  ")];
                        spans.extend(parse_markdown_inline(&chunk));
                        lines.push(Line::from(spans));
                    }
                }
            }
            ChatMessage::Assistant(s) => {
                lines.push(Line::from(vec![
                    Span::styled("Assistant ", Style::default().fg(Color::DarkGray)),
                    Span::styled("→ ", Style::default().fg(ACCENT)),
                ]));
                for chunk in wrap_message(s, content_width) {
                    if chunk.is_empty() {
                        lines.push(Line::from(Span::raw("")));
                    } else {
                        let mut spans = vec![Span::raw("  ")];
                        spans.extend(parse_markdown_inline(&chunk));
                        lines.push(Line::from(spans));
                    }
                }
            }
            ChatMessage::ToolLog(s) => {
                lines.push(Line::from(Span::styled(
                    format!("  {} ", s),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            ChatMessage::Thinking => {
                lines.push(Line::from(vec![Span::styled(
                    "  Réflexion... ",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )]));
            }
        }
    }

    let total_lines = lines.len();
    let visible = text_area.height as usize;
    let max_scroll = total_lines.saturating_sub(visible.min(1));
    app.last_max_scroll = max_scroll;
    let scroll_pos = app.scroll.min(max_scroll);
    let start = scroll_pos;
    let end = (start + visible).min(total_lines);
    let visible_lines: Vec<Line> = lines.into_iter().skip(start).take(end - start).collect();

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, text_area);

    let mut scrollbar_state = ScrollbarState::default()
        .position(scroll_pos)
        .content_length(total_lines);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol("█")
        .track_symbol(Some("│"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

fn draw_input_section(f: &mut Frame, app: &mut App, input_section: Rect) {
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
    let suggestions_line = Line::from(suggestion_spans);
    let suggestions_para =
        Paragraph::new(suggestions_line).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(suggestions_para, suggestions_area);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(36)])
        .split(shortcuts_area);
    let path_area = bottom_chunks[0];
    let shortcuts_area_right = bottom_chunks[1];

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
    let path_para = Paragraph::new(path_line).alignment(ratatui::layout::Alignment::Left);
    f.render_widget(path_para, path_area);

    let shortcuts = Line::from(vec![
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("envoyer"),
        Span::styled("  ↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("défiler"),
        Span::styled("  Ctrl+C ", Style::default().fg(Color::DarkGray)),
        Span::raw("quitter"),
    ]);
    let shortcuts_para = Paragraph::new(shortcuts).alignment(ratatui::layout::Alignment::Right);
    f.render_widget(shortcuts_para, shortcuts_area_right);
}

/// Centered popup area (percentage of parent).
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

fn draw_confirm_popup(f: &mut Frame, area: Rect, command: &str) {
    let popup_rect = popup_area(area, 70, 25);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" ⚠ Commande destructive ");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Commande : "),
            Span::styled(command, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("y ", Style::default().fg(ACCENT)),
            Span::raw("confirmer  "),
            Span::styled("n ", Style::default().fg(Color::DarkGray)),
            Span::raw("annuler"),
        ]),
    ];
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(Clear, popup_rect);
    f.render_widget(paragraph, popup_rect);
}
